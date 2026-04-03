use std::path::{Path, PathBuf};

use rayon::slice::ParallelSliceMut;

use crate::dict::TermId;
use crate::error::Result;

use super::merge::merge_sorted_dedup;
use super::segment::{
    Segment, read_binary_segment, read_ternary_segment, write_binary_segment, write_ternary_segment,
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

    /// Return a sorted, deduplicated iterator over all tuples (segments + buffer).
    pub fn scan(&mut self) -> Result<Vec<(TermId, TermId)>> {
        self.flush()?;
        let mut all = Vec::new();
        for segment in &self.segments {
            all.extend(read_binary_segment(&segment.path)?);
        }
        all.par_sort_unstable();
        all.dedup();
        Ok(all)
    }

    /// Compact all segments into one, removing duplicates.
    pub fn compact(&mut self) -> Result<()> {
        self.flush()?;
        if self.segments.len() <= 1 {
            return Ok(());
        }
        let mut streams: Vec<Vec<(TermId, TermId)>> = Vec::new();
        for segment in &self.segments {
            streams.push(read_binary_segment(&segment.path)?);
        }
        let merged = merge_sorted_dedup(streams);
        // Remove old segment files
        for segment in &self.segments {
            let _ = std::fs::remove_file(&segment.path);
        }
        self.segments.clear();
        if !merged.is_empty() {
            let path = self.next_segment_path();
            let segment = write_binary_segment(&path, &merged)?;
            self.segments.push(segment);
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.segments.iter().map(|s| s.len).sum::<usize>() + self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty() && self.buffer.is_empty()
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

    /// Return a sorted, deduplicated iterator over all tuples (segments + buffer).
    pub fn scan(&mut self) -> Result<Vec<(TermId, TermId, TermId)>> {
        self.flush()?;
        let mut all = Vec::new();
        for segment in &self.segments {
            all.extend(read_ternary_segment(&segment.path)?);
        }
        all.par_sort_unstable();
        all.dedup();
        Ok(all)
    }

    /// Compact all segments into one, removing duplicates.
    pub fn compact(&mut self) -> Result<()> {
        self.flush()?;
        if self.segments.len() <= 1 {
            return Ok(());
        }
        let mut streams: Vec<Vec<(TermId, TermId, TermId)>> = Vec::new();
        for segment in &self.segments {
            streams.push(read_ternary_segment(&segment.path)?);
        }
        let merged = merge_sorted_dedup_ternary(streams);
        for segment in &self.segments {
            let _ = std::fs::remove_file(&segment.path);
        }
        self.segments.clear();
        if !merged.is_empty() {
            let path = self.next_segment_path();
            let segment = write_ternary_segment(&path, &merged)?;
            self.segments.push(segment);
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.segments.iter().map(|s| s.len).sum::<usize>() + self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty() && self.buffer.is_empty()
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

/// K-way merge + dedup for ternary tuples.
fn merge_sorted_dedup_ternary(
    streams: Vec<Vec<(TermId, TermId, TermId)>>,
) -> Vec<(TermId, TermId, TermId)> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;

    type HeapEntry = Reverse<(TermId, TermId, TermId, usize, usize)>;
    let mut heap: BinaryHeap<HeapEntry> = BinaryHeap::new();

    for (stream_idx, stream) in streams.iter().enumerate() {
        if let Some(&(a, b, c)) = stream.first() {
            heap.push(Reverse((a, b, c, stream_idx, 0)));
        }
    }

    let mut result = Vec::new();
    while let Some(Reverse((a, b, c, stream_idx, pos))) = heap.pop() {
        if result.last() != Some(&(a, b, c)) {
            result.push((a, b, c));
        }
        let next_pos = pos + 1;
        if next_pos < streams[stream_idx].len() {
            let (na, nb, nc) = streams[stream_idx][next_pos];
            heap.push(Reverse((na, nb, nc, stream_idx, next_pos)));
        }
    }

    result
}
