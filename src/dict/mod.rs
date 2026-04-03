mod decoding;
mod encoding;
mod well_known;

pub use well_known::WellKnown;

use crate::rdf::Term;

use decoding::ReverseMap;
use encoding::ForwardMap;

pub type TermId = u64;

#[derive(Debug, Default)]
pub struct Dictionary {
    forward: ForwardMap,
    reverse: ReverseMap,
}

impl Dictionary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn encode(&mut self, term: Term) -> TermId {
        if let Some(id) = self.forward.get(&term) {
            return id;
        }

        let id = self.reverse.len() as TermId + 1;
        self.reverse.push(term.clone());
        self.forward.insert(term, id);
        id
    }

    pub fn encode_iri(&mut self, iri: &str) -> TermId {
        self.encode(Term::Iri(iri.to_owned()))
    }

    pub fn decode(&self, id: TermId) -> Option<&Term> {
        self.reverse.get(id)
    }

    pub fn len(&self) -> usize {
        self.reverse.len()
    }

    pub fn is_empty(&self) -> bool {
        self.reverse.is_empty()
    }
}
