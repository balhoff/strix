use crate::rdf::Term;

use super::TermId;

#[derive(Debug, Default)]
pub(super) struct ReverseMap {
    terms: Vec<Term>,
}

impl ReverseMap {
    pub fn push(&mut self, term: Term) {
        self.terms.push(term);
    }

    pub fn get(&self, id: TermId) -> Option<&Term> {
        if id == 0 {
            return None;
        }
        self.terms.get(id as usize - 1)
    }

    pub fn len(&self) -> usize {
        self.terms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
}
