use hashbrown::HashMap;

use crate::rdf::Term;

use super::TermId;

#[derive(Debug, Default)]
pub(super) struct ForwardMap {
    map: HashMap<Term, TermId>,
}

impl ForwardMap {
    pub fn get(&self, term: &Term) -> Option<TermId> {
        self.map.get(term).copied()
    }

    pub fn insert(&mut self, term: Term, id: TermId) {
        self.map.insert(term, id);
    }
}
