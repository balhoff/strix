mod parse;

use std::collections::BTreeSet;

use crate::dict::{Dictionary, TermId};
use crate::error::Result;
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
pub const OWL_SAME_AS_IRI: &str = "http://www.w3.org/2002/07/owl#sameAs";
pub const OWL_THING_IRI: &str = "http://www.w3.org/2002/07/owl#Thing";
pub const OWL_NOTHING_IRI: &str = "http://www.w3.org/2002/07/owl#Nothing";

#[derive(Debug, Default)]
pub struct RawSchema {
    // RDFS axioms (Phase 1)
    pub subclasses: BTreeSet<(TermId, TermId)>,
    pub subproperties: BTreeSet<(TermId, TermId)>,
    pub domains: BTreeSet<(TermId, TermId)>,
    pub ranges: BTreeSet<(TermId, TermId)>,
    pub classes: BTreeSet<TermId>,
    pub properties: BTreeSet<TermId>,

    // Property axioms (Phase 2)
    pub inverse_properties: BTreeSet<(TermId, TermId)>,
    pub symmetric_properties: BTreeSet<TermId>,
    pub transitive_properties: BTreeSet<TermId>,
    pub functional_properties: BTreeSet<TermId>,
    pub inverse_functional_properties: BTreeSet<TermId>,
    pub property_chains: Vec<(TermId, Vec<TermId>)>,

    // Class restrictions (Phase 2)
    /// (class, property, value) — SubClassOf(HasValue(P,v), C): property(x,P,v) → type(x,C)
    pub has_value_super: Vec<(TermId, TermId, TermId)>,
    /// (class, property, value) — SubClassOf(A, HasValue(P,v)): type(x,A) → property(x,P,v)
    pub has_value_sub: Vec<(TermId, TermId, TermId)>,
    /// (class, property, filler) — SubClassOf(SomeValuesFrom(P,D), C)
    pub some_values_from: Vec<(TermId, TermId, TermId)>,
    /// (class, property, filler) — SubClassOf(A, AllValuesFrom(P,B))
    pub all_values_from: Vec<(TermId, TermId, TermId)>,
    /// (class, [conjuncts]) — SubClassOf(IntersectionOf([C1,...]), C)
    pub intersection_of: Vec<(TermId, Vec<TermId>)>,
    /// (class, complement) — SubClassOf(A, ComplementOf(D))
    pub complement_of: BTreeSet<(TermId, TermId)>,
    /// (individual, class) — from SubClassOf(OneOf([a1,...]), C)
    pub one_of_types: Vec<(TermId, TermId)>,
    /// (class, property, optional_filler) — SubClassOf(A, MaxCard(0,P,C))
    pub max_card_zero: Vec<(TermId, TermId, Option<TermId>)>,
    /// (class, property, optional_filler) — SubClassOf(A, MaxCard(1,P,C))
    pub max_card_one: Vec<(TermId, TermId, Option<TermId>)>,

    // Inconsistency axioms (Phase 2)
    pub disjoint_classes: Vec<Vec<TermId>>,
    pub disjoint_properties: Vec<Vec<TermId>>,

    unsupported: BTreeSet<String>,
}

impl RawSchema {
    pub fn total_axioms(&self) -> usize {
        self.subclasses.len()
            + self.subproperties.len()
            + self.domains.len()
            + self.ranges.len()
            + self.inverse_properties.len()
            + self.symmetric_properties.len()
            + self.transitive_properties.len()
            + self.functional_properties.len()
            + self.inverse_functional_properties.len()
            + self.property_chains.len()
            + self.has_value_super.len()
            + self.has_value_sub.len()
            + self.some_values_from.len()
            + self.all_values_from.len()
            + self.intersection_of.len()
            + self.complement_of.len()
            + self.one_of_types.len()
            + self.max_card_zero.len()
            + self.max_card_one.len()
            + self.disjoint_classes.len()
            + self.disjoint_properties.len()
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
) -> Result<()> {
    if extract_schema && should_extract_schema_axiom(&triple) {
        extracted_schema.push(triple);
        return Ok(());
    }

    let subject = dictionary.encode(triple.subject);
    let object = dictionary.encode(triple.object);
    if triple.predicate == RDF_TYPE_IRI {
        store.insert_asserted_type(subject, object)?;
        return Ok(());
    }

    let predicate = dictionary.encode(Term::Iri(triple.predicate));
    store.insert_asserted_property(subject, predicate, object)?;
    Ok(())
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
