pub mod inconsistency;
pub mod sameas;

use std::collections::BTreeMap;

use crate::compile::CompiledSchema;
use crate::dict::TermId;
use crate::error::Result;
use crate::store::FactStore;
use crate::store::delta::difference_streaming_into;
use crate::store::merge::{MergeBinaryIter, MergeTernaryIter};
use crate::store::relation::{BinaryRelation, TernaryRelation};

pub use sameas::UnionFind;

#[derive(Clone, Debug, Default)]
pub struct ReasoningStats {
    pub iterations: usize,
    pub inferred_types: usize,
    pub inferred_properties: usize,
    pub fixpoint_reached: bool,
    pub equality_merges: usize,
    pub equality_iterations: usize,
}

impl ReasoningStats {
    pub fn total_inferred(&self) -> usize {
        self.inferred_types + self.inferred_properties
    }
}

pub struct MaterializeResult {
    pub stats: ReasoningStats,
    pub union_find: UnionFind,
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

// ─── Equality evaluation ───────────────────────────────────────────────────

use std::collections::HashMap;

/// Scan known facts for equality-producing rule firings.
///
/// Handles:
///   - Asserted owl:sameAs triples
///   - FunctionalProperty: multiple objects for same (pred, subject) → union
///   - InverseFunctionalProperty: multiple subjects for same (pred, object) → union
///   - MaxCardinality 1: type(x,A) ∧ property(x,P,y1) ∧ property(x,P,y2) [∧ type checks] → union
fn evaluate_equality_rules(
    store: &mut FactStore,
    schema: &CompiledSchema,
    union_find: &mut UnionFind,
    owl_same_as: TermId,
) -> Result<usize> {
    let mut new_equalities = 0usize;

    let has_fp = !schema.functional_properties.is_empty();
    let has_ifp = !schema.inverse_functional_properties.is_empty();
    let has_mc1 = !schema.max_card_one.is_empty();

    // Fast path: no schema equality axioms — only scan for asserted sameAs.
    if !has_fp && !has_ifp && !has_mc1 {
        for result in store.known_properties_iter()? {
            let (s, p, o) = result?;
            if p == owl_same_as && union_find.union(s, o) {
                new_equalities += 1;
            }
        }
        return Ok(new_equalities);
    }

    // Build instance → classes map for max_card_one (need to know type membership).
    let mut instance_classes: BTreeMap<TermId, Vec<TermId>> = BTreeMap::new();
    if has_mc1 {
        for result in store.known_types_iter()? {
            let (inst, cls) = result?;
            let cinst = union_find.canonical(inst);
            instance_classes.entry(cinst).or_default().push(cls);
        }
        for classes in instance_classes.values_mut() {
            classes.sort_unstable();
            classes.dedup();
        }
    }

    // Single scan of all known properties: group by (pred, canon_subj) for FP,
    // by (pred, canon_obj) for IFP, and by (canon_subj, pred) for MC1.
    let mut fp_groups: BTreeMap<(TermId, TermId), Vec<TermId>> = BTreeMap::new();
    let mut ifp_groups: BTreeMap<(TermId, TermId), Vec<TermId>> = BTreeMap::new();
    let mut mc1_groups: BTreeMap<(TermId, TermId), Vec<TermId>> = BTreeMap::new();

    for result in store.known_properties_iter()? {
        let (s, p, o) = result?;

        // Asserted or inferred owl:sameAs
        if p == owl_same_as {
            if union_find.union(s, o) {
                new_equalities += 1;
            }
            continue;
        }

        let cs = union_find.canonical(s);
        let co = union_find.canonical(o);

        if has_fp && schema.functional_properties.contains(&p) {
            fp_groups.entry((p, cs)).or_default().push(co);
        }
        if has_ifp && schema.inverse_functional_properties.contains(&p) {
            ifp_groups.entry((p, co)).or_default().push(cs);
        }
        if has_mc1 && schema.max_card_one_by_prop.contains_key(&p) {
            mc1_groups.entry((cs, p)).or_default().push(co);
        }
    }

    // FunctionalProperty: union multiple objects for same (pred, subject)
    for objects in fp_groups.values() {
        new_equalities += union_find.union_all(objects);
    }

    // InverseFunctionalProperty: union multiple subjects for same (pred, object)
    for subjects in ifp_groups.values() {
        new_equalities += union_find.union_all(subjects);
    }

    // MaxCardinality 1: for each (subject, pred) group, check matching axioms via index
    for (&(subj, pred), objects) in &mc1_groups {
        if objects.len() <= 1 {
            continue;
        }
        if let Some(axioms) = schema.max_card_one_by_prop.get(&pred) {
            for &(class, opt_filler) in axioms {
                // Check if subject is of the required class
                let has_class = instance_classes
                    .get(&subj)
                    .is_some_and(|classes| classes.binary_search(&class).is_ok());
                if !has_class {
                    continue;
                }
                // Filter objects by filler type if qualified
                let qualifying: Vec<TermId> = if let Some(filler) = opt_filler {
                    objects
                        .iter()
                        .copied()
                        .filter(|&o| {
                            let co = union_find.canonical(o);
                            instance_classes
                                .get(&co)
                                .is_some_and(|classes| classes.binary_search(&filler).is_ok())
                        })
                        .collect()
                } else {
                    objects.clone()
                };
                new_equalities += union_find.union_all(&qualifying);
            }
        }
    }

    Ok(new_equalities)
}

/// Generate candidate facts by expanding existing facts across all members
/// of each equivalence class (eq-rep-s, eq-rep-o rules).
///
/// For type(x, C) where x has equivalents {x, y, z}: emit type(y, C) and type(z, C).
/// For property(s, P, o) where s or o have equivalents: emit all combinations.
fn generate_equality_candidates(
    store: &mut FactStore,
    union_find: &mut UnionFind,
    candidate_types: &mut BinaryRelation,
    candidate_properties: &mut TernaryRelation,
) -> Result<()> {
    let eq_classes = union_find.equivalence_classes();
    if eq_classes.is_empty() {
        return Ok(());
    }

    // Build lookup: term → all members of its equivalence class.
    let mut members_of: HashMap<TermId, &Vec<TermId>> = HashMap::new();
    for members in eq_classes.values() {
        for &m in members {
            members_of.insert(m, members);
        }
    }

    // Expand type facts: type(inst, cls) → type(m, cls) for all m ∈ equiv(inst)
    for result in store.known_types_iter()? {
        let (inst, cls) = result?;
        if let Some(members) = members_of.get(&inst) {
            for &m in *members {
                if m != inst {
                    candidate_types.push((m, cls))?;
                }
            }
        }
    }

    // Expand property facts across subject and object equivalences.
    for result in store.known_properties_iter()? {
        let (s, p, o) = result?;
        let s_mems = members_of.get(&s).map(|v| v.as_slice());
        let o_mems = members_of.get(&o).map(|v| v.as_slice());

        match (s_mems, o_mems) {
            (Some(ss), Some(os)) => {
                // Both positions have equivalents — cross product.
                for &sm in ss {
                    for &om in os {
                        if sm != s || om != o {
                            candidate_properties.push((sm, p, om))?;
                        }
                    }
                }
            }
            (Some(ss), None) => {
                for &sm in ss {
                    if sm != s {
                        candidate_properties.push((sm, p, o))?;
                    }
                }
            }
            (None, Some(os)) => {
                for &om in os {
                    if om != o {
                        candidate_properties.push((s, p, om))?;
                    }
                }
            }
            (None, None) => {}
        }
    }

    Ok(())
}

/// Emit owl:sameAs triples for all non-trivial equivalence classes.
/// Emits the full pairwise closure: for each class {a, b, c, ...},
/// emit sameAs(x, y) for all distinct pairs.
fn emit_sameas_triples(
    store: &mut FactStore,
    union_find: &mut UnionFind,
    owl_same_as: TermId,
) -> Result<usize> {
    let classes = union_find.equivalence_classes();
    let mut count = 0usize;
    for members in classes.values() {
        for (i, &x) in members.iter().enumerate() {
            for &y in &members[i + 1..] {
                store
                    .derived_properties_mut()
                    .push((x, owl_same_as, y))?;
                store
                    .derived_properties_mut()
                    .push((y, owl_same_as, x))?;
                count += 2;
            }
        }
    }
    Ok(count)
}

// ─── Fixpoint loop ──────────────────────────────────────────────────────────

pub fn materialize(
    store: &mut FactStore,
    schema: &CompiledSchema,
    max_iterations: Option<usize>,
    engine_budget: usize,
    owl_same_as: TermId,
) -> Result<MaterializeResult> {
    let mut stats = ReasoningStats::default();
    let relation_budget = engine_budget / 4;
    let work_dir = store.work_dir().to_path_buf();

    let mut candidate_types = BinaryRelation::new(&work_dir, "engine-cand-types", relation_budget);
    let mut candidate_properties =
        TernaryRelation::new(&work_dir, "engine-cand-props", relation_budget);
    let mut union_find = UnionFind::new();

    // Seed from SameIndividual axioms (once, before the fixpoint loop).
    {
        let mut seeded = 0usize;
        for &(a, b) in &schema.same_individual_pairs {
            if union_find.union(a, b) {
                seeded += 1;
            }
        }
        if seeded > 0 {
            stats.equality_merges += seeded;
            generate_equality_candidates(
                store,
                &mut union_find,
                &mut candidate_types,
                &mut candidate_properties,
            )?;
        }
    }

    // Outer equality fixpoint: run inner fixpoint, check for new equalities
    // (from FunctionalProperty, InverseFunctionalProperty, MaxCardinality 1,
    // or asserted owl:sameAs), expand facts across equivalence classes, repeat.
    loop {
        inner_fixpoint(
            store,
            schema,
            max_iterations,
            &mut stats,
            &mut candidate_types,
            &mut candidate_properties,
            relation_budget,
        )?;

        let new_equalities =
            evaluate_equality_rules(store, schema, &mut union_find, owl_same_as)?;
        if new_equalities == 0 {
            break;
        }

        stats.equality_merges += new_equalities;
        stats.equality_iterations += 1;
        tracing::debug!(
            merges = new_equalities,
            total = stats.equality_merges,
            "Equality iteration {}",
            stats.equality_iterations
        );

        generate_equality_candidates(
            store,
            &mut union_find,
            &mut candidate_types,
            &mut candidate_properties,
        )?;
    }

    if union_find.has_merges() {
        let sameas_count = emit_sameas_triples(store, &mut union_find, owl_same_as)?;
        stats.inferred_properties += sameas_count;
    }

    stats.fixpoint_reached = true;
    Ok(MaterializeResult { stats, union_find })
}

/// Run the inner (non-equality) fixpoint to completion.
///
/// Seeds from asserted facts and processes any pre-populated candidates
/// (e.g. from canonical rewrites after equality discovery).
fn inner_fixpoint(
    store: &mut FactStore,
    schema: &CompiledSchema,
    max_iterations: Option<usize>,
    stats: &mut ReasoningStats,
    candidate_types: &mut BinaryRelation,
    candidate_properties: &mut TernaryRelation,
    relation_budget: usize,
) -> Result<()> {
    let work_dir = store.work_dir().to_path_buf();
    let mut delta_types = BinaryRelation::new(&work_dir, "engine-delta-types", relation_budget);
    let mut delta_properties =
        TernaryRelation::new(&work_dir, "engine-delta-props", relation_budget);

    let needs_property_index = !schema.indexed_predicates.is_empty();
    let needs_type_index = !schema.indexed_classes.is_empty();
    // cls-hv2 needs the seed join pass but doesn't contribute to indexed sets
    let needs_seed_join_pass =
        needs_property_index || needs_type_index || !schema.has_value_by_class.is_empty();

    for result in store.asserted_types_iter()? {
        let (instance, class) = result?;
        apply_type_rules(instance, class, schema, candidate_types)?;
    }
    for result in store.asserted_properties_iter()? {
        let (subject, predicate, object) = result?;
        apply_property_rules(
            subject,
            predicate,
            object,
            schema,
            candidate_types,
            candidate_properties,
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
                candidate_types,
                candidate_properties,
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
                candidate_types,
                candidate_properties,
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
            apply_type_rules(instance, class, schema, candidate_types)?;
            apply_type_join_rules(
                instance,
                class,
                schema,
                &known_indexes,
                candidate_types,
                candidate_properties,
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
                candidate_types,
                candidate_properties,
            )?;
            // delta ⋈ known joins
            apply_property_join_rules(
                subject,
                predicate,
                object,
                schema,
                &known_indexes,
                candidate_types,
                candidate_properties,
            )?;
            // delta ⋈ delta joins (property index only — type deltas already consumed above)
            apply_property_join_rules(
                subject,
                predicate,
                object,
                schema,
                &delta_indexes,
                candidate_types,
                candidate_properties,
            )?;
        }
    }

    Ok(())
}
