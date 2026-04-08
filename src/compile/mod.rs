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

    /// Predicates that require in-memory indexing for join evaluation.
    /// Currently: transitive properties. Will grow in Steps 4-5.
    pub indexed_predicates: BTreeSet<TermId>,

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

pub fn compile_schema(schema: &RawSchema) -> CompiledSchema {
    let (subclass_closure, subclass_iterations, subclass_inferred) =
        transitive_closure(&schema.subclasses);
    let (subproperty_closure, subproperty_iterations, subproperty_inferred) =
        transitive_closure(&schema.subproperties);

    // Predicates needing in-memory indexing for join-based rules.
    // Currently: transitive properties. Steps 4-5 will add more.
    let indexed_predicates = schema.transitive_properties.clone();

    CompiledSchema {
        superclasses: to_map(&subclass_closure),
        superproperties: to_map(&subproperty_closure),
        domains: to_map(&schema.domains),
        ranges: to_map(&schema.ranges),
        inverses: to_map(&schema.inverse_properties),
        symmetric_properties: schema.symmetric_properties.clone(),
        transitive_properties: schema.transitive_properties.clone(),
        indexed_predicates,
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
