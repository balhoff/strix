use rayon::slice::ParallelSliceMut;

use crate::compile::CompiledSchema;
use crate::error::Result;
use crate::store::FactStore;
use crate::store::delta::{difference_binary, difference_ternary};

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

pub fn materialize(
    store: &mut FactStore,
    schema: &CompiledSchema,
    max_iterations: Option<usize>,
) -> Result<ReasoningStats> {
    let mut stats = ReasoningStats::default();

    // Seed deltas from asserted facts
    let mut delta_types = store.asserted_types()?;
    let mut delta_properties = store.asserted_properties()?;

    while !delta_types.is_empty() || !delta_properties.is_empty() {
        if let Some(limit) = max_iterations
            && stats.iterations >= limit
        {
            anyhow::bail!("maximum iterations ({limit}) reached before fixpoint");
        }

        stats.iterations += 1;
        let mut candidate_types: Vec<(u64, u64)> = Vec::new();
        let mut candidate_properties: Vec<(u64, u64, u64)> = Vec::new();

        // Subclass propagation: for delta type(x, a), emit type(x, b) for all b in superclasses(a)
        for &(instance, class) in &delta_types {
            for &superclass in schema.superclasses_for(class) {
                candidate_types.push((instance, superclass));
            }
        }

        // Subproperty propagation: for delta prop(s, p, o), emit prop(s, q, o) for all q in superprops(p)
        for &(subject, predicate, object) in &delta_properties {
            for &superproperty in schema.superproperties_for(predicate) {
                candidate_properties.push((subject, superproperty, object));
            }

            // Domain inference: for delta prop(s, p, o), emit type(s, c) for all c in domains(p)
            for &domain in schema.domains_for(predicate) {
                candidate_types.push((subject, domain));
            }

            // Range inference: for delta prop(s, p, o), emit type(o, c) for all c in ranges(p)
            for &range in schema.ranges_for(predicate) {
                candidate_types.push((object, range));
            }
        }

        // Sort, dedup candidates
        candidate_types.par_sort_unstable();
        candidate_types.dedup();
        candidate_properties.par_sort_unstable();
        candidate_properties.dedup();

        // Difference check against known view
        let known_types = store.known_types()?;
        let known_properties = store.known_properties()?;
        let new_types = difference_binary(&candidate_types, &known_types);
        let new_properties = difference_ternary(&candidate_properties, &known_properties);

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

        delta_types = new_types;
        delta_properties = new_properties;
    }

    stats.fixpoint_reached = true;
    Ok(stats)
}
