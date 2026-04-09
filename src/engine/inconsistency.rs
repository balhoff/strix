use std::collections::{BTreeMap, BTreeSet};

use crate::compile::CompiledSchema;
use crate::dict::TermId;
use crate::error::Result;
use crate::store::FactStore;

/// A detected logical inconsistency in the materialized knowledge base.
#[derive(Clone, Debug)]
pub enum Inconsistency {
    DisjointClasses {
        individual: TermId,
        class_a: TermId,
        class_b: TermId,
    },
    ComplementOf {
        individual: TermId,
        class: TermId,
        complement: TermId,
    },
    DisjointProperties {
        subject: TermId,
        object: TermId,
        prop_a: TermId,
        prop_b: TermId,
    },
    MaxCardinalityZero {
        individual: TermId,
        class: TermId,
        property: TermId,
        object: TermId,
    },
}

/// Check the fully materialized store for logical inconsistencies.
///
/// Runs after fixpoint convergence. Scans known types and properties
/// against disjointness and cardinality constraints from the schema.
pub fn check_inconsistencies(
    store: &mut FactStore,
    schema: &CompiledSchema,
) -> Result<Vec<Inconsistency>> {
    let mut results = Vec::new();

    check_disjoint_types(store, schema, &mut results)?;
    check_disjoint_properties(store, schema, &mut results)?;
    check_max_card_zero(store, schema, &mut results)?;

    Ok(results)
}

/// Check DisjointClasses and ComplementOf constraints.
///
/// For each disjoint pair (A, B), reports any individual typed as both A and B.
fn check_disjoint_types(
    store: &mut FactStore,
    schema: &CompiledSchema,
    results: &mut Vec<Inconsistency>,
) -> Result<()> {
    if schema.disjoint_class_pairs.is_empty() && schema.complement_pairs.is_empty() {
        return Ok(());
    }

    // Collect classes involved in any disjointness/complement axiom.
    let mut relevant_classes: BTreeSet<TermId> = BTreeSet::new();
    for &(a, b) in &schema.disjoint_class_pairs {
        relevant_classes.insert(a);
        relevant_classes.insert(b);
    }
    for &(a, b) in &schema.complement_pairs {
        relevant_classes.insert(a);
        relevant_classes.insert(b);
    }

    // Build class → instances for relevant classes only.
    let mut class_instances: BTreeMap<TermId, BTreeSet<TermId>> = BTreeMap::new();
    for result in store.known_types_iter()? {
        let (inst, cls) = result?;
        if relevant_classes.contains(&cls) {
            class_instances.entry(cls).or_default().insert(inst);
        }
    }

    // Check disjoint class pairs.
    for &(a, b) in &schema.disjoint_class_pairs {
        if let (Some(insts_a), Some(insts_b)) =
            (class_instances.get(&a), class_instances.get(&b))
        {
            for &ind in insts_a {
                if insts_b.contains(&ind) {
                    results.push(Inconsistency::DisjointClasses {
                        individual: ind,
                        class_a: a,
                        class_b: b,
                    });
                }
            }
        }
    }

    // Check complement pairs (semantically equivalent to disjointness).
    for &(cls, comp) in &schema.complement_pairs {
        if let (Some(insts_a), Some(insts_b)) =
            (class_instances.get(&cls), class_instances.get(&comp))
        {
            for &ind in insts_a {
                if insts_b.contains(&ind) {
                    results.push(Inconsistency::ComplementOf {
                        individual: ind,
                        class: cls,
                        complement: comp,
                    });
                }
            }
        }
    }

    Ok(())
}

/// Check DisjointProperties constraints.
///
/// For each disjoint pair (P, Q), reports any (subject, object) that
/// appears in both property(s, P, o) and property(s, Q, o).
fn check_disjoint_properties(
    store: &mut FactStore,
    schema: &CompiledSchema,
    results: &mut Vec<Inconsistency>,
) -> Result<()> {
    if schema.disjoint_property_pairs.is_empty() {
        return Ok(());
    }

    let mut relevant_props: BTreeSet<TermId> = BTreeSet::new();
    for &(a, b) in &schema.disjoint_property_pairs {
        relevant_props.insert(a);
        relevant_props.insert(b);
    }

    // Build prop → set of (subject, object) for relevant properties.
    let mut prop_assertions: BTreeMap<TermId, BTreeSet<(TermId, TermId)>> = BTreeMap::new();
    for result in store.known_properties_iter()? {
        let (s, p, o) = result?;
        if relevant_props.contains(&p) {
            prop_assertions.entry(p).or_default().insert((s, o));
        }
    }

    for &(pa, pb) in &schema.disjoint_property_pairs {
        if let (Some(pairs_a), Some(pairs_b)) =
            (prop_assertions.get(&pa), prop_assertions.get(&pb))
        {
            for &(s, o) in pairs_a {
                if pairs_b.contains(&(s, o)) {
                    results.push(Inconsistency::DisjointProperties {
                        subject: s,
                        object: o,
                        prop_a: pa,
                        prop_b: pb,
                    });
                }
            }
        }
    }

    Ok(())
}

/// Check MaxCardinality 0 constraints.
///
/// SubClassOf(A, MaxCard(0, P, C)) is violated when:
///   type(x, A) ∧ property(x, P, y) [∧ type(y, C)]
fn check_max_card_zero(
    store: &mut FactStore,
    schema: &CompiledSchema,
    results: &mut Vec<Inconsistency>,
) -> Result<()> {
    if schema.max_card_zero.is_empty() {
        return Ok(());
    }

    // Collect classes relevant to max_card_zero: both the restricting class
    // and optional filler class.
    let mut relevant_classes: BTreeSet<TermId> = BTreeSet::new();
    for &(cls, _, filler) in &schema.max_card_zero {
        relevant_classes.insert(cls);
        if let Some(f) = filler {
            relevant_classes.insert(f);
        }
    }

    let mut instance_classes: BTreeMap<TermId, BTreeSet<TermId>> = BTreeMap::new();
    for result in store.known_types_iter()? {
        let (inst, cls) = result?;
        if relevant_classes.contains(&cls) {
            instance_classes.entry(inst).or_default().insert(cls);
        }
    }

    // Build (pred, subject) → [objects] for relevant properties.
    let relevant_preds: BTreeSet<TermId> =
        schema.max_card_zero.iter().map(|&(_, p, _)| p).collect();
    let mut pred_subj_objs: BTreeMap<(TermId, TermId), Vec<TermId>> = BTreeMap::new();
    for result in store.known_properties_iter()? {
        let (s, p, o) = result?;
        if relevant_preds.contains(&p) {
            pred_subj_objs.entry((p, s)).or_default().push(o);
        }
    }

    for &(class, prop, opt_filler) in &schema.max_card_zero {
        for (&inst, classes) in &instance_classes {
            if !classes.contains(&class) {
                continue;
            }
            if let Some(objects) = pred_subj_objs.get(&(prop, inst)) {
                for &obj in objects {
                    if let Some(filler) = opt_filler {
                        let obj_has_filler = instance_classes
                            .get(&obj)
                            .is_some_and(|c| c.contains(&filler));
                        if !obj_has_filler {
                            continue;
                        }
                    }
                    results.push(Inconsistency::MaxCardinalityZero {
                        individual: inst,
                        class,
                        property: prop,
                        object: obj,
                    });
                }
            }
        }
    }

    Ok(())
}
