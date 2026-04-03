use std::collections::BTreeSet;

use crate::dict::TermId;

#[derive(Debug, Default)]
pub struct FactStore {
    asserted_types: BTreeSet<(TermId, TermId)>,
    derived_types: BTreeSet<(TermId, TermId)>,
    asserted_properties: BTreeSet<(TermId, TermId, TermId)>,
    derived_properties: BTreeSet<(TermId, TermId, TermId)>,
}

impl FactStore {
    pub fn insert_asserted_type(&mut self, instance: TermId, class: TermId) {
        self.asserted_types.insert((instance, class));
    }

    pub fn insert_asserted_property(&mut self, subject: TermId, predicate: TermId, object: TermId) {
        self.asserted_properties
            .insert((subject, predicate, object));
    }

    pub fn insert_derived_type(&mut self, instance: TermId, class: TermId) -> bool {
        if self.asserted_types.contains(&(instance, class))
            || self.derived_types.contains(&(instance, class))
        {
            return false;
        }
        self.derived_types.insert((instance, class))
    }

    pub fn insert_derived_property(
        &mut self,
        subject: TermId,
        predicate: TermId,
        object: TermId,
    ) -> bool {
        if self
            .asserted_properties
            .contains(&(subject, predicate, object))
            || self
                .derived_properties
                .contains(&(subject, predicate, object))
        {
            return false;
        }
        self.derived_properties.insert((subject, predicate, object))
    }

    pub fn asserted_types(&self) -> impl Iterator<Item = (TermId, TermId)> + '_ {
        self.asserted_types.iter().copied()
    }

    pub fn asserted_properties(&self) -> impl Iterator<Item = (TermId, TermId, TermId)> + '_ {
        self.asserted_properties.iter().copied()
    }

    pub fn derived_types(&self) -> impl Iterator<Item = (TermId, TermId)> + '_ {
        self.derived_types.iter().copied()
    }

    pub fn derived_properties(&self) -> impl Iterator<Item = (TermId, TermId, TermId)> + '_ {
        self.derived_properties.iter().copied()
    }

    pub fn closure_types(&self) -> impl Iterator<Item = (TermId, TermId)> + '_ {
        self.asserted_types().chain(self.derived_types())
    }

    pub fn closure_properties(&self) -> impl Iterator<Item = (TermId, TermId, TermId)> + '_ {
        self.asserted_properties().chain(self.derived_properties())
    }

    pub fn inferred_count(&self) -> usize {
        self.derived_types.len() + self.derived_properties.len()
    }
}
