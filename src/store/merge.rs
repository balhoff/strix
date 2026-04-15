use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::dict::TermId;

use super::segment::{BinarySegmentIter, TernarySegmentIter};

/// K-way merge iterator over sorted tuple segment streams with deduplication.
pub struct MergeIter<T: Ord + Copy, I: Iterator<Item = std::io::Result<T>>> {
    heap: BinaryHeap<Reverse<(T, usize)>>,
    iters: Vec<I>,
    last: Option<T>,
    errored: bool,
}

impl<T: Ord + Copy, I: Iterator<Item = std::io::Result<T>>> MergeIter<T, I> {
    pub fn new(mut iters: Vec<I>) -> std::io::Result<Self> {
        let mut heap = BinaryHeap::new();
        for (idx, iter) in iters.iter_mut().enumerate() {
            match iter.next() {
                Some(Ok(tuple)) => heap.push(Reverse((tuple, idx))),
                Some(Err(e)) => return Err(e),
                None => {}
            }
        }
        Ok(Self {
            heap,
            iters,
            last: None,
            errored: false,
        })
    }
}

impl<T: Ord + Copy, I: Iterator<Item = std::io::Result<T>>> Iterator for MergeIter<T, I> {
    type Item = std::io::Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.errored {
            return None;
        }
        loop {
            let Reverse((tuple, idx)) = self.heap.pop()?;

            match self.iters[idx].next() {
                Some(Ok(next_tuple)) => self.heap.push(Reverse((next_tuple, idx))),
                Some(Err(e)) => {
                    self.errored = true;
                    return Some(Err(e));
                }
                None => {}
            }

            if self.last == Some(tuple) {
                continue;
            }
            self.last = Some(tuple);
            return Some(Ok(tuple));
        }
    }
}

pub type MergeBinaryIter = MergeIter<(TermId, TermId), BinarySegmentIter>;
pub type MergeTernaryIter = MergeIter<(TermId, TermId, TermId), TernarySegmentIter>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_merge_binary_deduplicates() {
        use super::super::segment::{BinarySegmentIter, write_binary_segment};

        let dir = tempfile::TempDir::new().unwrap();
        let p1 = dir.path().join("a.seg");
        let p2 = dir.path().join("b.seg");
        write_binary_segment(&p1, &[(1, 2), (3, 4), (7, 8)]).unwrap();
        write_binary_segment(&p2, &[(2, 3), (3, 4), (5, 6)]).unwrap();

        let iters = vec![
            BinarySegmentIter::open(&p1).unwrap(),
            BinarySegmentIter::open(&p2).unwrap(),
        ];
        let merged: Vec<_> = MergeBinaryIter::new(iters)
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(merged, vec![(1, 2), (2, 3), (3, 4), (5, 6), (7, 8)]);
    }

    #[test]
    fn streaming_merge_ternary_deduplicates() {
        use super::super::segment::{TernarySegmentIter, write_ternary_segment};

        let dir = tempfile::TempDir::new().unwrap();
        let p1 = dir.path().join("a.seg");
        let p2 = dir.path().join("b.seg");
        write_ternary_segment(&p1, &[(1, 2, 3), (4, 5, 6)]).unwrap();
        write_ternary_segment(&p2, &[(1, 2, 3), (7, 8, 9)]).unwrap();

        let iters = vec![
            TernarySegmentIter::open(&p1).unwrap(),
            TernarySegmentIter::open(&p2).unwrap(),
        ];
        let merged: Vec<_> = MergeTernaryIter::new(iters)
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(merged, vec![(1, 2, 3), (4, 5, 6), (7, 8, 9)]);
    }

    #[test]
    fn streaming_merge_empty_segments() {
        let iters: Vec<BinarySegmentIter> = vec![];
        let merged: Vec<_> = MergeBinaryIter::new(iters)
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(merged.is_empty());
    }
}
