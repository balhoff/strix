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
/// (transitive, someValuesFrom, allValuesFrom properties).
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

// ─── Type index for join-requiring rules ─────────────────────────────────────

/// In-memory index of type assertions for join-requiring rules.
///
/// Only indexes classes listed in `CompiledSchema::indexed_classes`
/// (someValuesFrom fillers, allValuesFrom classes, intersection conjuncts).
struct TypeIndex {
    /// instance → sorted vec of classes
    by_instance: BTreeMap<TermId, Vec<TermId>>,
}

impl TypeIndex {
    fn new() -> Self {
        Self {
            by_instance: BTreeMap::new(),
        }
    }

    fn insert(&mut self, instance: TermId, class: TermId) {
        self.by_instance
            .entry(instance)
            .or_default()
            .push(class);
    }

    fn dedup(&mut self) {
        for values in self.by_instance.values_mut() {
            values.sort_unstable();
            values.dedup();
        }
    }

    /// Classes that instance belongs to (filtered to indexed classes).
    fn classes_of(&self, instance: TermId) -> &[TermId] {
        self.by_instance
            .get(&instance)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn has_type(&self, instance: TermId, class: TermId) -> bool {
        self.classes_of(instance).binary_search(&class).is_ok()
    }
}

/// Build a type index from current known types, filtered to indexed classes.
fn build_type_index(store: &mut FactStore, schema: &CompiledSchema) -> Result<TypeIndex> {
    if schema.indexed_classes.is_empty() {
        return Ok(TypeIndex::new());
    }

    let mut index = TypeIndex::new();
    for result in store.known_types_iter()? {
        let (instance, class) = result?;
        if schema.indexed_classes.contains(&class) {
            index.insert(instance, class);
        }
    }
    index.dedup();
    Ok(index)
}

/// Bundled indexes for join-based rule evaluation.
struct JoinIndexes<'a> {
    types: &'a TypeIndex,
    properties: &'a PropertyIndex,
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
    for &ut in &schema.universal_types {
        candidate_types.push((instance, ut))?;
    }
    Ok(())
}

/// Join-based rules for type deltas that require in-memory indexes.
///
/// Handles:
///   - intersectionOf (cls-int2): type(x,C) where C is an intersection → emit type(x,Ci) for each conjunct
///   - intersectionOf (cls-int1): type(x,D) where D is a conjunct → check all other conjuncts in type_index
///   - allValuesFrom: type(x,A) where allValuesFrom(A,P,B) → for each property(x,P,y), emit type(y,B)
///   - hasValue (cls-hv2): type(x,A) where hasValue_sub(A,P,v) → emit property(x,P,v)
///   - someValuesFrom (type-triggered): type(y,D) where someValuesFrom(C,P,D) → for each property(z,P,y), emit type(z,C)
fn apply_type_join_rules(
    instance: TermId,
    class: TermId,
    schema: &CompiledSchema,
    indexes: &JoinIndexes<'_>,
    candidate_types: &mut BinaryRelation,
    candidate_properties: &mut TernaryRelation,
) -> Result<()> {
    // cls-int2: type(x,C) where C is intersection → emit conjuncts
    if let Some(conjuncts) = schema.intersection_conjuncts.get(&class) {
        for &conjunct in conjuncts {
            candidate_types.push((instance, conjunct))?;
        }
    }

    // cls-int1: type(x,D) where D is a conjunct → check all other conjuncts
    if let Some(intersection_classes) = schema.conjunct_of.get(&class) {
        for &int_class in intersection_classes {
            if let Some(conjuncts) = schema.intersection_conjuncts.get(&int_class) {
                let all_present = conjuncts
                    .iter()
                    .all(|&c| c == class || indexes.types.has_type(instance, c));
                if all_present {
                    candidate_types.push((instance, int_class))?;
                }
            }
        }
    }

    // cls-avf: type(x,A) where allValuesFrom(A,P,B) → for each property(x,P,y), emit type(y,B)
    if let Some(entries) = schema.all_values_from_by_class.get(&class) {
        for &(prop, filler) in entries {
            for &y in indexes.properties.objects_for(prop, instance) {
                candidate_types.push((y, filler))?;
            }
        }
    }

    // cls-hv2: type(x,A) where hasValue_sub(A,P,v) → emit property(x,P,v)
    if let Some(entries) = schema.has_value_by_class.get(&class) {
        for &(prop, val) in entries {
            candidate_properties.push((instance, prop, val))?;
        }
    }

    // cls-svf1 (type-triggered): type(y,D) where someValuesFrom(C,P,D) →
    //   for each property(z,P,y) in prop_index, emit type(z,C)
    if let Some(entries) = schema.some_values_from_by_filler.get(&class) {
        for &(svf_class, prop) in entries {
            for &z in indexes.properties.subjects_for(prop, instance) {
                candidate_types.push((z, svf_class))?;
            }
        }
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
    // cls-hv1: property(x,P,v) where hasValue_super(C,P,v) → type(x,C)
    if let Some(entries) = schema.has_value_by_prop.get(&predicate) {
        for &(class, val) in entries {
            if object == val {
                candidate_types.push((subject, class))?;
            }
        }
    }
    // svf-thing: property(x,P,_) → type(x,C) — no filler check needed
    if let Some(classes) = schema.svf_thing_by_prop.get(&predicate) {
        for &class in classes {
            candidate_types.push((subject, class))?;
        }
    }
    for &ut in &schema.universal_types {
        candidate_types.push((subject, ut))?;
        candidate_types.push((object, ut))?;
    }
    Ok(())
}

/// Join-based rules for property deltas that require in-memory indexes.
///
/// Handles:
///   - Transitive property chaining via PropertyIndex
///   - cls-svf1 (property-triggered): property(x,P,y) ∧ type(y,D) → type(x,C)
///   - cls-avf (property-triggered): property(x,P,y) ∧ type(x,A) → type(y,B)
fn apply_property_join_rules(
    subject: TermId,
    predicate: TermId,
    object: TermId,
    schema: &CompiledSchema,
    indexes: &JoinIndexes<'_>,
    candidate_types: &mut BinaryRelation,
    candidate_properties: &mut TernaryRelation,
) -> Result<()> {
    if schema.is_transitive(predicate) {
        for &z in indexes.properties.objects_for(predicate, object) {
            candidate_properties.push((subject, predicate, z))?;
        }
        for &w in indexes.properties.subjects_for(predicate, subject) {
            candidate_properties.push((w, predicate, object))?;
        }
    }

    // cls-svf1 (property-triggered): check type_index for type(y,D) → emit type(x,C)
    if let Some(entries) = schema.some_values_from_by_prop.get(&predicate) {
        for &(class, filler) in entries {
            if indexes.types.has_type(object, filler) {
                candidate_types.push((subject, class))?;
            }
        }
    }

    // cls-avf (property-triggered): check type_index for type(x,A) → emit type(y,B)
    if let Some(entries) = schema.all_values_from_by_prop.get(&predicate) {
        for &(class, filler) in entries {
            if indexes.types.has_type(subject, class) {
                candidate_types.push((object, filler))?;
            }
        }
    }

    // Property chains: delta(x, p_i, y) → walk backward/forward to complete chain
    if let Some(triggers) = schema.chain_triggers.get(&predicate) {
        for &(chain_idx, pos) in triggers {
            let (super_prop, chain) = &schema.property_chains[chain_idx];

            // Walk backward through positions 0..pos to collect chain start nodes
            let mut starts = vec![subject];
            for &pred in chain[..pos].iter().rev() {
                let mut next = Vec::new();
                for &node in &starts {
                    next.extend_from_slice(indexes.properties.subjects_for(pred, node));
                }
                starts = next;
                if starts.is_empty() {
                    break;
                }
            }
            if starts.is_empty() {
                continue;
            }

            // Walk forward through positions pos+1..n to collect chain end nodes
            let mut ends = vec![object];
            for &pred in &chain[pos + 1..] {
                let mut next = Vec::new();
                for &node in &ends {
                    next.extend_from_slice(indexes.properties.objects_for(pred, node));
                }
                ends = next;
                if ends.is_empty() {
                    break;
                }
            }

            for &s in &starts {
                for &e in &ends {
                    candidate_properties.push((s, *super_prop, e))?;
                }
            }
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
    let needs_type_index = !schema.indexed_classes.is_empty();
    // cls-hv2 needs the seed join pass but doesn't contribute to indexed sets
    let needs_seed_join_pass =
        needs_property_index || needs_type_index || !schema.has_value_by_class.is_empty();

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

    if needs_seed_join_pass {
        let seed_type_index = build_type_index(store, schema)?;
        let seed_prop_index = build_property_index(store, schema)?;
        let seed_indexes = JoinIndexes {
            types: &seed_type_index,
            properties: &seed_prop_index,
        };

        for result in store.asserted_types_iter()? {
            let (instance, class) = result?;
            apply_type_join_rules(
                instance,
                class,
                schema,
                &seed_indexes,
                &mut candidate_types,
                &mut candidate_properties,
            )?;
        }
        for result in store.asserted_properties_iter()? {
            let (subject, predicate, object) = result?;
            apply_property_join_rules(
                subject,
                predicate,
                object,
                schema,
                &seed_indexes,
                &mut candidate_types,
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

        candidate_types.compact()?;
        candidate_properties.compact()?;

        delta_types.clear();
        delta_properties.clear();

        // Streaming difference: candidates vs known → deltas
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

        candidate_types.clear();
        candidate_properties.clear();

        delta_types.compact()?;
        delta_properties.compact()?;

        // Build in-memory indexes of known facts for join-based rules.
        // Built BEFORE consuming deltas so they reflect the pre-delta state.
        let type_index = if needs_type_index {
            build_type_index(store, schema)?
        } else {
            TypeIndex::new()
        };
        let known_prop_index = if needs_property_index {
            build_property_index(store, schema)?
        } else {
            PropertyIndex::new()
        };
        let known_indexes = JoinIndexes {
            types: &type_index,
            properties: &known_prop_index,
        };

        for result in MergeBinaryIter::new(delta_types.segment_iters()?)? {
            let (instance, class) = result?;
            store.derived_types_mut().push((instance, class))?;
            apply_type_rules(instance, class, schema, &mut candidate_types)?;
            apply_type_join_rules(
                instance,
                class,
                schema,
                &known_indexes,
                &mut candidate_types,
                &mut candidate_properties,
            )?;
        }

        // Build a delta-only property index for delta⋈delta joins
        let delta_prop_index = if needs_property_index {
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
        let delta_indexes = JoinIndexes {
            types: &type_index,
            properties: &delta_prop_index,
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
                &known_indexes,
                &mut candidate_types,
                &mut candidate_properties,
            )?;
            // delta ⋈ delta joins (property index only — type deltas already consumed above)
            apply_property_join_rules(
                subject,
                predicate,
                object,
                schema,
                &delta_indexes,
                &mut candidate_types,
                &mut candidate_properties,
            )?;
        }
    }

    stats.fixpoint_reached = true;
    Ok(stats)
}
