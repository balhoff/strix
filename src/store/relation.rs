use std::path::{Path, PathBuf};

use rayon::slice::ParallelSliceMut;

use crate::dict::TermId;
use crate::error::Result;

use super::merge::{MergeBinaryIter, MergeTernaryIter};
use super::segment::{
    BinarySegmentIter, Segment, TernarySegmentIter, write_binary_segment,
    write_binary_segment_streaming, write_ternary_segment, write_ternary_segment_streaming,
};

/// A disk-backed relation of `(TermId, TermId)` pairs with in-memory buffer.
#[derive(Debug)]
pub struct BinaryRelation {
    segments: Vec<Segment>,
    buffer: Vec<(TermId, TermId)>,
    work_dir: PathBuf,
    segment_counter: usize,
    label: &'static str,
    budget_bytes: usize,
}

impl BinaryRelation {
    pub fn new(work_dir: &Path, label: &'static str, budget_bytes: usize) -> Self {
        Self {
            segments: Vec::new(),
            buffer: Vec::new(),
            work_dir: work_dir.to_path_buf(),
            segment_counter: 0,
            label,
            budget_bytes,
        }
    }

    pub fn push(&mut self, tuple: (TermId, TermId)) -> Result<()> {
        self.buffer.push(tuple);
        if self.buffer_bytes() >= self.budget_bytes {
            self.flush()?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        self.buffer.par_sort_unstable();
        self.buffer.dedup();
        let path = self.next_segment_path();
        let segment = write_binary_segment(&path, &self.buffer)?;
        self.segments.push(segment);
        self.buffer.clear();
        Ok(())
    }

    /// Compact all segments into one, removing duplicates.
    pub fn compact(&mut self) -> Result<()> {
        self.flush()?;
        if self.segments.len() <= 1 {
            return Ok(());
        }
        let mut iters = Vec::with_capacity(self.segments.len());
        for segment in &self.segments {
            iters.push(BinarySegmentIter::open(&segment.path)?);
        }
        let merge = MergeBinaryIter::new(iters)?;
        let path = self.next_segment_path();
        let new_segment = write_binary_segment_streaming(&path, merge)?;
        for segment in &self.segments {
            let _ = std::fs::remove_file(&segment.path);
        }
        self.segments.clear();
        if new_segment.len > 0 {
            self.segments.push(new_segment);
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.segments.iter().map(|s| s.len).sum::<usize>() + self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty() && self.buffer.is_empty()
    }

    /// Flush the buffer and return streaming readers for all segments.
    pub fn segment_iters(&mut self) -> Result<Vec<BinarySegmentIter>> {
        self.flush()?;
        let mut iters = Vec::with_capacity(self.segments.len());
        for segment in &self.segments {
            iters.push(BinarySegmentIter::open(&segment.path)?);
        }
        Ok(iters)
    }

    fn buffer_bytes(&self) -> usize {
        self.buffer.len() * std::mem::size_of::<(TermId, TermId)>()
    }

    fn next_segment_path(&mut self) -> PathBuf {
        let index = self.segment_counter;
        self.segment_counter += 1;
        self.work_dir
            .join(format!("{}-{:06}.seg", self.label, index))
    }
}

/// A disk-backed relation of `(TermId, TermId, TermId)` triples with in-memory buffer.
#[derive(Debug)]
pub struct TernaryRelation {
    segments: Vec<Segment>,
    buffer: Vec<(TermId, TermId, TermId)>,
    work_dir: PathBuf,
    segment_counter: usize,
    label: &'static str,
    budget_bytes: usize,
}

impl TernaryRelation {
    pub fn new(work_dir: &Path, label: &'static str, budget_bytes: usize) -> Self {
        Self {
            segments: Vec::new(),
            buffer: Vec::new(),
            work_dir: work_dir.to_path_buf(),
            segment_counter: 0,
            label,
            budget_bytes,
        }
    }

    pub fn push(&mut self, tuple: (TermId, TermId, TermId)) -> Result<()> {
        self.buffer.push(tuple);
        if self.buffer_bytes() >= self.budget_bytes {
            self.flush()?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        self.buffer.par_sort_unstable();
        self.buffer.dedup();
        let path = self.next_segment_path();
        let segment = write_ternary_segment(&path, &self.buffer)?;
        self.segments.push(segment);
        self.buffer.clear();
        Ok(())
    }

    /// Compact all segments into one, removing duplicates.
    pub fn compact(&mut self) -> Result<()> {
        self.flush()?;
        if self.segments.len() <= 1 {
            return Ok(());
        }
        let mut iters = Vec::with_capacity(self.segments.len());
        for segment in &self.segments {
            iters.push(TernarySegmentIter::open(&segment.path)?);
        }
        let merge = MergeTernaryIter::new(iters)?;
        let path = self.next_segment_path();
        let new_segment = write_ternary_segment_streaming(&path, merge)?;
        for segment in &self.segments {
            let _ = std::fs::remove_file(&segment.path);
        }
        self.segments.clear();
        if new_segment.len > 0 {
            self.segments.push(new_segment);
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.segments.iter().map(|s| s.len).sum::<usize>() + self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty() && self.buffer.is_empty()
    }

    /// Flush the buffer and return streaming readers for all segments.
    pub fn segment_iters(&mut self) -> Result<Vec<TernarySegmentIter>> {
        self.flush()?;
        let mut iters = Vec::with_capacity(self.segments.len());
        for segment in &self.segments {
            iters.push(TernarySegmentIter::open(&segment.path)?);
        }
        Ok(iters)
    }

    fn buffer_bytes(&self) -> usize {
        self.buffer.len() * std::mem::size_of::<(TermId, TermId, TermId)>()
    }

    fn next_segment_path(&mut self) -> PathBuf {
        let index = self.segment_counter;
        self.segment_counter += 1;
        self.work_dir
            .join(format!("{}-{:06}.seg", self.label, index))
    }
}
