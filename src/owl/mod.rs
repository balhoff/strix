use std::collections::BTreeSet;

use crate::dict::{Dictionary, TermId};
use crate::rdf::{Term, Triple};
use crate::store::FactStore;

pub const RDF_TYPE_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub const RDF_PROPERTY_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";
pub const RDFS_CLASS_IRI: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
pub const RDFS_SUBCLASS_OF_IRI: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
pub const RDFS_SUBPROPERTY_OF_IRI: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
pub const RDFS_DOMAIN_IRI: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
pub const RDFS_RANGE_IRI: &str = "http://www.w3.org/2000/01/rdf-schema#range";
pub const OWL_CLASS_IRI: &str = "http://www.w3.org/2002/07/owl#Class";
pub const OWL_OBJECT_PROPERTY_IRI: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
pub const OWL_DATATYPE_PROPERTY_IRI: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";

#[derive(Debug, Default)]
pub struct RawSchema {
    pub subclasses: BTreeSet<(TermId, TermId)>,
    pub subproperties: BTreeSet<(TermId, TermId)>,
    pub domains: BTreeSet<(TermId, TermId)>,
    pub ranges: BTreeSet<(TermId, TermId)>,
    pub classes: BTreeSet<TermId>,
    pub properties: BTreeSet<TermId>,
    unsupported: BTreeSet<String>,
}

impl RawSchema {
    pub fn total_axioms(&self) -> usize {
        self.subclasses.len() + self.subproperties.len() + self.domains.len() + self.ranges.len()
    }

    pub fn unsupported_constructs(&self) -> Vec<String> {
        self.unsupported.iter().cloned().collect()
    }
}

pub fn ingest_data_triple(
    triple: Triple,
    extract_schema: bool,
    dictionary: &mut Dictionary,
    schema: &mut RawSchema,
    store: &mut FactStore,
) {
    if extract_schema && should_extract_schema_axiom(&triple) {
        absorb_schema_triple(&triple, dictionary, schema);
        return;
    }

    let subject = dictionary.encode(triple.subject);
    let object = dictionary.encode(triple.object);
    if triple.predicate == RDF_TYPE_IRI {
        store.insert_asserted_type(subject, object);
        return;
    }

    let predicate = dictionary.encode(Term::Iri(triple.predicate));
    store.insert_asserted_property(subject, predicate, object);
}

pub fn ingest_ontology_triple(triple: Triple, dictionary: &mut Dictionary, schema: &mut RawSchema) {
    absorb_schema_triple(&triple, dictionary, schema);
}

fn absorb_schema_triple(triple: &Triple, dictionary: &mut Dictionary, schema: &mut RawSchema) {
    let object_iri = triple.object.as_iri();
    match triple.predicate.as_str() {
        RDFS_SUBCLASS_OF_IRI => match encode_binary_axiom(triple, dictionary) {
            Ok((subclass, superclass)) => {
                schema.subclasses.insert((subclass, superclass));
            }
            Err(error) => record_binary_axiom_issue(schema, "subclass", error),
        },
        RDFS_SUBPROPERTY_OF_IRI => match encode_binary_axiom(triple, dictionary) {
            Ok((subproperty, superproperty)) => {
                schema.subproperties.insert((subproperty, superproperty));
            }
            Err(error) => record_binary_axiom_issue(schema, "subPropertyOf", error),
        },
        RDFS_DOMAIN_IRI => match encode_binary_axiom(triple, dictionary) {
            Ok((property, class)) => {
                schema.domains.insert((property, class));
            }
            Err(error) => record_binary_axiom_issue(schema, "domain", error),
        },
        RDFS_RANGE_IRI => match encode_binary_axiom(triple, dictionary) {
            Ok((property, class)) => {
                schema.ranges.insert((property, class));
            }
            Err(error) => record_binary_axiom_issue(schema, "range", error),
        },
        RDF_TYPE_IRI => match object_iri {
            Some(iri) if iri == RDFS_CLASS_IRI || iri == OWL_CLASS_IRI => {
                match encode_named_iri(&triple.subject, dictionary) {
                    Ok(class) => {
                        schema.classes.insert(class);
                    }
                    Err(error) => record_declaration_issue(schema, "class", error),
                }
            }
            Some(iri)
                if iri == RDF_PROPERTY_IRI
                    || iri == OWL_OBJECT_PROPERTY_IRI
                    || iri == OWL_DATATYPE_PROPERTY_IRI =>
            {
                match encode_named_iri(&triple.subject, dictionary) {
                    Ok(property) => {
                        schema.properties.insert(property);
                    }
                    Err(error) => record_declaration_issue(schema, "property", error),
                }
            }
            Some(iri) if iri.starts_with("http://www.w3.org/2002/07/owl#") => {
                schema
                    .unsupported
                    .insert(format!("{iri} declarations are deferred beyond Phase 1"));
            }
            _ => {}
        },
        predicate if predicate.starts_with("http://www.w3.org/2002/07/owl#") => {
            schema
                .unsupported
                .insert(format!("{predicate} is not implemented in Phase 1"));
        }
        predicate if predicate.starts_with("http://www.w3.org/2000/01/rdf-schema#") => {
            schema
                .unsupported
                .insert(format!("{predicate} is not implemented in Phase 1"));
        }
        _ => {}
    }
}

fn encode_binary_axiom(
    triple: &Triple,
    dictionary: &mut Dictionary,
) -> std::result::Result<(TermId, TermId), SchemaTermError> {
    let left = encode_named_iri(&triple.subject, dictionary)?;
    let right = encode_named_iri(&triple.object, dictionary)?;
    Ok((left, right))
}

fn encode_named_iri(
    term: &Term,
    dictionary: &mut Dictionary,
) -> std::result::Result<TermId, SchemaTermError> {
    match term {
        Term::Iri(_) => Ok(dictionary.encode(term.clone())),
        Term::BlankNode(_) => Err(SchemaTermError::BlankNode),
        Term::Literal(_) => Err(SchemaTermError::Literal),
    }
}

fn should_extract_schema_axiom(triple: &Triple) -> bool {
    match triple.predicate.as_str() {
        RDFS_SUBCLASS_OF_IRI | RDFS_SUBPROPERTY_OF_IRI | RDFS_DOMAIN_IRI | RDFS_RANGE_IRI => true,
        RDF_TYPE_IRI => triple.object.as_iri().is_some_and(|iri| {
            iri == RDFS_CLASS_IRI
                || iri == RDF_PROPERTY_IRI
                || iri == OWL_CLASS_IRI
                || iri == OWL_OBJECT_PROPERTY_IRI
                || iri == OWL_DATATYPE_PROPERTY_IRI
        }),
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SchemaTermError {
    BlankNode,
    Literal,
}

fn record_binary_axiom_issue(schema: &mut RawSchema, label: &str, error: SchemaTermError) {
    match error {
        SchemaTermError::BlankNode => {
            schema.unsupported.insert(format!(
                "blank-node {label} axioms are deferred beyond Phase 1"
            ));
        }
        SchemaTermError::Literal => {
            schema.unsupported.insert(format!(
                "non-IRI {label} axioms are not supported in Phase 1"
            ));
        }
    }
}

fn record_declaration_issue(schema: &mut RawSchema, label: &str, error: SchemaTermError) {
    match error {
        SchemaTermError::BlankNode => {
            schema.unsupported.insert(format!(
                "blank-node {label} declarations are deferred beyond Phase 1"
            ));
        }
        SchemaTermError::Literal => {
            schema.unsupported.insert(format!(
                "non-IRI {label} declarations are not supported in Phase 1"
            ));
        }
    }
}
