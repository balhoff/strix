use crate::owl::{
    OWL_CLASS_IRI, OWL_DATATYPE_PROPERTY_IRI, OWL_NOTHING_IRI, OWL_OBJECT_PROPERTY_IRI,
    OWL_SAME_AS_IRI, OWL_THING_IRI, RDF_PROPERTY_IRI, RDF_TYPE_IRI, RDFS_CLASS_IRI,
    RDFS_DOMAIN_IRI, RDFS_RANGE_IRI, RDFS_SUBCLASS_OF_IRI, RDFS_SUBPROPERTY_OF_IRI,
};

use super::{Dictionary, TermId};

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
    pub owl_same_as: TermId,
    pub owl_thing: TermId,
    pub owl_nothing: TermId,
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
            owl_same_as: dictionary.encode_iri(OWL_SAME_AS_IRI),
            owl_thing: dictionary.encode_iri(OWL_THING_IRI),
            owl_nothing: dictionary.encode_iri(OWL_NOTHING_IRI),
        }
    }
}
