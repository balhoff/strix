pub mod delta;
pub mod merge;
pub mod relation;
pub mod segment;

use std::path::{Path, PathBuf};

use crate::dict::TermId;
use crate::error::Result;

use merge::{MergeBinaryIter, MergeTernaryIter};
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
    work_dir: PathBuf,
}

impl FactStore {
    /// Create a new fact store. `store_budget` is the memory budget for the
    /// store's 4 relation buffers (split evenly among them).
    pub fn new(work_dir: &Path, store_budget: usize) -> Result<Self> {
        let budget = store_budget / 4;
        std::fs::create_dir_all(work_dir)?;
        Ok(Self {
            asserted_types: BinaryRelation::new(work_dir, "asserted-types", budget),
            derived_types: BinaryRelation::new(work_dir, "derived-types", budget),
            asserted_properties: TernaryRelation::new(work_dir, "asserted-props", budget),
            derived_properties: TernaryRelation::new(work_dir, "derived-props", budget),
            work_dir: work_dir.to_path_buf(),
        })
    }

    pub fn work_dir(&self) -> &Path {
        &self.work_dir
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

    // --- Streaming scans ---

    /// Streaming iterator over asserted types, sorted and deduplicated.
    pub fn asserted_types_iter(&mut self) -> Result<MergeBinaryIter> {
        let iters = self.asserted_types.segment_iters()?;
        Ok(MergeBinaryIter::new(iters)?)
    }

    /// Streaming iterator over asserted properties, sorted and deduplicated.
    pub fn asserted_properties_iter(&mut self) -> Result<MergeTernaryIter> {
        let iters = self.asserted_properties.segment_iters()?;
        Ok(MergeTernaryIter::new(iters)?)
    }

    /// Streaming iterator over derived types, sorted and deduplicated.
    pub fn derived_types_iter(&mut self) -> Result<MergeBinaryIter> {
        let iters = self.derived_types.segment_iters()?;
        Ok(MergeBinaryIter::new(iters)?)
    }

    /// Streaming iterator over derived properties, sorted and deduplicated.
    pub fn derived_properties_iter(&mut self) -> Result<MergeTernaryIter> {
        let iters = self.derived_properties.segment_iters()?;
        Ok(MergeTernaryIter::new(iters)?)
    }

    /// Streaming iterator over known types (asserted + derived), sorted and deduplicated.
    pub fn known_types_iter(&mut self) -> Result<MergeBinaryIter> {
        let mut iters = self.asserted_types.segment_iters()?;
        iters.extend(self.derived_types.segment_iters()?);
        Ok(MergeBinaryIter::new(iters)?)
    }

    /// Streaming iterator over known properties (asserted + derived), sorted and deduplicated.
    pub fn known_properties_iter(&mut self) -> Result<MergeTernaryIter> {
        let mut iters = self.asserted_properties.segment_iters()?;
        iters.extend(self.derived_properties.segment_iters()?);
        Ok(MergeTernaryIter::new(iters)?)
    }
}
