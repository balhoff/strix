mod parse;

use std::collections::BTreeSet;

use crate::dict::{Dictionary, TermId};
use crate::rdf::{Term, Triple};
use crate::store::FactStore;

pub use parse::{load_extracted_schema, load_ontology_path};

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

#[derive(Debug, Default)]
pub struct ExtractedSchema {
    triples: Vec<Triple>,
}

impl ExtractedSchema {
    pub fn is_empty(&self) -> bool {
        self.triples.is_empty()
    }

    fn push(&mut self, triple: Triple) {
        self.triples.push(triple);
    }
}

pub fn ingest_data_triple(
    triple: Triple,
    extract_schema: bool,
    dictionary: &mut Dictionary,
    extracted_schema: &mut ExtractedSchema,
    store: &mut FactStore,
) {
    if extract_schema && should_extract_schema_axiom(&triple) {
        extracted_schema.push(triple);
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
