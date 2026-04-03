#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleFamily {
    Rdfs,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule {
    pub id: &'static str,
    pub family: RuleFamily,
    pub description: &'static str,
}

#[derive(Clone, Debug, Default)]
pub struct RuleSet {
    pub rules: Vec<Rule>,
}

impl RuleSet {
    pub fn phase_one() -> Self {
        Self {
            rules: vec![
                Rule {
                    id: "rdfs-subclass",
                    family: RuleFamily::Rdfs,
                    description: "Propagate rdf:type over rdfs:subClassOf*",
                },
                Rule {
                    id: "rdfs-subproperty",
                    family: RuleFamily::Rdfs,
                    description: "Propagate property assertions over rdfs:subPropertyOf*",
                },
                Rule {
                    id: "rdfs-domain",
                    family: RuleFamily::Rdfs,
                    description: "Infer subject types from rdfs:domain",
                },
                Rule {
                    id: "rdfs-range",
                    family: RuleFamily::Rdfs,
                    description: "Infer object types from rdfs:range",
                },
            ],
        }
    }
}
