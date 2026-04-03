use crate::compile::CompiledSchema;
use crate::error::{AppError, Result};
use crate::store::FactStore;

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
    let mut delta_types = store.asserted_types().collect::<Vec<_>>();
    let mut delta_properties = store.asserted_properties().collect::<Vec<_>>();

    while !delta_types.is_empty() || !delta_properties.is_empty() {
        if let Some(limit) = max_iterations {
            if stats.iterations >= limit {
                return Err(AppError::new(format!(
                    "maximum iterations ({limit}) reached before fixpoint"
                )));
            }
        }

        stats.iterations += 1;
        let mut next_types = Vec::new();
        let mut next_properties = Vec::new();

        for (instance, class) in delta_types.drain(..) {
            for superclass in schema.superclasses_for(class) {
                if store.insert_derived_type(instance, *superclass) {
                    next_types.push((instance, *superclass));
                    stats.inferred_types += 1;
                }
            }
        }

        for (subject, predicate, object) in delta_properties.drain(..) {
            for superproperty in schema.superproperties_for(predicate) {
                if store.insert_derived_property(subject, *superproperty, object) {
                    next_properties.push((subject, *superproperty, object));
                    stats.inferred_properties += 1;
                }
            }

            for domain in schema.domains_for(predicate) {
                if store.insert_derived_type(subject, *domain) {
                    next_types.push((subject, *domain));
                    stats.inferred_types += 1;
                }
            }

            for range in schema.ranges_for(predicate) {
                if store.insert_derived_type(object, *range) {
                    next_types.push((object, *range));
                    stats.inferred_types += 1;
                }
            }
        }

        delta_types = next_types;
        delta_properties = next_properties;
    }

    stats.fixpoint_reached = true;
    Ok(stats)
}
