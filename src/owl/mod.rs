mod parse;

use std::collections::{BTreeMap, BTreeSet};

use horned_owl::model::{ClassExpression, DataRange, RcStr};

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
pub const RDFS_LITERAL_IRI: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";
pub const XSD_STRING_IRI: &str = "http://www.w3.org/2001/XMLSchema#string";
pub const RDF_LANG_STRING_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

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

    // Inconsistency axioms
    pub disjoint_classes: Vec<Vec<TermId>>,
    pub disjoint_properties: Vec<Vec<TermId>>,
    pub irreflexive_properties: BTreeSet<TermId>,
    pub asymmetric_properties: BTreeSet<TermId>,

    // Individual axioms
    pub same_individuals: Vec<Vec<TermId>>,
    pub different_individuals: Vec<Vec<TermId>>,

    // Equality-producing axioms
    /// (class, [key_properties]) — HasKey(C, [P1,...,Pn])
    pub has_key: Vec<(TermId, Vec<TermId>)>,

    // SWRL rules
    pub swrl_rules: Vec<RawSwrlRule>,

    // Proxy naming infrastructure (anonymous CE / ObjectInverseOf support)
    pub proxy_counter: u32,
    pub inverse_cache: BTreeMap<TermId, TermId>,
    pub sub_proxy_cache: BTreeMap<ClassExpression<RcStr>, TermId>,
    pub super_proxy_cache: BTreeMap<ClassExpression<RcStr>, TermId>,
    pub data_range_cache: BTreeMap<DataRange<RcStr>, TermId>,
    pub proxy_terms: BTreeSet<TermId>,
    /// Human-readable OFN display string for each proxy TermId.
    pub proxy_display: BTreeMap<TermId, String>,

    // ABox assertions from ontology (OFN/OWX format)
    /// (subject, predicate, object) — ObjectPropertyAssertion / DataPropertyAssertion
    pub extra_property_assertions: Vec<(TermId, TermId, TermId)>,

    /// (literal_term, datatype_term) — type(literal, datatype) from DataPropertyAssertion
    pub literal_datatype_types: Vec<(TermId, TermId)>,

    /// True if the schema uses data range restrictions (DataSomeValuesFrom,
    /// DataAllValuesFrom, DataPropertyRange with a datatype, DataMaxCardinality,
    /// or DataHasValue). When false, literal-to-datatype type assertions can
    /// be skipped to avoid unnecessary overhead in the reasoning loop.
    pub has_data_range_restrictions: bool,

    // Negative assertions
    /// (property, subject, object) — NegativeObjectPropertyAssertion
    pub negative_object_property_assertions: Vec<(TermId, TermId, TermId)>,
    /// (property, subject, value) — NegativeDataPropertyAssertion
    pub negative_data_property_assertions: Vec<(TermId, TermId, TermId)>,

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
            + self.irreflexive_properties.len()
            + self.asymmetric_properties.len()
            + self.has_key.len()
            + self.swrl_rules.len()
            + self.same_individuals.len()
            + self.different_individuals.len()
            + self.negative_object_property_assertions.len()
            + self.negative_data_property_assertions.len()
    }

    pub fn unsupported_constructs(&self) -> Vec<String> {
        self.unsupported.iter().cloned().collect()
    }

    pub fn ontology_report(&self) -> crate::output::report::OntologyReport {
        crate::output::report::OntologyReport {
            total_axioms: self.total_axioms(),
            subclass_of: self.subclasses.len(),
            sub_property_of: self.subproperties.len(),
            domain: self.domains.len(),
            range: self.ranges.len(),
            inverse_of: self.inverse_properties.len(),
            symmetric_property: self.symmetric_properties.len(),
            transitive_property: self.transitive_properties.len(),
            functional_property: self.functional_properties.len(),
            inverse_functional_property: self.inverse_functional_properties.len(),
            property_chain: self.property_chains.len(),
            has_value: self.has_value_super.len() + self.has_value_sub.len(),
            some_values_from: self.some_values_from.len(),
            all_values_from: self.all_values_from.len(),
            intersection_of: self.intersection_of.len(),
            complement_of: self.complement_of.len(),
            one_of: self.one_of_types.len(),
            max_cardinality_zero: self.max_card_zero.len(),
            max_cardinality_one: self.max_card_one.len(),
            disjoint_classes: self.disjoint_classes.len(),
            disjoint_properties: self.disjoint_properties.len(),
            irreflexive_property: self.irreflexive_properties.len(),
            asymmetric_property: self.asymmetric_properties.len(),
            has_key: self.has_key.len(),
            same_individual: self.same_individuals.len(),
            different_individuals: self.different_individuals.len(),
            negative_property_assertion: self.negative_object_property_assertions.len()
                + self.negative_data_property_assertions.len(),
            swrl_rules: self.swrl_rules.len(),
        }
    }
}

#[derive(Debug)]
pub struct RawSwrlRule {
    pub body: Vec<RawSwrlAtom>,
    pub head: Vec<RawSwrlAtom>,
}

#[derive(Debug)]
pub enum RawSwrlAtom {
    ClassAtom {
        class: TermId,
        arg: RawSwrlArg,
    },
    PropertyAtom {
        property: TermId,
        subject: RawSwrlArg,
        object: RawSwrlArg,
    },
    SameIndividualAtom {
        left: RawSwrlArg,
        right: RawSwrlArg,
    },
    DifferentIndividualsAtom {
        left: RawSwrlArg,
        right: RawSwrlArg,
    },
}

#[derive(Debug)]
pub enum RawSwrlArg {
    Variable(TermId),
    Constant(TermId),
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
    literal_types: &mut Vec<(TermId, TermId)>,
) -> Result<()> {
    if extract_schema && should_extract_schema_axiom(&triple) {
        extracted_schema.push(triple);
        return Ok(());
    }

    // Collect literal-datatype pairs for deferred injection. These are only
    // inserted into the store later if the schema uses data range restrictions.
    let literal_datatype = if let Term::Literal(ref lit) = triple.object {
        lit.datatype.as_deref().map(|dt| dictionary.encode_iri(dt))
    } else {
        None
    };

    let subject = dictionary.encode(triple.subject);
    let object = dictionary.encode(triple.object);

    if let Some(datatype_id) = literal_datatype {
        literal_types.push((object, datatype_id));
    }
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
