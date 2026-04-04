use crate::compile::CompiledSchema;
use crate::error::Result;
use crate::store::FactStore;
use crate::store::delta::difference_streaming_into;
use crate::store::merge::{MergeBinaryIter, MergeTernaryIter};
use crate::store::relation::{BinaryRelation, TernaryRelation};

#[derive(Clone, Debug, Default)]
pub struct ReasoningStats {
    pub iterations: usize,
    pub inferred_types: usize,
    pub inferred_properties: usize,
    pub fixpoint_reached: bool,
}

impl ReasoningStats {
    pub fn total_inferred(&self) -> usize {
        self.inferred_types + self.inferred_properties
    }
}

fn apply_type_rules(
    instance: u64,
    class: u64,
    schema: &CompiledSchema,
    candidate_types: &mut BinaryRelation,
) -> Result<()> {
    for &superclass in schema.superclasses_for(class) {
        candidate_types.push((instance, superclass))?;
    }
    Ok(())
}

fn apply_property_rules(
    subject: u64,
    predicate: u64,
    object: u64,
    schema: &CompiledSchema,
    candidate_types: &mut BinaryRelation,
    candidate_properties: &mut TernaryRelation,
) -> Result<()> {
    for &superproperty in schema.superproperties_for(predicate) {
        candidate_properties.push((subject, superproperty, object))?;
    }
    for &domain in schema.domains_for(predicate) {
        candidate_types.push((subject, domain))?;
    }
    for &range in schema.ranges_for(predicate) {
        candidate_types.push((object, range))?;
    }
    Ok(())
}

pub fn materialize(
    store: &mut FactStore,
    schema: &CompiledSchema,
    max_iterations: Option<usize>,
    engine_budget: usize,
) -> Result<ReasoningStats> {
    let mut stats = ReasoningStats::default();
    let relation_budget = engine_budget / 4;
    let work_dir = store.work_dir();

    let mut candidate_types = BinaryRelation::new(work_dir, "engine-cand-types", relation_budget);
    let mut candidate_properties =
        TernaryRelation::new(work_dir, "engine-cand-props", relation_budget);
    let mut delta_types = BinaryRelation::new(work_dir, "engine-delta-types", relation_budget);
    let mut delta_properties =
        TernaryRelation::new(work_dir, "engine-delta-props", relation_budget);

    // Seed: stream asserted facts to generate initial candidates
    for result in store.asserted_types_iter()? {
        let (instance, class) = result?;
        apply_type_rules(instance, class, schema, &mut candidate_types)?;
    }
    for result in store.asserted_properties_iter()? {
        let (subject, predicate, object) = result?;
        apply_property_rules(
            subject,
            predicate,
            object,
            schema,
            &mut candidate_types,
            &mut candidate_properties,
        )?;
    }

    loop {
        if candidate_types.is_empty() && candidate_properties.is_empty() {
            break;
        }

        if let Some(limit) = max_iterations
            && stats.iterations >= limit
        {
            anyhow::bail!("maximum iterations ({limit}) reached before fixpoint");
        }
        stats.iterations += 1;

        // Compact candidates into single sorted, deduped segments
        candidate_types.compact()?;
        candidate_properties.compact()?;

        // Clear deltas from previous iteration
        delta_types.clear();
        delta_properties.clear();

        // Streaming difference: candidate stream vs known stream → delta relations
        let new_type_count = difference_streaming_into(
            MergeBinaryIter::new(candidate_types.segment_iters()?)?,
            store.known_types_iter()?,
            |tuple| delta_types.push(tuple),
        )?;

        let new_prop_count = difference_streaming_into(
            MergeTernaryIter::new(candidate_properties.segment_iters()?)?,
            store.known_properties_iter()?,
            |tuple| delta_properties.push(tuple),
        )?;

        if new_type_count == 0 && new_prop_count == 0 {
            break;
        }

        stats.inferred_types += new_type_count;
        stats.inferred_properties += new_prop_count;

        // Clear candidates for next round
        candidate_types.clear();
        candidate_properties.clear();

        // Compact deltas so each is a single sorted, deduped segment
        delta_types.compact()?;
        delta_properties.compact()?;

        // Single-pass delta consumption: push into derived + generate next candidates
        for result in MergeBinaryIter::new(delta_types.segment_iters()?)? {
            let (instance, class) = result?;
            store.derived_types_mut().push((instance, class))?;
            apply_type_rules(instance, class, schema, &mut candidate_types)?;
        }

        for result in MergeTernaryIter::new(delta_properties.segment_iters()?)? {
            let (subject, predicate, object) = result?;
            store
                .derived_properties_mut()
                .push((subject, predicate, object))?;
            apply_property_rules(
                subject,
                predicate,
                object,
                schema,
                &mut candidate_types,
                &mut candidate_properties,
            )?;
        }
    }

    stats.fixpoint_reached = true;
    Ok(stats)
}
