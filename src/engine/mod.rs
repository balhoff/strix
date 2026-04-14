pub mod inconsistency;
pub mod sameas;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use crate::compile::CompiledSchema;
use crate::compile::ir::{SwrlArg, SwrlBodyAtom, SwrlHeadAtom};
use crate::dict::{Dictionary, TermId};
use crate::error::Result;
use crate::rdf::Term;
use crate::store::FactStore;
use crate::store::delta::difference_streaming_into;
use crate::store::merge::{MergeBinaryIter, MergeTernaryIter};
use crate::store::relation::{BinaryRelation, TernaryRelation};

use inconsistency::Inconsistency;
pub use sameas::UnionFind;

use crate::output::report::{IterationReport, RuleFirings};

#[derive(Clone, Debug, Default)]
pub struct ReasoningStats {
    pub iterations: usize,
    pub inferred_types: usize,
    pub inferred_properties: usize,
    pub fixpoint_reached: bool,
    pub equality_merges: usize,
    pub equality_iterations: usize,
    pub iteration_details: Vec<IterationReport>,
    pub rule_firings: RuleFirings,
}

impl ReasoningStats {
    pub fn total_inferred(&self) -> usize {
        self.inferred_types + self.inferred_properties
    }
}

pub struct MaterializeResult {
    pub stats: ReasoningStats,
    pub union_find: UnionFind,
    /// DifferentIndividuals pairs inferred by SWRL DifferentIndividualsAtom heads.
    pub swrl_different_pairs: Vec<(TermId, TermId)>,
    /// Inconsistencies detected during equality reasoning (e.g. LiteralConflict).
    pub literal_conflicts: Vec<Inconsistency>,
}

// ─── In-memory index for join-requiring rules ───────────────────────────────

/// In-memory index of property assertions for join-requiring rules.
///
/// Only indexes predicates listed in `CompiledSchema::indexed_predicates`
/// (transitive, someValuesFrom, allValuesFrom properties).
fn sorted_insert(vec: &mut Vec<TermId>, value: TermId) {
    if let Err(pos) = vec.binary_search(&value) {
        vec.insert(pos, value);
    }
}

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

    fn insert_sorted(&mut self, subject: TermId, predicate: TermId, object: TermId) {
        sorted_insert(self.by_pred_subj.entry((predicate, subject)).or_default(), object);
        sorted_insert(self.by_pred_obj.entry((predicate, object)).or_default(), subject);
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

    /// All (subject, object) pairs for a given predicate.
    fn triples_for(&self, predicate: TermId) -> impl Iterator<Item = (TermId, TermId)> + '_ {
        self.by_pred_subj
            .range((predicate, 0)..=(predicate, TermId::MAX))
            .flat_map(|(&(_, subj), objects)| objects.iter().map(move |&obj| (subj, obj)))
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
            index.insert_sorted(subject, predicate, object);
        }
    }
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
    /// class → sorted vec of instances
    by_class: BTreeMap<TermId, Vec<TermId>>,
}

impl TypeIndex {
    fn new() -> Self {
        Self {
            by_instance: BTreeMap::new(),
            by_class: BTreeMap::new(),
        }
    }

    fn insert_sorted(&mut self, instance: TermId, class: TermId) {
        sorted_insert(self.by_instance.entry(instance).or_default(), class);
        sorted_insert(self.by_class.entry(class).or_default(), instance);
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

    /// Instances of a given class (filtered to indexed classes).
    fn instances_of(&self, class: TermId) -> &[TermId] {
        self.by_class.get(&class).map(Vec::as_slice).unwrap_or(&[])
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
            index.insert_sorted(instance, class);
        }
    }
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
    firings: &mut RuleFirings,
) -> Result<()> {
    let supers = schema.superclasses_for(class);
    if !supers.is_empty() {
        firings.subclass += supers.len();
        for &superclass in supers {
            candidate_types.push((instance, superclass))?;
        }
    }
    if !schema.universal_types.is_empty() {
        firings.universal_type += schema.universal_types.len();
        for &ut in &schema.universal_types {
            candidate_types.push((instance, ut))?;
        }
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
    firings: &mut RuleFirings,
) -> Result<()> {
    // cls-int2: type(x,C) where C is intersection super → emit conjuncts from all rules
    if let Some(rule_indices) = schema.intersection_by_class.get(&class) {
        for &idx in rule_indices {
            let (_, conjuncts) = &schema.intersection_rules[idx];
            firings.intersection += conjuncts.len();
            for &conjunct in conjuncts {
                candidate_types.push((instance, conjunct))?;
            }
        }
    }

    // cls-int1: type(x,D) where D is a conjunct → check all other conjuncts
    if let Some(rule_indices) = schema.conjunct_of.get(&class) {
        for &idx in rule_indices {
            let (int_class, conjuncts) = &schema.intersection_rules[idx];
            let all_present = conjuncts
                .iter()
                .all(|&c| c == class || indexes.types.has_type(instance, c));
            if all_present {
                firings.intersection += 1;
                candidate_types.push((instance, *int_class))?;
            }
        }
    }

    // cls-avf: type(x,A) where allValuesFrom(A,P,B) → for each property(x,P,y), emit type(y,B)
    if let Some(entries) = schema.all_values_from_by_class.get(&class) {
        for &(prop, filler) in entries {
            let objects = indexes.properties.objects_for(prop, instance);
            firings.all_values_from += objects.len();
            for &y in objects {
                candidate_types.push((y, filler))?;
            }
        }
    }

    // cls-hv2: type(x,A) where hasValue_sub(A,P,v) → emit property(x,P,v)
    if let Some(entries) = schema.has_value_by_class.get(&class) {
        firings.has_value += entries.len();
        for &(prop, val) in entries {
            candidate_properties.push((instance, prop, val))?;
        }
    }

    // cls-svf1 (type-triggered): type(y,D) where someValuesFrom(C,P,D) ���
    //   for each property(z,P,y) in prop_index, emit type(z,C)
    if let Some(entries) = schema.some_values_from_by_filler.get(&class) {
        for &(svf_class, prop) in entries {
            let subjects = indexes.properties.subjects_for(prop, instance);
            firings.some_values_from += subjects.len();
            for &z in subjects {
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
    firings: &mut RuleFirings,
) -> Result<()> {
    let supers = schema.superproperties_for(predicate);
    if !supers.is_empty() {
        firings.subproperty += supers.len();
        for &superproperty in supers {
            candidate_properties.push((subject, superproperty, object))?;
        }
    }
    let doms = schema.domains_for(predicate);
    if !doms.is_empty() {
        firings.domain += doms.len();
        for &domain in doms {
            candidate_types.push((subject, domain))?;
        }
    }
    let rngs = schema.ranges_for(predicate);
    if !rngs.is_empty() {
        firings.range += rngs.len();
        for &range in rngs {
            candidate_types.push((object, range))?;
        }
    }
    let invs = schema.inverses_for(predicate);
    if !invs.is_empty() {
        firings.inverse += invs.len();
        for &inverse in invs {
            candidate_properties.push((object, inverse, subject))?;
        }
    }
    if schema.is_symmetric(predicate) {
        firings.symmetric += 1;
        candidate_properties.push((object, predicate, subject))?;
    }
    // cls-hv1: property(x,P,v) where hasValue_super(C,P,v) → type(x,C)
    if let Some(entries) = schema.has_value_by_prop.get(&predicate) {
        for &(class, val) in entries {
            if object == val {
                firings.has_value += 1;
                candidate_types.push((subject, class))?;
            }
        }
    }
    // svf-thing: property(x,P,_) → type(x,C) — no filler check needed
    if let Some(classes) = schema.svf_thing_by_prop.get(&predicate) {
        firings.some_values_from += classes.len();
        for &class in classes {
            candidate_types.push((subject, class))?;
        }
    }
    if !schema.universal_types.is_empty() {
        firings.universal_type += schema.universal_types.len() * 2;
        for &ut in &schema.universal_types {
            candidate_types.push((subject, ut))?;
            candidate_types.push((object, ut))?;
        }
    }
    Ok(())
}

/// Join-based rules for property deltas that require in-memory indexes.
///
/// Handles:
///   - Transitive property chaining via PropertyIndex
///   - cls-svf1 (property-triggered): property(x,P,y) ∧ type(y,D) → type(x,C)
///   - cls-avf (property-triggered): property(x,P,y) ∧ type(x,A) → type(y,B)

/// Reusable buffers for property chain backward/forward walks.
/// Avoids per-fact heap allocation on a hot path.
struct ChainBuffers {
    /// Accumulates backward-walk start nodes; persists while forward walk runs.
    starts: Vec<TermId>,
    /// Current frontier during a walk step.
    current: Vec<TermId>,
    /// Next frontier being built during a walk step.
    next: Vec<TermId>,
}

impl ChainBuffers {
    fn new() -> Self {
        Self {
            starts: Vec::new(),
            current: Vec::new(),
            next: Vec::new(),
        }
    }
}

fn apply_property_join_rules(
    subject: TermId,
    predicate: TermId,
    object: TermId,
    schema: &CompiledSchema,
    indexes: &JoinIndexes<'_>,
    candidate_types: &mut BinaryRelation,
    candidate_properties: &mut TernaryRelation,
    firings: &mut RuleFirings,
    chain_bufs: &mut ChainBuffers,
) -> Result<()> {
    if schema.is_transitive(predicate) {
        let fwd = indexes.properties.objects_for(predicate, object);
        let bwd = indexes.properties.subjects_for(predicate, subject);
        firings.transitive += fwd.len() + bwd.len();
        for &z in fwd {
            candidate_properties.push((subject, predicate, z))?;
        }
        for &w in bwd {
            candidate_properties.push((w, predicate, object))?;
        }
    }

    // cls-svf1 (property-triggered): check type_index for type(y,D) → emit type(x,C)
    if let Some(entries) = schema.some_values_from_by_prop.get(&predicate) {
        for &(class, filler) in entries {
            if indexes.types.has_type(object, filler) {
                firings.some_values_from += 1;
                candidate_types.push((subject, class))?;
            }
        }
    }

    // cls-avf (property-triggered): check type_index for type(x,A) → emit type(y,B)
    if let Some(entries) = schema.all_values_from_by_prop.get(&predicate) {
        for &(class, filler) in entries {
            if indexes.types.has_type(subject, class) {
                firings.all_values_from += 1;
                candidate_types.push((object, filler))?;
            }
        }
    }

    // Property chains: delta(x, p_i, y) → walk backward/forward to complete chain
    if let Some(triggers) = schema.chain_triggers.get(&predicate) {
        for &(chain_idx, pos) in triggers {
            let (super_prop, chain) = &schema.property_chains[chain_idx];

            // Walk backward through positions 0..pos to collect chain start nodes
            chain_bufs.current.clear();
            chain_bufs.current.push(subject);
            for &pred in chain[..pos].iter().rev() {
                chain_bufs.next.clear();
                for &node in &chain_bufs.current {
                    chain_bufs
                        .next
                        .extend_from_slice(indexes.properties.subjects_for(pred, node));
                }
                std::mem::swap(&mut chain_bufs.current, &mut chain_bufs.next);
                if chain_bufs.current.is_empty() {
                    break;
                }
            }
            if chain_bufs.current.is_empty() {
                continue;
            }
            chain_bufs.starts.clear();
            std::mem::swap(&mut chain_bufs.starts, &mut chain_bufs.current);

            // Walk forward through positions pos+1..n to collect chain end nodes
            chain_bufs.current.clear();
            chain_bufs.current.push(object);
            for &pred in &chain[pos + 1..] {
                chain_bufs.next.clear();
                for &node in &chain_bufs.current {
                    chain_bufs
                        .next
                        .extend_from_slice(indexes.properties.objects_for(pred, node));
                }
                std::mem::swap(&mut chain_bufs.current, &mut chain_bufs.next);
                if chain_bufs.current.is_empty() {
                    break;
                }
            }

            let count = chain_bufs.starts.len() * chain_bufs.current.len();
            firings.property_chain += count;
            for &s in &chain_bufs.starts {
                for &e in &chain_bufs.current {
                    candidate_properties.push((s, *super_prop, e))?;
                }
            }
        }
    }

    Ok(())
}

// ─── SWRL rule evaluation ──────────────────────────────────────────────────

/// Context for SWRL rule evaluation within a single fixpoint iteration.
struct SwrlContext<'a> {
    indexes: &'a JoinIndexes<'a>,
    union_find: &'a UnionFind,
    different_pairs: &'a RefCell<BTreeSet<(TermId, TermId)>>,
    equality_class_index: BTreeMap<TermId, Vec<TermId>>,
    owl_same_as: TermId,
}

/// Mutable output sinks for SWRL rule evaluation.
struct SwrlSink<'a> {
    candidate_types: &'a mut BinaryRelation,
    candidate_properties: &'a mut TernaryRelation,
    different_pairs: &'a mut Vec<(TermId, TermId)>,
    firings: &'a mut RuleFirings,
}

/// Apply SWRL rules triggered by a type delta.
fn apply_swrl_type_rules(
    instance: TermId,
    class: TermId,
    schema: &CompiledSchema,
    ctx: &SwrlContext<'_>,
    sink: &mut SwrlSink<'_>,
) -> Result<()> {
    let rule_indices = match schema.swrl_by_type_trigger.get(&class) {
        Some(indices) => indices,
        None => return Ok(()),
    };
    loop {
        let prev_diff_count = ctx.different_pairs.borrow().len();

        for &rule_idx in rule_indices {
            let rule = &schema.swrl_rules[rule_idx];
            let mut bindings = vec![None; rule.num_vars as usize];

            if let SwrlBodyAtom::ClassAtom { arg, .. } = &rule.body[rule.trigger] {
                bind_arg(*arg, instance, &mut bindings);
            }

            resolve_body(
                &rule.body,
                &rule.remaining,
                &mut bindings,
                ctx,
                &mut |bindings| {
                    sink.firings.swrl += 1;
                    let _ = emit_head(&rule.head, bindings, ctx, sink);
                },
            );
        }

        if ctx.different_pairs.borrow().len() == prev_diff_count {
            break;
        }
    }
    Ok(())
}

/// Apply SWRL rules triggered by a property delta.
fn apply_swrl_property_rules(
    subject: TermId,
    predicate: TermId,
    object: TermId,
    schema: &CompiledSchema,
    ctx: &SwrlContext<'_>,
    sink: &mut SwrlSink<'_>,
) -> Result<()> {
    let rule_indices = match schema.swrl_by_prop_trigger.get(&predicate) {
        Some(indices) => indices,
        None => return Ok(()),
    };
    loop {
        let prev_diff_count = ctx.different_pairs.borrow().len();

        for &rule_idx in rule_indices {
            let rule = &schema.swrl_rules[rule_idx];
            let mut bindings = vec![None; rule.num_vars as usize];

            if let SwrlBodyAtom::PropertyAtom {
                subject: s_arg,
                object: o_arg,
                ..
            } = &rule.body[rule.trigger]
            {
                bind_arg(*s_arg, subject, &mut bindings);
                bind_arg(*o_arg, object, &mut bindings);
            }

            resolve_body(
                &rule.body,
                &rule.remaining,
                &mut bindings,
                ctx,
                &mut |bindings| {
                    sink.firings.swrl += 1;
                    let _ = emit_head(&rule.head, bindings, ctx, sink);
                },
            );
        }

        if ctx.different_pairs.borrow().len() == prev_diff_count {
            break;
        }
    }
    Ok(())
}

fn bind_arg(arg: SwrlArg, value: TermId, bindings: &mut [Option<TermId>]) {
    if let SwrlArg::Variable(v) = arg {
        bindings[v as usize] = Some(value);
    }
}

/// Build an immutable index of equivalence classes: root → [all members].
fn build_equality_class_index(uf: &UnionFind) -> BTreeMap<TermId, Vec<TermId>> {
    let mut index: BTreeMap<TermId, Vec<TermId>> = BTreeMap::new();
    for &term in uf.known_terms() {
        let root = uf.find_immutable(term);
        index.entry(root).or_default().push(term);
    }
    index.retain(|_, members| members.len() > 1);
    index
}

/// Apply SWRL rules whose trigger is SameIndividualAtom or DifferentIndividualsAtom.
///
/// These rules can't be dispatched from type/property deltas, so we enumerate
/// all known equality/difference pairs and bind the trigger variables directly.
fn apply_swrl_equality_rules(
    schema: &CompiledSchema,
    ctx: &SwrlContext<'_>,
    sink: &mut SwrlSink<'_>,
) -> Result<()> {
    if schema.swrl_equality_triggered.is_empty() {
        return Ok(());
    }

    loop {
        let prev_diff_count = ctx.different_pairs.borrow().len();

        for &rule_idx in &schema.swrl_equality_triggered {
            let rule = &schema.swrl_rules[rule_idx];
            let mut bindings = vec![None; rule.num_vars as usize];

            match &rule.body[rule.trigger] {
                SwrlBodyAtom::SameIndividualAtom { left, right } => {
                    let l_const = resolve_arg(*left, &bindings);
                    let r_const = resolve_arg(*right, &bindings);
                    let l_var = left.as_variable().map(|v| v as usize);
                    let r_var = right.as_variable().map(|v| v as usize);

                    match (l_const, r_const) {
                        (Some(lv), Some(rv)) => {
                            // Both sides constant — just check if they're in the same class.
                            if ctx.union_find.find_immutable(lv)
                                == ctx.union_find.find_immutable(rv)
                            {
                                resolve_body(
                                    &rule.body,
                                    &rule.remaining,
                                    &mut bindings,
                                    ctx,
                                    &mut |bindings| {
                                        sink.firings.swrl += 1;
                                        let _ = emit_head(&rule.head, bindings, ctx, sink);
                                    },
                                );
                            }
                        }
                        (Some(bound), None) | (None, Some(bound)) => {
                            let var = if l_const.is_some() {
                                r_var.unwrap()
                            } else {
                                l_var.unwrap()
                            };
                            let root = ctx.union_find.find_immutable(bound);
                            if let Some(members) = ctx.equality_class_index.get(&root) {
                                for &m in members {
                                    bindings[var] = Some(m);
                                    resolve_body(
                                        &rule.body,
                                        &rule.remaining,
                                        &mut bindings,
                                        ctx,
                                        &mut |bindings| {
                                            sink.firings.swrl += 1;
                                            let _ =
                                                emit_head(&rule.head, bindings, ctx, sink);
                                        },
                                    );
                                }
                                bindings[var] = None;
                            }
                        }
                        (None, None) => {
                            let lv = l_var.unwrap();
                            let rv = r_var.unwrap();
                            for members in ctx.equality_class_index.values() {
                                for (i, &a) in members.iter().enumerate() {
                                    for &b in &members[i + 1..] {
                                        bindings[lv] = Some(a);
                                        bindings[rv] = Some(b);
                                        resolve_body(
                                            &rule.body,
                                            &rule.remaining,
                                            &mut bindings,
                                            ctx,
                                            &mut |bindings| {
                                                sink.firings.swrl += 1;
                                                let _ = emit_head(
                                                    &rule.head, bindings, ctx, sink,
                                                );
                                            },
                                        );
                                        bindings[lv] = Some(b);
                                        bindings[rv] = Some(a);
                                        resolve_body(
                                            &rule.body,
                                            &rule.remaining,
                                            &mut bindings,
                                            ctx,
                                            &mut |bindings| {
                                                sink.firings.swrl += 1;
                                                let _ = emit_head(
                                                    &rule.head, bindings, ctx, sink,
                                                );
                                            },
                                        );
                                    }
                                }
                            }
                            bindings[lv] = None;
                            bindings[rv] = None;
                        }
                    }
                }
                SwrlBodyAtom::DifferentIndividualsAtom { left, right } => {
                    let l_const = resolve_arg(*left, &bindings);
                    let r_const = resolve_arg(*right, &bindings);
                    let l_var = left.as_variable().map(|v| v as usize);
                    let r_var = right.as_variable().map(|v| v as usize);
                    let pairs: Vec<(TermId, TermId)> =
                        ctx.different_pairs.borrow().iter().copied().collect();

                    match (l_const, r_const) {
                        (Some(lv), Some(rv)) => {
                            if pairs.contains(&(lv, rv)) {
                                resolve_body(
                                    &rule.body,
                                    &rule.remaining,
                                    &mut bindings,
                                    ctx,
                                    &mut |bindings| {
                                        sink.firings.swrl += 1;
                                        let _ = emit_head(&rule.head, bindings, ctx, sink);
                                    },
                                );
                            }
                        }
                        (Some(bound), None) | (None, Some(bound)) => {
                            let var = if l_const.is_some() {
                                r_var.unwrap()
                            } else {
                                l_var.unwrap()
                            };
                            let partners: Vec<TermId> = pairs
                                .iter()
                                .filter_map(|&(a, b)| {
                                    if l_const.is_some() && a == bound {
                                        Some(b)
                                    } else if r_const.is_some() && b == bound {
                                        Some(a)
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            for partner in partners {
                                bindings[var] = Some(partner);
                                resolve_body(
                                    &rule.body,
                                    &rule.remaining,
                                    &mut bindings,
                                    ctx,
                                    &mut |bindings| {
                                        sink.firings.swrl += 1;
                                        let _ = emit_head(&rule.head, bindings, ctx, sink);
                                    },
                                );
                            }
                            bindings[var] = None;
                        }
                        (None, None) => {
                            let lv = l_var.unwrap();
                            let rv = r_var.unwrap();
                            for (a, b) in pairs {
                                bindings[lv] = Some(a);
                                bindings[rv] = Some(b);
                                resolve_body(
                                    &rule.body,
                                    &rule.remaining,
                                    &mut bindings,
                                    ctx,
                                    &mut |bindings| {
                                        sink.firings.swrl += 1;
                                        let _ = emit_head(&rule.head, bindings, ctx, sink);
                                    },
                                );
                            }
                            bindings[lv] = None;
                            bindings[rv] = None;
                        }
                    }
                }
                _ => unreachable!(),
            }
        }

        if ctx.different_pairs.borrow().len() == prev_diff_count {
            break;
        }
    }
    Ok(())
}

fn var_index(arg: SwrlArg) -> usize {
    match arg {
        SwrlArg::Variable(v) => v as usize,
        SwrlArg::Constant(_) => unreachable!(),
    }
}

fn resolve_arg(arg: SwrlArg, bindings: &[Option<TermId>]) -> Option<TermId> {
    match arg {
        SwrlArg::Variable(v) => bindings[v as usize],
        SwrlArg::Constant(c) => Some(c),
    }
}

/// Recursively resolve remaining body atoms against indexes.
fn resolve_body(
    all_atoms: &[SwrlBodyAtom],
    remaining: &[usize],
    bindings: &mut Vec<Option<TermId>>,
    ctx: &SwrlContext<'_>,
    callback: &mut impl FnMut(&[Option<TermId>]),
) {
    if remaining.is_empty() {
        callback(bindings);
        return;
    }

    let atom_idx = remaining[0];
    let rest = &remaining[1..];
    let atom = &all_atoms[atom_idx];

    match atom {
        SwrlBodyAtom::ClassAtom { class, arg } => match resolve_arg(*arg, bindings) {
            Some(instance) => {
                if ctx.indexes.types.has_type(instance, *class) {
                    resolve_body(all_atoms, rest, bindings, ctx, callback);
                }
            }
            None => {
                let var = match arg {
                    SwrlArg::Variable(v) => *v as usize,
                    SwrlArg::Constant(_) => unreachable!(),
                };
                for &inst in ctx.indexes.types.instances_of(*class) {
                    bindings[var] = Some(inst);
                    resolve_body(all_atoms, rest, bindings, ctx, callback);
                }
                bindings[var] = None;
            }
        },
        SwrlBodyAtom::PropertyAtom {
            property,
            subject,
            object,
        } => {
            let s = resolve_arg(*subject, bindings);
            let o = resolve_arg(*object, bindings);
            match (s, o) {
                (Some(sv), Some(ov)) => {
                    if ctx
                        .indexes
                        .properties
                        .objects_for(*property, sv)
                        .contains(&ov)
                    {
                        resolve_body(all_atoms, rest, bindings, ctx, callback);
                    }
                }
                (Some(sv), None) => {
                    let var = match object {
                        SwrlArg::Variable(v) => *v as usize,
                        SwrlArg::Constant(_) => unreachable!(),
                    };
                    for &obj in ctx.indexes.properties.objects_for(*property, sv) {
                        bindings[var] = Some(obj);
                        resolve_body(all_atoms, rest, bindings, ctx, callback);
                    }
                    bindings[var] = None;
                }
                (None, Some(ov)) => {
                    let var = match subject {
                        SwrlArg::Variable(v) => *v as usize,
                        SwrlArg::Constant(_) => unreachable!(),
                    };
                    for &subj in ctx.indexes.properties.subjects_for(*property, ov) {
                        bindings[var] = Some(subj);
                        resolve_body(all_atoms, rest, bindings, ctx, callback);
                    }
                    bindings[var] = None;
                }
                (None, None) => {
                    let s_var = match subject {
                        SwrlArg::Variable(v) => *v as usize,
                        SwrlArg::Constant(_) => unreachable!(),
                    };
                    let o_var = match object {
                        SwrlArg::Variable(v) => *v as usize,
                        SwrlArg::Constant(_) => unreachable!(),
                    };
                    let triples: Vec<(TermId, TermId)> =
                        ctx.indexes.properties.triples_for(*property).collect();
                    for (subj, obj) in triples {
                        bindings[s_var] = Some(subj);
                        bindings[o_var] = Some(obj);
                        resolve_body(all_atoms, rest, bindings, ctx, callback);
                    }
                    bindings[s_var] = None;
                    bindings[o_var] = None;
                }
            }
        }
        SwrlBodyAtom::SameIndividualAtom { left, right } => {
            let l = resolve_arg(*left, bindings);
            let r = resolve_arg(*right, bindings);
            match (l, r) {
                (Some(lv), Some(rv)) => {
                    if ctx.union_find.find_immutable(lv) == ctx.union_find.find_immutable(rv) {
                        resolve_body(all_atoms, rest, bindings, ctx, callback);
                    }
                }
                (Some(bound), None) | (None, Some(bound)) => {
                    let var = var_index(if l.is_some() { *right } else { *left });
                    let root = ctx.union_find.find_immutable(bound);
                    if let Some(members) = ctx.equality_class_index.get(&root) {
                        for &member in members {
                            bindings[var] = Some(member);
                            resolve_body(all_atoms, rest, bindings, ctx, callback);
                        }
                        bindings[var] = None;
                    }
                }
                (None, None) => {}
            }
        }
        SwrlBodyAtom::DifferentIndividualsAtom { left, right } => {
            let l = resolve_arg(*left, bindings);
            let r = resolve_arg(*right, bindings);
            match (l, r) {
                (Some(lv), Some(rv)) => {
                    if ctx.different_pairs.borrow().contains(&(lv, rv)) {
                        resolve_body(all_atoms, rest, bindings, ctx, callback);
                    }
                }
                (Some(bound), None) | (None, Some(bound)) => {
                    let var = var_index(if l.is_some() { *right } else { *left });
                    let partners: Vec<TermId> = ctx
                        .different_pairs
                        .borrow()
                        .range((bound, TermId::MIN)..=(bound, TermId::MAX))
                        .map(|&(_, partner)| partner)
                        .collect();
                    for partner in partners {
                        bindings[var] = Some(partner);
                        resolve_body(all_atoms, rest, bindings, ctx, callback);
                    }
                    bindings[var] = None;
                }
                (None, None) => {}
            }
        }
    }
}

/// Emit the head atom given complete bindings.
fn emit_head(
    head: &SwrlHeadAtom,
    bindings: &[Option<TermId>],
    ctx: &SwrlContext<'_>,
    sink: &mut SwrlSink<'_>,
) -> Result<()> {
    match head {
        SwrlHeadAtom::ClassAtom { class, arg } => {
            if let Some(instance) = resolve_arg(*arg, bindings) {
                sink.candidate_types.push((instance, *class))?;
            }
        }
        SwrlHeadAtom::PropertyAtom {
            property,
            subject,
            object,
        } => {
            if let (Some(s), Some(o)) = (
                resolve_arg(*subject, bindings),
                resolve_arg(*object, bindings),
            ) {
                sink.candidate_properties.push((s, *property, o))?;
            }
        }
        SwrlHeadAtom::SameIndividualAtom { left, right } => {
            if let (Some(l), Some(r)) =
                (resolve_arg(*left, bindings), resolve_arg(*right, bindings))
            {
                sink.candidate_properties.push((l, ctx.owl_same_as, r))?;
            }
        }
        SwrlHeadAtom::DifferentIndividualsAtom { left, right } => {
            if let (Some(l), Some(r)) =
                (resolve_arg(*left, bindings), resolve_arg(*right, bindings))
            {
                sink.different_pairs.push((l, r));
                let mut set = ctx.different_pairs.borrow_mut();
                set.insert((l, r));
                set.insert((r, l));
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
///
/// Returns `(new_equalities, literal_conflicts)`. Literal conflicts are pairs
/// of distinct literal TermIds that equality rules tried to merge — these are
/// inconsistencies because literal identity is fixed.
fn evaluate_equality_rules(
    store: &mut FactStore,
    schema: &CompiledSchema,
    union_find: &mut UnionFind,
    owl_same_as: TermId,
    dictionary: &Dictionary,
) -> Result<(usize, Vec<Inconsistency>)> {
    let mut new_equalities = 0usize;
    let mut literal_conflicts: Vec<Inconsistency> = Vec::new();

    let is_literal = |id: TermId| matches!(dictionary.decode(id), Some(Term::Literal(_)));

    let has_fp = !schema.functional_properties.is_empty();
    let has_ifp = !schema.inverse_functional_properties.is_empty();
    let has_mc1 = !schema.max_card_one.is_empty();
    let has_key = !schema.has_key.is_empty();

    // Fast path: no schema equality axioms — only scan for asserted sameAs.
    if !has_fp && !has_ifp && !has_mc1 && !has_key {
        for result in store.known_properties_iter()? {
            let (s, p, o) = result?;
            if p == owl_same_as && union_find.union(s, o) {
                new_equalities += 1;
            }
        }
        return Ok((new_equalities, literal_conflicts));
    }

    // Build instance → classes map for max_card_one and has_key (need type membership).
    let mut instance_classes: BTreeMap<TermId, Vec<TermId>> = BTreeMap::new();
    if has_mc1 || has_key {
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
    // by (pred, canon_obj) for IFP, by (canon_subj, pred) for MC1/HasKey.
    let mut fp_groups: BTreeMap<(TermId, TermId), Vec<TermId>> = BTreeMap::new();
    let mut ifp_groups: BTreeMap<(TermId, TermId), Vec<TermId>> = BTreeMap::new();
    let mut mc1_groups: BTreeMap<(TermId, TermId), Vec<TermId>> = BTreeMap::new();
    let mut key_values: BTreeMap<(TermId, TermId), Vec<TermId>> = BTreeMap::new();

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
        if has_key && schema.has_key_preds.contains(&p) {
            key_values.entry((cs, p)).or_default().push(co);
        }
    }

    // FunctionalProperty: union multiple objects for same (pred, subject).
    // Literals cannot be merged — distinct literals are an inconsistency.
    for (&(pred, subj), objects) in &fp_groups {
        new_equalities +=
            union_all_checking_literals(subj, pred, objects, union_find, &is_literal, &mut literal_conflicts);
    }

    // InverseFunctionalProperty: union multiple subjects for same (pred, object).
    // Subjects are always individuals, never literals.
    for subjects in ifp_groups.values() {
        new_equalities += union_find.union_all(subjects);
    }

    // MaxCardinality 1: for each (subject, pred) group, check matching axioms via index.
    // Objects may be literals for data properties.
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
                new_equalities += union_all_checking_literals(
                    subj,
                    pred,
                    &qualifying,
                    union_find,
                    &is_literal,
                    &mut literal_conflicts,
                );
            }
        }
    }

    // HasKey: for each HasKey(C, [P1,...,Pn]), group instances of C by their
    // key-tuple (values for P1,...,Pn). Instances sharing a key-tuple are merged.
    if has_key {
        // Build class → instances reverse index for HasKey classes only.
        let mut class_to_instances: BTreeMap<TermId, Vec<TermId>> = BTreeMap::new();
        let key_classes: BTreeSet<TermId> = schema.has_key.iter().map(|(cls, _)| *cls).collect();
        for (&inst, classes) in &instance_classes {
            for &c in classes {
                if key_classes.contains(&c) {
                    class_to_instances.entry(c).or_default().push(inst);
                }
            }
        }

        for (class, key_props) in &schema.has_key {
            let instances = match class_to_instances.get(class) {
                Some(insts) => insts,
                None => continue,
            };

            // Group instances by key-tuple. Skip instances missing any key
            // property. Multi-valued key properties produce cross-product tuples.
            let mut tuple_to_instances: BTreeMap<Vec<TermId>, Vec<TermId>> = BTreeMap::new();
            for &inst in instances {
                let mut tuples: Vec<Vec<TermId>> = vec![vec![]];
                let mut complete = true;
                for &prop in key_props {
                    let values = match key_values.get(&(inst, prop)) {
                        Some(vs) => vs,
                        None => {
                            complete = false;
                            break;
                        }
                    };
                    let mut canon_vals: Vec<TermId> =
                        values.iter().map(|&v| union_find.canonical(v)).collect();
                    canon_vals.sort_unstable();
                    canon_vals.dedup();

                    // Cap cross-product to prevent combinatorial blowup from
                    // pathological multi-valued key properties.
                    let next_size = tuples.len() * canon_vals.len();
                    if next_size > 10_000 {
                        tracing::warn!(
                            "HasKey tuple expansion for instance {} exceeds 10k — skipping",
                            inst
                        );
                        complete = false;
                        break;
                    }

                    let mut next_tuples = Vec::with_capacity(next_size);
                    for partial in &tuples {
                        for &val in &canon_vals {
                            let mut extended = partial.clone();
                            extended.push(val);
                            next_tuples.push(extended);
                        }
                    }
                    tuples = next_tuples;
                }
                if complete {
                    for tuple in tuples {
                        tuple_to_instances.entry(tuple).or_default().push(inst);
                    }
                }
            }

            for instances_group in tuple_to_instances.values() {
                new_equalities += union_find.union_all(instances_group);
            }
        }
    }

    Ok((new_equalities, literal_conflicts))
}

/// Merge items via union-find, but skip merges between distinct literals.
/// Distinct-literal pairs are recorded as `LiteralConflict` inconsistencies
/// with the individual and property that caused the attempted merge.
fn union_all_checking_literals(
    individual: TermId,
    property: TermId,
    items: &[TermId],
    union_find: &mut UnionFind,
    is_literal: &impl Fn(TermId) -> bool,
    conflicts: &mut Vec<Inconsistency>,
) -> usize {
    if items.len() <= 1 {
        return 0;
    }
    // Partition into literals and non-literals.
    let mut literals = Vec::new();
    let mut non_literals = Vec::new();
    for &item in items {
        if is_literal(item) {
            literals.push(item);
        } else {
            non_literals.push(item);
        }
    }
    let count = union_find.union_all(&non_literals);
    // Distinct literals in the same merge group → inconsistency.
    if literals.len() >= 2 {
        let mut deduped: Vec<TermId> = literals.iter().map(|&l| union_find.canonical(l)).collect();
        deduped.sort_unstable();
        deduped.dedup();
        if deduped.len() >= 2 {
            conflicts.push(Inconsistency::LiteralConflict {
                individual,
                property,
                literal_a: deduped[0],
                literal_b: deduped[1],
            });
        }
    }
    count
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
                store.derived_properties_mut().push((x, owl_same_as, y))?;
                store.derived_properties_mut().push((y, owl_same_as, x))?;
                count += 2;
            }
        }
    }
    Ok(count)
}

// ─── Post-fixpoint DifferentIndividuals from DisjointProperties ─────────────

/// Infer DifferentIndividuals pairs from DisjointProperties axioms.
///
/// Rule: property(x, P, y) ∧ property(x, Q, z) ∧ DisjointProperties(P, Q) → y ≠ z
///
/// Runs post-fixpoint (DifferentIndividuals is negative — doesn't produce
/// new type/property facts, only feeds into inconsistency checking).
///
/// Accepts the pre-built property assertions index to avoid a redundant
/// full scan of `known_properties_iter()` (the same index is used by
/// `check_disjoint_properties` in the inconsistency checker).
pub fn infer_different_from_disjoint_properties(
    prop_assertions: &BTreeMap<TermId, BTreeSet<(TermId, TermId)>>,
    schema: &CompiledSchema,
) -> Vec<(TermId, TermId)> {
    if schema.disjoint_property_pairs.is_empty() {
        return Vec::new();
    }

    // Re-index as pred → subject → [objects] for efficient cross-product
    let mut by_pred_subj: BTreeMap<TermId, BTreeMap<TermId, Vec<TermId>>> = BTreeMap::new();
    for (&pred, pairs) in prop_assertions {
        let subj_map = by_pred_subj.entry(pred).or_default();
        for &(s, o) in pairs {
            subj_map.entry(s).or_default().push(o);
        }
    }

    let mut pairs = Vec::new();
    for &(pa, pb) in &schema.disjoint_property_pairs {
        if let (Some(subjs_a), Some(subjs_b)) = (by_pred_subj.get(&pa), by_pred_subj.get(&pb)) {
            for (s, objects_a) in subjs_a {
                if let Some(objects_b) = subjs_b.get(s) {
                    for &y in objects_a {
                        for &z in objects_b {
                            if y != z {
                                pairs.push((y, z));
                            }
                        }
                    }
                }
            }
        }
    }

    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

// ─── Fixpoint loop ──────────────────────────────────────────────────────────

pub fn materialize(
    store: &mut FactStore,
    schema: &CompiledSchema,
    max_iterations: Option<usize>,
    engine_budget: usize,
    owl_same_as: TermId,
    dictionary: &Dictionary,
) -> Result<MaterializeResult> {
    let mut stats = ReasoningStats::default();
    let relation_budget = engine_budget / 4;
    let work_dir = store.work_dir().to_path_buf();

    let mut candidate_types =
        BinaryRelation::with_compression(&work_dir, "engine-cand-types", relation_budget, false);
    let mut candidate_properties =
        TernaryRelation::with_compression(&work_dir, "engine-cand-props", relation_budget, false);
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

    let mut swrl_different_pairs: Vec<(TermId, TermId)> = Vec::new();
    let mut literal_conflicts: Vec<Inconsistency> = Vec::new();

    // Outer equality fixpoint: run inner fixpoint, check for new equalities
    // (from FunctionalProperty, InverseFunctionalProperty, MaxCardinality 1,
    // or asserted owl:sameAs), expand facts across equivalence classes, repeat.
    loop {
        let mut swrl_state = SwrlState {
            union_find: &union_find,
            owl_same_as,
            different_pairs: &mut swrl_different_pairs,
        };
        inner_fixpoint(
            store,
            schema,
            max_iterations,
            &mut stats,
            &mut candidate_types,
            &mut candidate_properties,
            relation_budget,
            &mut swrl_state,
        )?;

        let (new_equalities, lit_conflicts) =
            evaluate_equality_rules(store, schema, &mut union_find, owl_same_as, dictionary)?;
        literal_conflicts.extend(lit_conflicts);
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
    Ok(MaterializeResult {
        stats,
        union_find,
        swrl_different_pairs,
        literal_conflicts,
    })
}

/// SWRL-related state threaded through the inner fixpoint.
struct SwrlState<'a> {
    union_find: &'a UnionFind,
    owl_same_as: TermId,
    different_pairs: &'a mut Vec<(TermId, TermId)>,
}

/// Run the inner (non-equality) fixpoint to completion.
///
/// Seeds from asserted facts and processes any pre-populated candidates
/// (e.g. from canonical rewrites after equality discovery).
#[allow(clippy::too_many_arguments)]
fn inner_fixpoint(
    store: &mut FactStore,
    schema: &CompiledSchema,
    max_iterations: Option<usize>,
    stats: &mut ReasoningStats,
    candidate_types: &mut BinaryRelation,
    candidate_properties: &mut TernaryRelation,
    relation_budget: usize,
    swrl_state: &mut SwrlState<'_>,
) -> Result<()> {
    let work_dir = store.work_dir().to_path_buf();
    let mut delta_types =
        BinaryRelation::with_compression(&work_dir, "engine-delta-types", relation_budget, false);
    let mut delta_properties =
        TernaryRelation::with_compression(&work_dir, "engine-delta-props", relation_budget, false);

    let mut chain_bufs = ChainBuffers::new();

    let needs_property_index = !schema.indexed_predicates.is_empty();
    let needs_type_index = !schema.indexed_classes.is_empty();
    let has_swrl = !schema.swrl_rules.is_empty();
    // cls-hv2 needs the seed join pass but doesn't contribute to indexed sets
    let needs_seed_join_pass =
        needs_property_index || needs_type_index || !schema.has_value_by_class.is_empty();

    // Symmetric set of different-individual pairs for O(log n) lookup.
    // Wrapped in RefCell so SWRL body resolution (reads) and head emission
    // (writes) can share it during recursive rule evaluation.
    let different_pairs_set: RefCell<BTreeSet<(TermId, TermId)>> = RefCell::new(
        schema
            .different_individual_pairs
            .iter()
            .flat_map(|&(a, b)| [(a, b), (b, a)])
            .collect(),
    );

    let firings = &mut stats.rule_firings;

    for result in store.asserted_types_iter()? {
        let (instance, class) = result?;
        apply_type_rules(instance, class, schema, candidate_types, firings)?;
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
            firings,
        )?;
    }

    // Persistent indexes, incrementally updated with each iteration's deltas.
    let mut type_index = if needs_type_index {
        build_type_index(store, schema)?
    } else {
        TypeIndex::new()
    };
    let mut known_prop_index = if needs_property_index {
        build_property_index(store, schema)?
    } else {
        PropertyIndex::new()
    };

    if needs_seed_join_pass || has_swrl {
        let seed_indexes = JoinIndexes {
            types: &type_index,
            properties: &known_prop_index,
        };
        let swrl_ctx = SwrlContext {
            indexes: &seed_indexes,
            union_find: swrl_state.union_find,
            different_pairs: &different_pairs_set,
            equality_class_index: build_equality_class_index(swrl_state.union_find),
            owl_same_as: swrl_state.owl_same_as,
        };

        for result in store.asserted_types_iter()? {
            let (instance, class) = result?;
            if needs_seed_join_pass {
                apply_type_join_rules(
                    instance,
                    class,
                    schema,
                    &seed_indexes,
                    candidate_types,
                    candidate_properties,
                    firings,
                )?;
            }
            if has_swrl {
                let mut sink = SwrlSink {
                    candidate_types,
                    candidate_properties,
                    different_pairs: swrl_state.different_pairs,
                    firings,
                };
                apply_swrl_type_rules(instance, class, schema, &swrl_ctx, &mut sink)?;
            }
        }
        for result in store.asserted_properties_iter()? {
            let (subject, predicate, object) = result?;
            if needs_seed_join_pass {
                apply_property_join_rules(
                    subject,
                    predicate,
                    object,
                    schema,
                    &seed_indexes,
                    candidate_types,
                    candidate_properties,
                    firings,
                    &mut chain_bufs,
                )?;
            }
            if has_swrl {
                let mut sink = SwrlSink {
                    candidate_types,
                    candidate_properties,
                    different_pairs: swrl_state.different_pairs,
                    firings,
                };
                apply_swrl_property_rules(
                    subject, predicate, object, schema, &swrl_ctx, &mut sink,
                )?;
            }
        }

        // Evaluate SWRL rules triggered by equality/difference atoms.
        {
            let mut sink = SwrlSink {
                candidate_types,
                candidate_properties,
                different_pairs: swrl_state.different_pairs,
                firings,
            };
            apply_swrl_equality_rules(schema, &swrl_ctx, &mut sink)?;
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
        stats.iteration_details.push(IterationReport {
            iteration: stats.iterations,
            new_types: new_type_count,
            new_properties: new_prop_count,
        });

        candidate_types.clear();
        candidate_properties.clear();

        delta_types.compact()?;
        delta_properties.compact()?;

        // Merge this iteration's deltas into persistent indexes so that
        // join lookups see known ∪ delta. This ensures multi-way joins
        // (intersection conjuncts, property chain walks, SWRL body atoms)
        // can find facts from both known and delta in a single pass.
        if needs_type_index {
            for result in MergeBinaryIter::new(delta_types.segment_iters()?)? {
                let (instance, class) = result?;
                if schema.indexed_classes.contains(&class) {
                    type_index.insert_sorted(instance, class);
                }
            }
        }
        if needs_property_index {
            for result in MergeTernaryIter::new(delta_properties.segment_iters()?)? {
                let (s, p, o) = result?;
                if schema.indexed_predicates.contains(&p) {
                    known_prop_index.insert_sorted(s, p, o);
                }
            }
        }

        {
            let indexes = JoinIndexes {
                types: &type_index,
                properties: &known_prop_index,
            };
            let swrl_ctx = SwrlContext {
                indexes: &indexes,
                union_find: swrl_state.union_find,
                different_pairs: &different_pairs_set,
                equality_class_index: build_equality_class_index(swrl_state.union_find),
                owl_same_as: swrl_state.owl_same_as,
            };

            let firings = &mut stats.rule_firings;
            for result in MergeBinaryIter::new(delta_types.segment_iters()?)? {
                let (instance, class) = result?;
                store.derived_types_mut().push((instance, class))?;
                apply_type_rules(instance, class, schema, candidate_types, firings)?;
                apply_type_join_rules(
                    instance,
                    class,
                    schema,
                    &indexes,
                    candidate_types,
                    candidate_properties,
                    firings,
                )?;
                if has_swrl {
                    let mut sink = SwrlSink {
                        candidate_types,
                        candidate_properties,
                        different_pairs: swrl_state.different_pairs,
                        firings,
                    };
                    apply_swrl_type_rules(instance, class, schema, &swrl_ctx, &mut sink)?;
                }
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
                    candidate_types,
                    candidate_properties,
                    firings,
                )?;
                apply_property_join_rules(
                    subject,
                    predicate,
                    object,
                    schema,
                    &indexes,
                    candidate_types,
                    candidate_properties,
                    firings,
                    &mut chain_bufs,
                )?;
                if has_swrl {
                    let mut sink = SwrlSink {
                        candidate_types,
                        candidate_properties,
                        different_pairs: swrl_state.different_pairs,
                        firings,
                    };
                    apply_swrl_property_rules(
                        subject, predicate, object, schema, &swrl_ctx, &mut sink,
                    )?;
                }
            }

            // Evaluate SWRL rules triggered by equality/difference atoms.
            {
                let mut sink = SwrlSink {
                    candidate_types,
                    candidate_properties,
                    different_pairs: swrl_state.different_pairs,
                    firings,
                };
                apply_swrl_equality_rules(schema, &swrl_ctx, &mut sink)?;
            }
        }
    }

    Ok(())
}
