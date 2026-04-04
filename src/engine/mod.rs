use rayon::slice::ParallelSliceMut;

use crate::compile::CompiledSchema;
use crate::error::Result;
use crate::store::FactStore;
use crate::store::delta::difference_streaming;

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
    candidate_types: &mut Vec<(u64, u64)>,
) {
    for &superclass in schema.superclasses_for(class) {
        candidate_types.push((instance, superclass));
    }
}

fn apply_property_rules(
    subject: u64,
    predicate: u64,
    object: u64,
    schema: &CompiledSchema,
    candidate_types: &mut Vec<(u64, u64)>,
    candidate_properties: &mut Vec<(u64, u64, u64)>,
) {
    for &superproperty in schema.superproperties_for(predicate) {
        candidate_properties.push((subject, superproperty, object));
    }
    for &domain in schema.domains_for(predicate) {
        candidate_types.push((subject, domain));
    }
    for &range in schema.ranges_for(predicate) {
        candidate_types.push((object, range));
    }
}

pub fn materialize(
    store: &mut FactStore,
    schema: &CompiledSchema,
    max_iterations: Option<usize>,
) -> Result<ReasoningStats> {
    let mut stats = ReasoningStats::default();
    let mut candidate_types: Vec<(u64, u64)> = Vec::new();
    let mut candidate_properties: Vec<(u64, u64, u64)> = Vec::new();

    // Seed: stream asserted facts to generate initial candidates
    for result in store.asserted_types_iter()? {
        let (instance, class) = result?;
        apply_type_rules(instance, class, schema, &mut candidate_types);
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
        );
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

        // Sort, dedup candidates
        candidate_types.par_sort_unstable();
        candidate_types.dedup();
        candidate_properties.par_sort_unstable();
        candidate_properties.dedup();

        // Difference check against known view (streaming)
        let new_types = difference_streaming(&candidate_types, store.known_types_iter()?)?;
        let new_properties =
            difference_streaming(&candidate_properties, store.known_properties_iter()?)?;

        if new_types.is_empty() && new_properties.is_empty() {
            break;
        }

        // Merge new facts into derived segments
        for &(instance, class) in &new_types {
            store.derived_types_mut().push((instance, class))?;
        }
        for &(subject, predicate, object) in &new_properties {
            store
                .derived_properties_mut()
                .push((subject, predicate, object))?;
        }

        stats.inferred_types += new_types.len();
        stats.inferred_properties += new_properties.len();

        // Generate next iteration's candidates from new facts (delta)
        candidate_types.clear();
        candidate_properties.clear();
        for &(instance, class) in &new_types {
            apply_type_rules(instance, class, schema, &mut candidate_types);
        }
        for &(subject, predicate, object) in &new_properties {
            apply_property_rules(
                subject,
                predicate,
                object,
                schema,
                &mut candidate_types,
                &mut candidate_properties,
            );
        }
    }

    stats.fixpoint_reached = true;
    Ok(stats)
}
