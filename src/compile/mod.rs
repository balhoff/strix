pub mod ir;

use std::collections::{BTreeMap, BTreeSet};

use crate::dict::TermId;
use crate::owl::RawSchema;

#[derive(Clone, Debug)]
pub struct CompiledSchema {
    // RDFS (Phase 1)
    pub superclasses: BTreeMap<TermId, Vec<TermId>>,
    pub superproperties: BTreeMap<TermId, Vec<TermId>>,
    pub domains: BTreeMap<TermId, Vec<TermId>>,
    pub ranges: BTreeMap<TermId, Vec<TermId>>,

    // Property axioms (Phase 2)
    pub inverses: BTreeMap<TermId, Vec<TermId>>,
    pub symmetric_properties: BTreeSet<TermId>,
    pub transitive_properties: BTreeSet<TermId>,
    /// (super_property, [chain_predicates]) — non-transitive property chains
    pub property_chains: Vec<(TermId, Vec<TermId>)>,
    /// pred → [(chain_index, position)] — which chains fire on this predicate
    pub chain_triggers: BTreeMap<TermId, Vec<(usize, usize)>>,

    // owl:Thing-derived rules
    /// Classes every individual belongs to (from SubClassOf(owl:Thing, C)).
    pub universal_types: Vec<TermId>,
    /// prop → [class] — property-existence rules from someValuesFrom with
    /// owl:Thing filler: property(x,P,_) → type(x,C)
    pub svf_thing_by_prop: BTreeMap<TermId, Vec<TermId>>,

    // Class restriction rules (Phase 2, Step 4)
    /// prop → [(class, value)] — cls-hv1: property(x,P,v) → type(x,C)
    pub has_value_by_prop: BTreeMap<TermId, Vec<(TermId, TermId)>>,
    /// class → [(prop, value)] — cls-hv2: type(x,A) → property(x,P,v)
    pub has_value_by_class: BTreeMap<TermId, Vec<(TermId, TermId)>>,
    /// prop → [(class, filler)] — cls-svf1: property(x,P,y) ∧ type(y,D) → type(x,C)
    pub some_values_from_by_prop: BTreeMap<TermId, Vec<(TermId, TermId)>>,
    /// filler → [(class, prop)] — cls-svf1 (type-triggered): type(y,D) → check prop_index
    pub some_values_from_by_filler: BTreeMap<TermId, Vec<(TermId, TermId)>>,
    /// class → [(prop, filler)] — cls-avf: type(x,A) ∧ property(x,P,y) → type(y,B)
    pub all_values_from_by_class: BTreeMap<TermId, Vec<(TermId, TermId)>>,
    /// prop → [(class, filler)] — cls-avf (property-triggered): property(x,P,y) → check type_index
    pub all_values_from_by_prop: BTreeMap<TermId, Vec<(TermId, TermId)>>,
    /// intersection_class → [conjuncts] — cls-int2: type(x,C) → type(x,Ci)
    pub intersection_conjuncts: BTreeMap<TermId, Vec<TermId>>,
    /// conjunct → [intersection_classes] — cls-int1: check all conjuncts → type(x,C)
    pub conjunct_of: BTreeMap<TermId, Vec<TermId>>,

    /// Predicates that require in-memory indexing for join evaluation.
    pub indexed_predicates: BTreeSet<TermId>,
    /// Classes that require in-memory indexing for join evaluation.
    pub indexed_classes: BTreeSet<TermId>,

    pub schema_iterations: usize,
    pub schema_inferred: usize,
    pub rule_set: ir::RuleSet,
}

impl CompiledSchema {
    pub fn superclasses_for(&self, class: TermId) -> &[TermId] {
        self.superclasses
            .get(&class)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn superproperties_for(&self, property: TermId) -> &[TermId] {
        self.superproperties
            .get(&property)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn domains_for(&self, property: TermId) -> &[TermId] {
        self.domains
            .get(&property)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn ranges_for(&self, property: TermId) -> &[TermId] {
        self.ranges.get(&property).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn inverses_for(&self, property: TermId) -> &[TermId] {
        self.inverses
            .get(&property)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn is_symmetric(&self, property: TermId) -> bool {
        self.symmetric_properties.contains(&property)
    }

    pub fn is_transitive(&self, property: TermId) -> bool {
        self.transitive_properties.contains(&property)
    }
}

/// Compiles a raw schema into indexed lookup tables for the engine.
///
/// `owl_thing` is the TermId for `owl:Thing`. It is used to:
/// - Strip superclass entries targeting owl:Thing (trivially true).
/// - Extract universal type rules from `SubClassOf(owl:Thing, C)`.
/// - Rewrite someValuesFrom with owl:Thing filler to property-existence rules.
/// - Drop allValuesFrom / domain / range entries with owl:Thing (trivially true).
/// - Remove owl:Thing from intersectionOf conjunct lists.
/// - Normalize MaxCardinality entries with owl:Thing filler to unqualified.
pub fn compile_schema(schema: &RawSchema, owl_thing: TermId) -> CompiledSchema {
    let (mut subclass_closure, subclass_iterations, subclass_inferred) =
        transitive_closure(&schema.subclasses);
    let (subproperty_closure, subproperty_iterations, subproperty_inferred) =
        transitive_closure(&schema.subproperties);

    // Extract universal types: SubClassOf(owl:Thing, C) means every individual
    // is of type C. Collect before filtering so we don't lose them.
    let universal_types: Vec<TermId> = subclass_closure
        .iter()
        .filter(|&&(sub, sup)| sub == owl_thing && sup != owl_thing)
        .map(|&(_, sup)| sup)
        .collect();

    // Strip owl:Thing as both superclass target (trivially true for every
    // individual) and as subclass source (now handled by universal_types).
    subclass_closure.retain(|&(sub, sup)| sup != owl_thing && sub != owl_thing);

    let mut has_value_by_prop: BTreeMap<TermId, Vec<(TermId, TermId)>> = BTreeMap::new();
    let mut has_value_by_class: BTreeMap<TermId, Vec<(TermId, TermId)>> = BTreeMap::new();
    for &(class, prop, val) in &schema.has_value_super {
        has_value_by_prop.entry(prop).or_default().push((class, val));
    }
    for &(class, prop, val) in &schema.has_value_sub {
        has_value_by_class.entry(class).or_default().push((prop, val));
    }

    // someValuesFrom: separate owl:Thing filler into property-existence rules
    let mut some_values_from_by_prop: BTreeMap<TermId, Vec<(TermId, TermId)>> = BTreeMap::new();
    let mut some_values_from_by_filler: BTreeMap<TermId, Vec<(TermId, TermId)>> = BTreeMap::new();
    let mut svf_thing_by_prop: BTreeMap<TermId, Vec<TermId>> = BTreeMap::new();
    for &(class, prop, filler) in &schema.some_values_from {
        if filler == owl_thing {
            svf_thing_by_prop.entry(prop).or_default().push(class);
        } else {
            some_values_from_by_prop
                .entry(prop)
                .or_default()
                .push((class, filler));
            some_values_from_by_filler
                .entry(filler)
                .or_default()
                .push((class, prop));
        }
    }

    // allValuesFrom: drop owl:Thing filler (trivially true)
    let mut all_values_from_by_class: BTreeMap<TermId, Vec<(TermId, TermId)>> = BTreeMap::new();
    let mut all_values_from_by_prop: BTreeMap<TermId, Vec<(TermId, TermId)>> = BTreeMap::new();
    for &(class, prop, filler) in &schema.all_values_from {
        if filler == owl_thing {
            continue;
        }
        all_values_from_by_class
            .entry(class)
            .or_default()
            .push((prop, filler));
        all_values_from_by_prop
            .entry(prop)
            .or_default()
            .push((class, filler));
    }

    // intersectionOf: remove owl:Thing from conjunct lists
    let mut intersection_conjuncts: BTreeMap<TermId, Vec<TermId>> = BTreeMap::new();
    let mut conjunct_of: BTreeMap<TermId, Vec<TermId>> = BTreeMap::new();
    for (class, conjuncts) in &schema.intersection_of {
        let filtered: Vec<TermId> = conjuncts
            .iter()
            .copied()
            .filter(|&c| c != owl_thing)
            .collect();
        if filtered.is_empty() {
            continue;
        }
        intersection_conjuncts.insert(*class, filtered.clone());
        for &conjunct in &filtered {
            conjunct_of.entry(conjunct).or_default().push(*class);
        }
    }

    // Domain/range: filter out owl:Thing targets (trivially true)
    let filtered_domains: BTreeSet<(TermId, TermId)> = schema
        .domains
        .iter()
        .copied()
        .filter(|&(_, cls)| cls != owl_thing)
        .collect();
    let filtered_ranges: BTreeSet<(TermId, TermId)> = schema
        .ranges
        .iter()
        .copied()
        .filter(|&(_, cls)| cls != owl_thing)
        .collect();

    // Property chains: normalize self-join chains [p,p,...] → p to transitive
    let mut transitive_properties = schema.transitive_properties.clone();
    let mut property_chains: Vec<(TermId, Vec<TermId>)> = Vec::new();
    let mut chain_triggers: BTreeMap<TermId, Vec<(usize, usize)>> = BTreeMap::new();
    for (super_prop, chain) in &schema.property_chains {
        if chain.iter().all(|&p| p == *super_prop) {
            transitive_properties.insert(*super_prop);
        } else {
            let idx = property_chains.len();
            for (pos, &pred) in chain.iter().enumerate() {
                chain_triggers.entry(pred).or_default().push((idx, pos));
            }
            property_chains.push((*super_prop, chain.clone()));
        }
    }

    // Predicates needing in-memory indexing for join-based rules.
    // Built from filtered lookup tables so owl:Thing-only entries are excluded.
    let mut indexed_predicates = transitive_properties.clone();
    for (_, chain) in &property_chains {
        indexed_predicates.extend(chain);
    }
    indexed_predicates.extend(some_values_from_by_prop.keys());
    indexed_predicates.extend(all_values_from_by_prop.keys());

    // Classes needing in-memory indexing for join-based rules.
    let mut indexed_classes: BTreeSet<TermId> = BTreeSet::new();
    indexed_classes.extend(some_values_from_by_filler.keys());
    indexed_classes.extend(all_values_from_by_class.keys());
    indexed_classes.extend(intersection_conjuncts.values().flatten());

    CompiledSchema {
        superclasses: to_map(&subclass_closure),
        superproperties: to_map(&subproperty_closure),
        domains: to_map(&filtered_domains),
        ranges: to_map(&filtered_ranges),
        inverses: to_map(&schema.inverse_properties),
        symmetric_properties: schema.symmetric_properties.clone(),
        transitive_properties,
        property_chains,
        chain_triggers,
        universal_types,
        svf_thing_by_prop,
        has_value_by_prop,
        has_value_by_class,
        some_values_from_by_prop,
        some_values_from_by_filler,
        all_values_from_by_class,
        all_values_from_by_prop,
        intersection_conjuncts,
        conjunct_of,
        indexed_predicates,
        indexed_classes,
        schema_iterations: subclass_iterations.max(subproperty_iterations),
        schema_inferred: subclass_inferred + subproperty_inferred,
        rule_set: ir::RuleSet::build(),
    }
}

fn to_map(pairs: &BTreeSet<(TermId, TermId)>) -> BTreeMap<TermId, Vec<TermId>> {
    let mut map = BTreeMap::<TermId, Vec<TermId>>::new();
    for (left, right) in pairs {
        if left == right {
            continue;
        }
        map.entry(*left).or_default().push(*right);
    }
    for values in map.values_mut() {
        values.sort_unstable();
        values.dedup();
    }
    map
}

fn transitive_closure(
    seed: &BTreeSet<(TermId, TermId)>,
) -> (BTreeSet<(TermId, TermId)>, usize, usize) {
    // Build adjacency list from seed edges
    let mut successors: BTreeMap<TermId, Vec<TermId>> = BTreeMap::new();
    for &(from, to) in seed {
        successors.entry(from).or_default().push(to);
    }

    // For each source node, DFS to find all transitively reachable nodes
    let mut closure = seed.clone();
    for (&start, direct) in &successors {
        let mut stack = direct.clone();
        let mut visited: BTreeSet<TermId> = stack.iter().copied().collect();
        while let Some(node) = stack.pop() {
            closure.insert((start, node));
            if let Some(next) = successors.get(&node) {
                for &s in next {
                    if visited.insert(s) {
                        stack.push(s);
                    }
                }
            }
        }
    }

    let inferred = closure.len().saturating_sub(seed.len());
    let iterations = if seed.is_empty() { 0 } else { 1 };
    (closure, iterations, inferred)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::transitive_closure;

    #[test]
    fn computes_transitive_closure() {
        let seed = BTreeSet::from([(1, 2), (2, 3), (3, 4)]);
        let (closure, _, inferred) = transitive_closure(&seed);
        assert!(closure.contains(&(1, 4)));
        assert!(closure.contains(&(1, 3)));
        assert_eq!(inferred, 3);
    }
}
