pub mod ir;

use std::collections::{BTreeMap, BTreeSet};

use crate::dict::TermId;
use crate::owl::{RawSchema, RawSwrlArg, RawSwrlAtom};

use ir::{CompiledSwrlRule, SwrlArg, SwrlBodyAtom, SwrlHeadAtom};

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
    /// All intersection rules: (super_class, [conjuncts]).
    /// Multiple rules may share the same super_class.
    pub intersection_rules: Vec<(TermId, Vec<TermId>)>,
    /// super_class → [rule indices] — cls-int2: type(x,C) → type(x,Ci)
    pub intersection_by_class: BTreeMap<TermId, Vec<usize>>,
    /// conjunct → [rule indices] — cls-int1: check all conjuncts → type(x,C)
    pub conjunct_of: BTreeMap<TermId, Vec<usize>>,

    // Equality-producing rules (Phase 2, Step 6)
    pub functional_properties: BTreeSet<TermId>,
    pub inverse_functional_properties: BTreeSet<TermId>,
    /// (class, prop, opt_filler) — SubClassOf(A, MaxCard(1,P,C))
    pub max_card_one: Vec<(TermId, TermId, Option<TermId>)>,
    /// prop → [(class, opt_filler)] — max_card_one indexed by predicate
    pub max_card_one_by_prop: BTreeMap<TermId, Vec<(TermId, Option<TermId>)>>,

    // Inconsistency detection (Phase 2, Step 7)
    /// Pairwise disjoint class pairs — flattened from DisjointClasses axioms.
    pub disjoint_class_pairs: Vec<(TermId, TermId)>,
    /// (class, complement) — from SubClassOf(A, ComplementOf(D)).
    pub complement_pairs: Vec<(TermId, TermId)>,
    /// Pairwise disjoint property pairs — flattened from DisjointProperties axioms.
    pub disjoint_property_pairs: Vec<(TermId, TermId)>,
    /// (class, prop, opt_filler) — SubClassOf(A, MaxCard(0,P,C))
    pub max_card_zero: Vec<(TermId, TermId, Option<TermId>)>,
    pub irreflexive_properties: BTreeSet<TermId>,
    pub asymmetric_properties: BTreeSet<TermId>,

    // Equality-producing axioms (Phase 3)
    /// (class, [key_properties]) — HasKey(C, [P1,...,Pn])
    pub has_key: Vec<(TermId, Vec<TermId>)>,
    /// All predicates appearing in any HasKey axiom (for fast scan filtering).
    pub has_key_preds: BTreeSet<TermId>,

    // Individual axioms (Phase 3)
    pub same_individual_pairs: Vec<(TermId, TermId)>,
    pub different_individual_pairs: Vec<(TermId, TermId)>,

    // Negative assertions (Phase 3)
    /// (property, subject, object) — combined from NegativeOPA + NegativeDPA
    pub negative_property_assertions: Vec<(TermId, TermId, TermId)>,

    // SWRL rules (Phase 3)
    pub swrl_rules: Vec<CompiledSwrlRule>,
    /// class → [rule indices] — rules triggered by a ClassAtom matching this class
    pub swrl_by_type_trigger: BTreeMap<TermId, Vec<usize>>,
    /// predicate → [rule indices] — rules triggered by a PropertyAtom matching this predicate
    pub swrl_by_prop_trigger: BTreeMap<TermId, Vec<usize>>,
    /// rule indices whose trigger is SameIndividualAtom or DifferentIndividualsAtom
    pub swrl_equality_triggered: Vec<usize>,

    /// Predicates that require in-memory indexing for join evaluation.
    pub indexed_predicates: BTreeSet<TermId>,
    /// Classes that require in-memory indexing for join evaluation.
    pub indexed_classes: BTreeSet<TermId>,

    pub schema_iterations: usize,
    pub schema_inferred: usize,

    /// Proxy TermIds created for anonymous class/property expressions.
    /// These must be filtered from output triples.
    pub proxy_terms: BTreeSet<TermId>,
    /// Human-readable OFN display string for each proxy TermId.
    pub proxy_display: BTreeMap<TermId, String>,
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

    pub fn compiled_schema_report(&self) -> crate::output::report::CompiledSchemaReport {
        crate::output::report::CompiledSchemaReport {
            subclass_closure_size: self.superclasses.values().map(|v| v.len()).sum(),
            subproperty_closure_size: self.superproperties.values().map(|v| v.len()).sum(),
            domain_entries: self.domains.values().map(|v| v.len()).sum(),
            range_entries: self.ranges.values().map(|v| v.len()).sum(),
            inverse_entries: self.inverses.values().map(|v| v.len()).sum(),
            property_chains: self.property_chains.len(),
            chain_trigger_entries: self.chain_triggers.values().map(|v| v.len()).sum(),
            universal_types: self.universal_types.len(),
            has_value_rules: self.has_value_by_prop.values().map(|v| v.len()).sum::<usize>()
                + self.has_value_by_class.values().map(|v| v.len()).sum::<usize>(),
            some_values_from_rules: self
                .some_values_from_by_prop
                .values()
                .map(|v| v.len())
                .sum::<usize>()
                + self.svf_thing_by_prop.values().map(|v| v.len()).sum::<usize>(),
            all_values_from_rules: self
                .all_values_from_by_class
                .values()
                .map(|v| v.len())
                .sum(),
            intersection_rules: self.intersection_rules.len(),
            max_cardinality_one_rules: self.max_card_one.len(),
            swrl_rules_compiled: self.swrl_rules.len(),
            indexed_predicates: self.indexed_predicates.len(),
            indexed_classes: self.indexed_classes.len(),
            proxy_terms: self.proxy_terms.len(),
            schema_closure_iterations: self.schema_iterations,
            schema_closure_inferred: self.schema_inferred,
        }
    }

    /// Returns the list of rule categories that are active for this ontology
    /// (i.e., have non-empty compiled lookup tables).
    pub fn active_rules(&self) -> Vec<String> {
        let mut active = Vec::new();
        if !self.superclasses.is_empty() {
            active.push("subclass-propagation".to_string());
        }
        if !self.superproperties.is_empty() {
            active.push("subproperty-propagation".to_string());
        }
        if !self.domains.is_empty() {
            active.push("domain-inference".to_string());
        }
        if !self.ranges.is_empty() {
            active.push("range-inference".to_string());
        }
        if !self.inverses.is_empty() {
            active.push("inverse-property".to_string());
        }
        if !self.symmetric_properties.is_empty() {
            active.push("symmetric-property".to_string());
        }
        if !self.transitive_properties.is_empty() {
            active.push("transitive-property".to_string());
        }
        if !self.property_chains.is_empty() {
            active.push("property-chain".to_string());
        }
        if !self.universal_types.is_empty() {
            active.push("universal-type".to_string());
        }
        if !self.has_value_by_prop.is_empty() || !self.has_value_by_class.is_empty() {
            active.push("has-value".to_string());
        }
        if !self.some_values_from_by_prop.is_empty() || !self.svf_thing_by_prop.is_empty() {
            active.push("some-values-from".to_string());
        }
        if !self.all_values_from_by_class.is_empty() {
            active.push("all-values-from".to_string());
        }
        if !self.intersection_rules.is_empty() {
            active.push("intersection-of".to_string());
        }
        if !self.functional_properties.is_empty() {
            active.push("functional-property".to_string());
        }
        if !self.inverse_functional_properties.is_empty() {
            active.push("inverse-functional-property".to_string());
        }
        if !self.max_card_one.is_empty() {
            active.push("max-cardinality-one".to_string());
        }
        if !self.has_key.is_empty() {
            active.push("has-key".to_string());
        }
        if !self.swrl_rules.is_empty() {
            active.push("swrl".to_string());
        }
        active
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
///
/// `rdfs_literal` is the TermId for `rdfs:Literal` (the top data range).
/// It is treated like `owl:Thing` for data property restriction fillers.
pub fn compile_schema(schema: &RawSchema, owl_thing: TermId, rdfs_literal: TermId) -> CompiledSchema {
    let is_top = |id: TermId| id == owl_thing || id == rdfs_literal;
    let (mut subclass_closure, subclass_iterations, subclass_inferred) =
        transitive_closure(&schema.subclasses);
    let (subproperty_closure, subproperty_iterations, subproperty_inferred) =
        transitive_closure(&schema.subproperties);

    // Extract universal types: SubClassOf(owl:Thing, C) means every individual
    // is of type C. Collect before filtering so we don't lose them.
    let universal_types: Vec<TermId> = subclass_closure
        .iter()
        .filter(|&&(sub, sup)| sub == owl_thing && !is_top(sup))
        .map(|&(_, sup)| sup)
        .collect();

    // Strip owl:Thing / rdfs:Literal as both superclass target (trivially true
    // for every individual) and as subclass source (now handled by universal_types).
    subclass_closure.retain(|&(sub, sup)| !is_top(sup) && !is_top(sub));

    let mut has_value_by_prop: BTreeMap<TermId, Vec<(TermId, TermId)>> = BTreeMap::new();
    let mut has_value_by_class: BTreeMap<TermId, Vec<(TermId, TermId)>> = BTreeMap::new();
    for &(class, prop, val) in &schema.has_value_super {
        has_value_by_prop
            .entry(prop)
            .or_default()
            .push((class, val));
    }
    for &(class, prop, val) in &schema.has_value_sub {
        has_value_by_class
            .entry(class)
            .or_default()
            .push((prop, val));
    }

    // someValuesFrom: separate owl:Thing filler into property-existence rules
    let mut some_values_from_by_prop: BTreeMap<TermId, Vec<(TermId, TermId)>> = BTreeMap::new();
    let mut some_values_from_by_filler: BTreeMap<TermId, Vec<(TermId, TermId)>> = BTreeMap::new();
    let mut svf_thing_by_prop: BTreeMap<TermId, Vec<TermId>> = BTreeMap::new();
    for &(class, prop, filler) in &schema.some_values_from {
        if is_top(filler) {
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
        if is_top(filler) {
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
    let mut intersection_rules: Vec<(TermId, Vec<TermId>)> = Vec::new();
    let mut intersection_by_class: BTreeMap<TermId, Vec<usize>> = BTreeMap::new();
    let mut conjunct_of: BTreeMap<TermId, Vec<usize>> = BTreeMap::new();
    for (class, conjuncts) in &schema.intersection_of {
        let filtered: Vec<TermId> = conjuncts
            .iter()
            .copied()
            .filter(|&c| !is_top(c))
            .collect();
        if filtered.is_empty() {
            continue;
        }
        let idx = intersection_rules.len();
        intersection_by_class.entry(*class).or_default().push(idx);
        for &conjunct in &filtered {
            conjunct_of.entry(conjunct).or_default().push(idx);
        }
        intersection_rules.push((*class, filtered));
    }

    // Domain/range: filter out owl:Thing / rdfs:Literal targets (trivially true)
    let filtered_domains: BTreeSet<(TermId, TermId)> = schema
        .domains
        .iter()
        .copied()
        .filter(|&(_, cls)| !is_top(cls))
        .collect();
    let filtered_ranges: BTreeSet<(TermId, TermId)> = schema
        .ranges
        .iter()
        .copied()
        .filter(|&(_, cls)| !is_top(cls))
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

    // MaxCardinality 1: normalize owl:Thing / rdfs:Literal filler to None (unqualified).
    let max_card_one: Vec<(TermId, TermId, Option<TermId>)> = schema
        .max_card_one
        .iter()
        .map(|&(cls, prop, filler)| (cls, prop, filler.filter(|&f| !is_top(f))))
        .collect();
    let mut max_card_one_by_prop: BTreeMap<TermId, Vec<(TermId, Option<TermId>)>> = BTreeMap::new();
    for &(cls, prop, filler) in &max_card_one {
        max_card_one_by_prop
            .entry(prop)
            .or_default()
            .push((cls, filler));
    }

    let disjoint_class_pairs = flatten_pairwise(&schema.disjoint_classes);
    let complement_pairs: Vec<(TermId, TermId)> = schema.complement_of.iter().copied().collect();
    let disjoint_property_pairs = flatten_pairwise(&schema.disjoint_properties);

    // MaxCardinality 0: normalize owl:Thing / rdfs:Literal filler to None.
    let max_card_zero: Vec<(TermId, TermId, Option<TermId>)> = schema
        .max_card_zero
        .iter()
        .map(|&(cls, prop, filler)| (cls, prop, filler.filter(|&f| !is_top(f))))
        .collect();

    // Compile SWRL rules
    let swrl = compile_swrl_rules(schema);

    // Predicates needing in-memory indexing for join-based rules.
    // Built from filtered lookup tables so owl:Thing-only entries are excluded.
    let mut indexed_predicates = transitive_properties.clone();
    for (_, chain) in &property_chains {
        indexed_predicates.extend(chain);
    }
    indexed_predicates.extend(some_values_from_by_prop.keys());
    indexed_predicates.extend(all_values_from_by_prop.keys());
    for (_, key_props) in &schema.has_key {
        indexed_predicates.extend(key_props);
    }
    // Classes needing in-memory indexing for join-based rules.
    let mut indexed_classes: BTreeSet<TermId> = BTreeSet::new();
    indexed_classes.extend(some_values_from_by_filler.keys());
    indexed_classes.extend(schema.has_key.iter().map(|(cls, _)| cls));
    indexed_classes.extend(all_values_from_by_class.keys());
    indexed_classes.extend(intersection_rules.iter().flat_map(|(_, cs)| cs.iter()));

    // Register SWRL-referenced predicates and classes for indexing
    for rule in &swrl.rules {
        for atom in &rule.body {
            match atom {
                SwrlBodyAtom::PropertyAtom { property, .. } => {
                    indexed_predicates.insert(*property);
                }
                SwrlBodyAtom::ClassAtom { class, .. } => {
                    indexed_classes.insert(*class);
                }
                SwrlBodyAtom::SameIndividualAtom { .. }
                | SwrlBodyAtom::DifferentIndividualsAtom { .. } => {}
            }
        }
    }

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
        intersection_rules,
        intersection_by_class,
        conjunct_of,
        functional_properties: schema.functional_properties.clone(),
        inverse_functional_properties: schema.inverse_functional_properties.clone(),
        max_card_one,
        max_card_one_by_prop,
        disjoint_class_pairs,
        complement_pairs,
        disjoint_property_pairs,
        max_card_zero,
        irreflexive_properties: schema.irreflexive_properties.clone(),
        asymmetric_properties: schema.asymmetric_properties.clone(),
        has_key_preds: schema
            .has_key
            .iter()
            .flat_map(|(_, ps)| ps.iter().copied())
            .collect(),
        has_key: schema.has_key.clone(),
        same_individual_pairs: flatten_pairwise(&schema.same_individuals),
        different_individual_pairs: flatten_pairwise(&schema.different_individuals),
        negative_property_assertions: schema
            .negative_object_property_assertions
            .iter()
            .chain(schema.negative_data_property_assertions.iter())
            .copied()
            .collect(),
        swrl_rules: swrl.rules,
        swrl_by_type_trigger: swrl.by_type_trigger,
        swrl_by_prop_trigger: swrl.by_prop_trigger,
        swrl_equality_triggered: swrl.equality_triggered,
        indexed_predicates,
        indexed_classes,
        schema_iterations: subclass_iterations.max(subproperty_iterations),
        schema_inferred: subclass_inferred + subproperty_inferred,
        proxy_terms: schema.proxy_terms.clone(),
        proxy_display: schema.proxy_display.clone(),
    }
}

struct CompiledSwrlRules {
    rules: Vec<CompiledSwrlRule>,
    by_type_trigger: BTreeMap<TermId, Vec<usize>>,
    by_prop_trigger: BTreeMap<TermId, Vec<usize>>,
    equality_triggered: Vec<usize>,
}

/// Compile raw SWRL rules into evaluated form, returning rules and trigger indexes.
fn compile_swrl_rules(schema: &RawSchema) -> CompiledSwrlRules {
    let mut rules = Vec::new();
    let mut by_type: BTreeMap<TermId, Vec<usize>> = BTreeMap::new();
    let mut by_prop: BTreeMap<TermId, Vec<usize>> = BTreeMap::new();
    let mut equality_triggered: Vec<usize> = Vec::new();

    for raw in &schema.swrl_rules {
        let mut var_map: BTreeMap<TermId, u32> = BTreeMap::new();
        let mut next_var: u32 = 0;
        let mut convert_arg = |arg: &RawSwrlArg| -> SwrlArg {
            match arg {
                RawSwrlArg::Variable(id) => {
                    let v = *var_map.entry(*id).or_insert_with(|| {
                        let v = next_var;
                        next_var += 1;
                        v
                    });
                    SwrlArg::Variable(v)
                }
                RawSwrlArg::Constant(id) => SwrlArg::Constant(*id),
            }
        };

        let body: Vec<SwrlBodyAtom> = raw
            .body
            .iter()
            .map(|atom| match atom {
                RawSwrlAtom::ClassAtom { class, arg } => SwrlBodyAtom::ClassAtom {
                    class: *class,
                    arg: convert_arg(arg),
                },
                RawSwrlAtom::PropertyAtom {
                    property,
                    subject,
                    object,
                } => SwrlBodyAtom::PropertyAtom {
                    property: *property,
                    subject: convert_arg(subject),
                    object: convert_arg(object),
                },
                RawSwrlAtom::SameIndividualAtom { left, right } => {
                    SwrlBodyAtom::SameIndividualAtom {
                        left: convert_arg(left),
                        right: convert_arg(right),
                    }
                }
                RawSwrlAtom::DifferentIndividualsAtom { left, right } => {
                    SwrlBodyAtom::DifferentIndividualsAtom {
                        left: convert_arg(left),
                        right: convert_arg(right),
                    }
                }
            })
            .collect();

        let heads: Vec<SwrlHeadAtom> = raw
            .head
            .iter()
            .map(|atom| match atom {
                RawSwrlAtom::ClassAtom { class, arg } => SwrlHeadAtom::ClassAtom {
                    class: *class,
                    arg: convert_arg(arg),
                },
                RawSwrlAtom::PropertyAtom {
                    property,
                    subject,
                    object,
                } => SwrlHeadAtom::PropertyAtom {
                    property: *property,
                    subject: convert_arg(subject),
                    object: convert_arg(object),
                },
                RawSwrlAtom::SameIndividualAtom { left, right } => {
                    SwrlHeadAtom::SameIndividualAtom {
                        left: convert_arg(left),
                        right: convert_arg(right),
                    }
                }
                RawSwrlAtom::DifferentIndividualsAtom { left, right } => {
                    SwrlHeadAtom::DifferentIndividualsAtom {
                        left: convert_arg(left),
                        right: convert_arg(right),
                    }
                }
            })
            .collect();

        let num_vars = next_var;
        let trigger = select_trigger(&body);
        let remaining: Vec<usize> = (0..body.len()).filter(|&i| i != trigger).collect();

        // Multi-head rules expand into one compiled rule per head atom.
        for head in heads {
            let idx = rules.len();
            match &body[trigger] {
                SwrlBodyAtom::ClassAtom { class, .. } => {
                    by_type.entry(*class).or_default().push(idx);
                }
                SwrlBodyAtom::PropertyAtom { property, .. } => {
                    by_prop.entry(*property).or_default().push(idx);
                }
                SwrlBodyAtom::SameIndividualAtom { .. }
                | SwrlBodyAtom::DifferentIndividualsAtom { .. } => {
                    equality_triggered.push(idx);
                }
            }
            rules.push(CompiledSwrlRule {
                trigger,
                remaining: remaining.clone(),
                body: body.clone(),
                head,
                num_vars,
            });
        }
    }

    CompiledSwrlRules {
        rules,
        by_type_trigger: by_type,
        by_prop_trigger: by_prop,
        equality_triggered,
    }
}

/// Select the best trigger atom for a SWRL rule body.
///
/// Heuristic: pick the atom that will be most selective when scanned from
/// deltas. Property atoms bind 2 variables per match (subject + object)
/// vs 1 for class atoms, so we prefer them when tied on "new variables bound".
fn select_trigger(body: &[SwrlBodyAtom]) -> usize {
    let mut bound: u64 = 0;
    let mut best = 0;
    let mut best_score: (bool, usize, bool) = (false, 0, false);

    for (i, atom) in body.iter().enumerate() {
        let var_mask = atom_var_mask(atom);
        let new = (var_mask & !bound).count_ones() as usize;
        let is_prop = matches!(atom, SwrlBodyAtom::PropertyAtom { .. });
        let is_dispatchable = !matches!(
            atom,
            SwrlBodyAtom::SameIndividualAtom { .. } | SwrlBodyAtom::DifferentIndividualsAtom { .. }
        );
        let score = (is_dispatchable, new, is_prop);
        if score > best_score {
            best_score = score;
            best = i;
        }
        bound |= var_mask;
    }
    best
}

/// Returns a bitmask of variable IDs referenced by the atom.
fn atom_var_mask(atom: &SwrlBodyAtom) -> u64 {
    match atom {
        SwrlBodyAtom::ClassAtom { arg, .. } => arg.as_variable().map_or(0, |v| 1 << v),
        SwrlBodyAtom::PropertyAtom {
            subject, object, ..
        } => {
            subject.as_variable().map_or(0, |v| 1 << v) | object.as_variable().map_or(0, |v| 1 << v)
        }
        SwrlBodyAtom::SameIndividualAtom { left, right }
        | SwrlBodyAtom::DifferentIndividualsAtom { left, right } => {
            left.as_variable().map_or(0, |v| 1 << v) | right.as_variable().map_or(0, |v| 1 << v)
        }
    }
}

/// Flatten n-ary disjointness groups into all pairwise (a, b) pairs.
fn flatten_pairwise(groups: &[Vec<TermId>]) -> Vec<(TermId, TermId)> {
    let mut pairs = Vec::new();
    for group in groups {
        for i in 0..group.len() {
            for j in (i + 1)..group.len() {
                pairs.push((group[i], group[j]));
            }
        }
    }
    pairs
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
