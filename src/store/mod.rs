pub mod delta;
pub mod merge;
pub mod relation;
pub mod segment;

use std::path::{Path, PathBuf};

use crate::dict::TermId;
use crate::error::Result;

use relation::{BinaryRelation, TernaryRelation};

/// Predicate-partitioned fact store with disk-backed sorted-run segments.
///
/// ABox facts are split into `type_assertions` (instance, class) and
/// `property_assertions` (subject, predicate, object), each with
/// separate asserted and derived segment sets.
#[derive(Debug)]
pub struct FactStore {
    asserted_types: BinaryRelation,
    derived_types: BinaryRelation,
    asserted_properties: TernaryRelation,
    derived_properties: TernaryRelation,
    _work_dir: PathBuf,
}

/// Default budget per relation buffer (1/8 of total budget, shared across 4 relations).
fn relation_budget(total_budget: usize) -> usize {
    total_budget / 4
}

impl FactStore {
    pub fn new(work_dir: &Path, memory_budget: usize) -> Result<Self> {
        let budget = relation_budget(memory_budget);
        std::fs::create_dir_all(work_dir)?;
        Ok(Self {
            asserted_types: BinaryRelation::new(work_dir, "asserted-types", budget),
            derived_types: BinaryRelation::new(work_dir, "derived-types", budget),
            asserted_properties: TernaryRelation::new(work_dir, "asserted-props", budget),
            derived_properties: TernaryRelation::new(work_dir, "derived-props", budget),
            _work_dir: work_dir.to_path_buf(),
        })
    }

    // --- Insertion ---

    pub fn insert_asserted_type(&mut self, instance: TermId, class: TermId) -> Result<()> {
        self.asserted_types.push((instance, class))
    }

    pub fn insert_asserted_property(
        &mut self,
        subject: TermId,
        predicate: TermId,
        object: TermId,
    ) -> Result<()> {
        self.asserted_properties.push((subject, predicate, object))
    }

    // --- Derived facts (for engine) ---

    pub fn derived_types_mut(&mut self) -> &mut BinaryRelation {
        &mut self.derived_types
    }

    pub fn derived_properties_mut(&mut self) -> &mut TernaryRelation {
        &mut self.derived_properties
    }

    // --- Scans ---

    /// All asserted type facts, sorted and deduplicated.
    pub fn asserted_types(&mut self) -> Result<Vec<(TermId, TermId)>> {
        self.asserted_types.scan()
    }

    /// All derived type facts, sorted and deduplicated.
    pub fn derived_types(&mut self) -> Result<Vec<(TermId, TermId)>> {
        self.derived_types.scan()
    }

    /// All asserted property facts, sorted and deduplicated.
    pub fn asserted_properties(&mut self) -> Result<Vec<(TermId, TermId, TermId)>> {
        self.asserted_properties.scan()
    }

    /// All derived property facts, sorted and deduplicated.
    pub fn derived_properties(&mut self) -> Result<Vec<(TermId, TermId, TermId)>> {
        self.derived_properties.scan()
    }

    /// Known view: asserted + derived types, sorted and deduplicated.
    pub fn known_types(&mut self) -> Result<Vec<(TermId, TermId)>> {
        let mut known = self.asserted_types.scan()?;
        known.extend(self.derived_types.scan()?);
        known.sort_unstable();
        known.dedup();
        Ok(known)
    }

    /// Known view: asserted + derived properties, sorted and deduplicated.
    pub fn known_properties(&mut self) -> Result<Vec<(TermId, TermId, TermId)>> {
        let mut known = self.asserted_properties.scan()?;
        known.extend(self.derived_properties.scan()?);
        known.sort_unstable();
        known.dedup();
        Ok(known)
    }

    /// Count of derived (inferred) facts.
    pub fn inferred_count(&mut self) -> Result<usize> {
        Ok(self.derived_types.scan()?.len() + self.derived_properties.scan()?.len())
    }
}
