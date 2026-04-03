pub mod ir;

use std::collections::{BTreeMap, BTreeSet};

use crate::dict::TermId;
use crate::owl::RawSchema;

#[derive(Clone, Debug, Default)]
pub struct CompiledSchema {
    pub superclasses: BTreeMap<TermId, Vec<TermId>>,
    pub superproperties: BTreeMap<TermId, Vec<TermId>>,
    pub domains: BTreeMap<TermId, Vec<TermId>>,
    pub ranges: BTreeMap<TermId, Vec<TermId>>,
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
}

pub fn compile_schema(schema: &RawSchema) -> CompiledSchema {
    let (subclass_closure, subclass_iterations, subclass_inferred) =
        transitive_closure(&schema.subclasses);
    let (subproperty_closure, subproperty_iterations, subproperty_inferred) =
        transitive_closure(&schema.subproperties);

    CompiledSchema {
        superclasses: to_map(&subclass_closure),
        superproperties: to_map(&subproperty_closure),
        domains: to_map(&schema.domains),
        ranges: to_map(&schema.ranges),
        schema_iterations: subclass_iterations.max(subproperty_iterations),
        schema_inferred: subclass_inferred + subproperty_inferred,
        rule_set: ir::RuleSet::phase_one(),
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
    let mut closure = seed.clone();
    let mut iterations = 0usize;

    loop {
        let snapshot = closure.iter().copied().collect::<Vec<_>>();
        let mut additions = Vec::new();

        for (left, middle) in &snapshot {
            for (candidate_middle, right) in &snapshot {
                if middle == candidate_middle && !closure.contains(&(*left, *right)) {
                    additions.push((*left, *right));
                }
            }
        }

        if additions.is_empty() {
            if !closure.is_empty() && iterations == 0 {
                iterations = 1;
            }
            break;
        }

        iterations += 1;
        for pair in additions {
            closure.insert(pair);
        }
    }

    let inferred = closure.len().saturating_sub(seed.len());
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
