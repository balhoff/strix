use std::collections::HashMap;

use crate::owl::{
    OWL_CLASS_IRI, OWL_DATATYPE_PROPERTY_IRI, OWL_OBJECT_PROPERTY_IRI, RDF_PROPERTY_IRI,
    RDF_TYPE_IRI, RDFS_CLASS_IRI, RDFS_DOMAIN_IRI, RDFS_RANGE_IRI, RDFS_SUBCLASS_OF_IRI,
    RDFS_SUBPROPERTY_OF_IRI,
};
use crate::rdf::Term;

pub type TermId = u64;

#[derive(Debug, Default)]
pub struct Dictionary {
    forward: HashMap<Term, TermId>,
    reverse: Vec<Term>,
}

impl Dictionary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn encode(&mut self, term: Term) -> TermId {
        if let Some(identifier) = self.forward.get(&term) {
            return *identifier;
        }

        let identifier = self.reverse.len() as TermId + 1;
        self.reverse.push(term.clone());
        self.forward.insert(term, identifier);
        identifier
    }

    pub fn encode_iri(&mut self, iri: impl Into<String>) -> TermId {
        self.encode(Term::Iri(iri.into()))
    }

    pub fn decode(&self, identifier: TermId) -> Option<&Term> {
        if identifier == 0 {
            return None;
        }
        self.reverse.get(identifier as usize - 1)
    }

    pub fn len(&self) -> usize {
        self.reverse.len()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WellKnown {
    pub rdf_type: TermId,
    pub rdf_property: TermId,
    pub rdfs_class: TermId,
    pub rdfs_subclass_of: TermId,
    pub rdfs_subproperty_of: TermId,
    pub rdfs_domain: TermId,
    pub rdfs_range: TermId,
    pub owl_class: TermId,
    pub owl_object_property: TermId,
    pub owl_datatype_property: TermId,
}

impl WellKnown {
    pub fn register(dictionary: &mut Dictionary) -> Self {
        Self {
            rdf_type: dictionary.encode_iri(RDF_TYPE_IRI),
            rdf_property: dictionary.encode_iri(RDF_PROPERTY_IRI),
            rdfs_class: dictionary.encode_iri(RDFS_CLASS_IRI),
            rdfs_subclass_of: dictionary.encode_iri(RDFS_SUBCLASS_OF_IRI),
            rdfs_subproperty_of: dictionary.encode_iri(RDFS_SUBPROPERTY_OF_IRI),
            rdfs_domain: dictionary.encode_iri(RDFS_DOMAIN_IRI),
            rdfs_range: dictionary.encode_iri(RDFS_RANGE_IRI),
            owl_class: dictionary.encode_iri(OWL_CLASS_IRI),
            owl_object_property: dictionary.encode_iri(OWL_OBJECT_PROPERTY_IRI),
            owl_datatype_property: dictionary.encode_iri(OWL_DATATYPE_PROPERTY_IRI),
        }
    }
}
