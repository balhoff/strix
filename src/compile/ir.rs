use crate::dict::TermId;

pub type RuleId = &'static str;
pub type StratumId = u32;
pub type VarId = u32;

/// Rule family — distinguishes the origin of a rule for reporting.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleFamily {
    /// Rules derived from OWL 2 RL (including the RDFS subset).
    OwlRl,
    /// Rules from SWRL (Phase 3).
    Swrl,
}

/// A typed rule in the internal representation.
///
/// Phase 1 rules are schema-parameterized: the body references a delta
/// relation and the head emits facts by iterating over closure lookup
/// tables at evaluation time. Phase 2+ rules may have concrete constants
/// in their bindings.
#[derive(Clone, Debug)]
pub struct Rule {
    pub id: RuleId,
    pub family: RuleFamily,
    pub stratum: StratumId,
    pub body: Vec<BodyAtom>,
    pub head: HeadAtom,
}

/// Which predicate-partitioned relation a body or head atom references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelationId {
    TypeAssertion,
    PropertyAssertion,
}

/// A binding position in a body or head atom.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Binding {
    Variable(VarId),
    Constant(TermId),
    Wildcard,
}

/// A single atom in the rule body (a relation + binding pattern).
#[derive(Clone, Debug)]
pub struct BodyAtom {
    pub relation: RelationId,
    pub bindings: Vec<Binding>,
}

/// The rule head (a relation + binding pattern).
#[derive(Clone, Debug)]
pub struct HeadAtom {
    pub relation: RelationId,
    pub bindings: Vec<Binding>,
}

/// How a rule should be evaluated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvalStrategy {
    /// A schema-parameterized rule: the engine uses closure lookup tables
    /// at evaluation time (Phase 1 RDFS patterns).
    SchemaParameterized(SchemaPattern),
    /// A concrete specialized rule with constants in body/head bindings
    /// (Phase 2+ OWL RL restrictions, Phase 3 SWRL).
    Concrete,
}

/// Which schema-parameterized pattern this rule implements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaPattern {
    /// type(?x, ?a) -> type(?x, ?b) for ?b in superclasses(?a)
    SubclassPropagation,
    /// property(?s, ?p, ?o) -> property(?s, ?q, ?o) for ?q in superproperties(?p)
    SubpropertyPropagation,
    /// property(?s, ?p, ?o) -> type(?s, ?c) for ?c in domains(?p)
    DomainInference,
    /// property(?s, ?p, ?o) -> type(?o, ?c) for ?c in ranges(?p)
    RangeInference,
    /// property(?s, ?p, ?o) -> property(?o, ?q, ?s) for ?q in inverses(?p)
    InverseProperty,
    /// property(?s, ?p, ?o) -> property(?o, ?p, ?s) when symmetric(?p)
    SymmetricProperty,
    /// property(?x, ?p, ?y) ∧ property(?y, ?p, ?z) -> property(?x, ?p, ?z) when transitive(?p)
    TransitiveProperty,
}

// ── Compiled SWRL types ──────────────────────────────────────────────

/// A compiled SWRL rule ready for evaluation.
#[derive(Clone, Debug)]
pub struct CompiledSwrlRule {
    /// Index of the trigger atom in `body`.
    pub trigger: usize,
    /// Body atom indices excluding the trigger, pre-computed for evaluation.
    pub remaining: Vec<usize>,
    pub body: Vec<SwrlBodyAtom>,
    pub head: SwrlHeadAtom,
    /// Total number of distinct variables across this rule.
    pub num_vars: u32,
}

#[derive(Clone, Debug)]
pub enum SwrlBodyAtom {
    ClassAtom { class: TermId, arg: SwrlArg },
    PropertyAtom { property: TermId, subject: SwrlArg, object: SwrlArg },
    SameIndividualAtom { left: SwrlArg, right: SwrlArg },
    DifferentIndividualsAtom { left: SwrlArg, right: SwrlArg },
}

#[derive(Clone, Debug)]
pub enum SwrlHeadAtom {
    ClassAtom { class: TermId, arg: SwrlArg },
    PropertyAtom { property: TermId, subject: SwrlArg, object: SwrlArg },
    SameIndividualAtom { left: SwrlArg, right: SwrlArg },
    DifferentIndividualsAtom { left: SwrlArg, right: SwrlArg },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SwrlArg {
    Variable(u32),
    Constant(TermId),
}

impl SwrlArg {
    pub fn as_variable(self) -> Option<u32> {
        match self {
            Self::Variable(v) => Some(v),
            Self::Constant(_) => None,
        }
    }
}

/// A compiled set of rules, grouped by stratum.
#[derive(Clone, Debug)]
pub struct RuleSet {
    pub rules: Vec<(Rule, EvalStrategy)>,
}

impl RuleSet {
    /// Construct the rule set for all currently implemented rules.
    pub fn build() -> Self {
        Self {
            rules: vec![
                (
                    Rule {
                        id: "rdfs-subclass",
                        family: RuleFamily::OwlRl,
                        stratum: 1,
                        body: vec![BodyAtom {
                            relation: RelationId::TypeAssertion,
                            bindings: vec![Binding::Variable(0), Binding::Variable(1)],
                        }],
                        head: HeadAtom {
                            relation: RelationId::TypeAssertion,
                            bindings: vec![Binding::Variable(0), Binding::Variable(2)],
                        },
                    },
                    EvalStrategy::SchemaParameterized(SchemaPattern::SubclassPropagation),
                ),
                (
                    Rule {
                        id: "rdfs-subproperty",
                        family: RuleFamily::OwlRl,
                        stratum: 1,
                        body: vec![BodyAtom {
                            relation: RelationId::PropertyAssertion,
                            bindings: vec![
                                Binding::Variable(0),
                                Binding::Variable(1),
                                Binding::Variable(2),
                            ],
                        }],
                        head: HeadAtom {
                            relation: RelationId::PropertyAssertion,
                            bindings: vec![
                                Binding::Variable(0),
                                Binding::Variable(3),
                                Binding::Variable(2),
                            ],
                        },
                    },
                    EvalStrategy::SchemaParameterized(SchemaPattern::SubpropertyPropagation),
                ),
                (
                    Rule {
                        id: "rdfs-domain",
                        family: RuleFamily::OwlRl,
                        stratum: 1,
                        body: vec![BodyAtom {
                            relation: RelationId::PropertyAssertion,
                            bindings: vec![
                                Binding::Variable(0),
                                Binding::Variable(1),
                                Binding::Variable(2),
                            ],
                        }],
                        head: HeadAtom {
                            relation: RelationId::TypeAssertion,
                            bindings: vec![Binding::Variable(0), Binding::Variable(3)],
                        },
                    },
                    EvalStrategy::SchemaParameterized(SchemaPattern::DomainInference),
                ),
                (
                    Rule {
                        id: "rdfs-range",
                        family: RuleFamily::OwlRl,
                        stratum: 1,
                        body: vec![BodyAtom {
                            relation: RelationId::PropertyAssertion,
                            bindings: vec![
                                Binding::Variable(0),
                                Binding::Variable(1),
                                Binding::Variable(2),
                            ],
                        }],
                        head: HeadAtom {
                            relation: RelationId::TypeAssertion,
                            bindings: vec![Binding::Variable(2), Binding::Variable(3)],
                        },
                    },
                    EvalStrategy::SchemaParameterized(SchemaPattern::RangeInference),
                ),
                (
                    Rule {
                        id: "owl-inverse",
                        family: RuleFamily::OwlRl,
                        stratum: 1,
                        body: vec![BodyAtom {
                            relation: RelationId::PropertyAssertion,
                            bindings: vec![
                                Binding::Variable(0),
                                Binding::Variable(1),
                                Binding::Variable(2),
                            ],
                        }],
                        head: HeadAtom {
                            relation: RelationId::PropertyAssertion,
                            bindings: vec![
                                Binding::Variable(2),
                                Binding::Variable(3),
                                Binding::Variable(0),
                            ],
                        },
                    },
                    EvalStrategy::SchemaParameterized(SchemaPattern::InverseProperty),
                ),
                (
                    Rule {
                        id: "owl-symmetric",
                        family: RuleFamily::OwlRl,
                        stratum: 1,
                        body: vec![BodyAtom {
                            relation: RelationId::PropertyAssertion,
                            bindings: vec![
                                Binding::Variable(0),
                                Binding::Variable(1),
                                Binding::Variable(2),
                            ],
                        }],
                        head: HeadAtom {
                            relation: RelationId::PropertyAssertion,
                            bindings: vec![
                                Binding::Variable(2),
                                Binding::Variable(1),
                                Binding::Variable(0),
                            ],
                        },
                    },
                    EvalStrategy::SchemaParameterized(SchemaPattern::SymmetricProperty),
                ),
                (
                    Rule {
                        id: "owl-transitive",
                        family: RuleFamily::OwlRl,
                        stratum: 1,
                        body: vec![BodyAtom {
                            relation: RelationId::PropertyAssertion,
                            bindings: vec![
                                Binding::Variable(0),
                                Binding::Variable(1),
                                Binding::Variable(2),
                            ],
                        }],
                        head: HeadAtom {
                            relation: RelationId::PropertyAssertion,
                            bindings: vec![
                                Binding::Variable(0),
                                Binding::Variable(1),
                                Binding::Variable(3),
                            ],
                        },
                    },
                    EvalStrategy::SchemaParameterized(SchemaPattern::TransitiveProperty),
                ),
            ],
        }
    }

    pub fn rule_ids(&self) -> Vec<String> {
        self.rules.iter().map(|(r, _)| r.id.to_string()).collect()
    }
}
