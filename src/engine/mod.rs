use std::collections::BTreeMap;

use crate::compile::CompiledSchema;
use crate::dict::TermId;
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

// ─── In-memory index for join-requiring rules ───────────────────────────────

/// In-memory index of property assertions for join-requiring rules.
///
/// Only indexes predicates listed in `CompiledSchema::indexed_predicates`
/// (currently transitive properties; Steps 4-5 will add more).
struct PropertyIndex {
    /// (predicate, subject) → sorted vec of objects
    by_pred_subj: BTreeMap<(TermId, TermId), Vec<TermId>>,
    /// (predicate, object) → sorted vec of subjects
    by_pred_obj: BTreeMap<(TermId, TermId), Vec<TermId>>,
}

impl PropertyIndex {
    fn new() -> Self {
        Self {
            by_pred_subj: BTreeMap::new(),
            by_pred_obj: BTreeMap::new(),
        }
    }

    fn insert(&mut self, subject: TermId, predicate: TermId, object: TermId) {
        self.by_pred_subj
            .entry((predicate, subject))
            .or_default()
            .push(object);
        self.by_pred_obj
            .entry((predicate, object))
            .or_default()
            .push(subject);
    }

    fn dedup(&mut self) {
        for values in self.by_pred_subj.values_mut() {
            values.sort_unstable();
            values.dedup();
        }
        for values in self.by_pred_obj.values_mut() {
            values.sort_unstable();
            values.dedup();
        }
    }

    /// Objects z such that property(subject, predicate, z) is known.
    fn objects_for(&self, predicate: TermId, subject: TermId) -> &[TermId] {
        self.by_pred_subj
            .get(&(predicate, subject))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Subjects w such that property(w, predicate, object) is known.
    fn subjects_for(&self, predicate: TermId, object: TermId) -> &[TermId] {
        self.by_pred_obj
            .get(&(predicate, object))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

/// Build a property index from current known properties, filtered to only
/// predicates that need indexing for join-based rules.
fn build_property_index(store: &mut FactStore, schema: &CompiledSchema) -> Result<PropertyIndex> {
    if schema.indexed_predicates.is_empty() {
        return Ok(PropertyIndex::new());
    }

    let mut index = PropertyIndex::new();
    for result in store.known_properties_iter()? {
        let (subject, predicate, object) = result?;
        if schema.indexed_predicates.contains(&predicate) {
            index.insert(subject, predicate, object);
        }
    }
    index.dedup();
    Ok(index)
}

// ─── Rule application ───────────────────────────────────────────────────────

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
    for &inverse in schema.inverses_for(predicate) {
        candidate_properties.push((object, inverse, subject))?;
    }
    if schema.is_symmetric(predicate) {
        candidate_properties.push((object, predicate, subject))?;
    }
    Ok(())
}

/// Join-based rules for property deltas that require an in-memory index.
///
/// Currently handles transitive properties. When delta property(x, p, y)
/// arrives and p is transitive:
///   - Forward: for each known property(y, p, z), emit property(x, p, z)
///   - Backward: for each known property(w, p, x), emit property(w, p, y)
fn apply_property_join_rules(
    subject: TermId,
    predicate: TermId,
    object: TermId,
    schema: &CompiledSchema,
    property_index: &PropertyIndex,
    candidate_properties: &mut TernaryRelation,
) -> Result<()> {
    if schema.is_transitive(predicate) {
        // Forward: delta(x, p, y) ∧ known(y, p, z) → candidate(x, p, z)
        for &z in property_index.objects_for(predicate, object) {
            candidate_properties.push((subject, predicate, z))?;
        }
        // Backward: known(w, p, x) ∧ delta(x, p, y) → candidate(w, p, y)
        for &w in property_index.subjects_for(predicate, subject) {
            candidate_properties.push((w, predicate, object))?;
        }
    }
    Ok(())
}

// ─── Fixpoint loop ──────────────────────────────────────────────────────────

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

    let needs_property_index = !schema.indexed_predicates.is_empty();

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

    // Seed join-based rules (transitive properties, etc.) using an index
    // built from asserted facts.
    if needs_property_index {
        let seed_index = build_property_index(store, schema)?;
        for result in store.asserted_properties_iter()? {
            let (subject, predicate, object) = result?;
            apply_property_join_rules(
                subject,
                predicate,
                object,
                schema,
                &seed_index,
                &mut candidate_properties,
            )?;
        }
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

        // Build in-memory index of known properties for join-based rules.
        // Built BEFORE consuming deltas so it reflects the pre-delta state.
        let property_index = if needs_property_index {
            build_property_index(store, schema)?
        } else {
            PropertyIndex::new()
        };

        // Single-pass delta consumption: push into derived + generate next candidates
        for result in MergeBinaryIter::new(delta_types.segment_iters()?)? {
            let (instance, class) = result?;
            store.derived_types_mut().push((instance, class))?;
            apply_type_rules(instance, class, schema, &mut candidate_types)?;
        }

        // Build a delta-only index for delta⋈delta joins (transitive, etc.)
        let delta_property_index = if needs_property_index {
            let mut idx = PropertyIndex::new();
            for result in MergeTernaryIter::new(delta_properties.segment_iters()?)? {
                let (s, p, o) = result?;
                if schema.indexed_predicates.contains(&p) {
                    idx.insert(s, p, o);
                }
            }
            idx.dedup();
            idx
        } else {
            PropertyIndex::new()
        };

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
            // delta ⋈ known joins
            apply_property_join_rules(
                subject,
                predicate,
                object,
                schema,
                &property_index,
                &mut candidate_properties,
            )?;
            // delta ⋈ delta joins
            apply_property_join_rules(
                subject,
                predicate,
                object,
                schema,
                &delta_property_index,
                &mut candidate_properties,
            )?;
        }
    }

    stats.fixpoint_reached = true;
    Ok(stats)
}
