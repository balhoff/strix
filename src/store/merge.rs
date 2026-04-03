use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::dict::TermId;

/// K-way merge of sorted binary-tuple streams with deduplication.
pub fn merge_sorted_dedup(streams: Vec<Vec<(TermId, TermId)>>) -> Vec<(TermId, TermId)> {
    let mut heap: BinaryHeap<Reverse<(TermId, TermId, usize, usize)>> = BinaryHeap::new();

    for (stream_idx, stream) in streams.iter().enumerate() {
        if let Some(&(a, b)) = stream.first() {
            heap.push(Reverse((a, b, stream_idx, 0)));
        }
    }

    let mut result = Vec::new();
    while let Some(Reverse((a, b, stream_idx, pos))) = heap.pop() {
        if result.last() != Some(&(a, b)) {
            result.push((a, b));
        }
        let next_pos = pos + 1;
        if next_pos < streams[stream_idx].len() {
            let (na, nb) = streams[stream_idx][next_pos];
            heap.push(Reverse((na, nb, stream_idx, next_pos)));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_and_deduplicates() {
        let a = vec![(1, 2), (3, 4), (7, 8)];
        let b = vec![(2, 3), (3, 4), (5, 6)];
        let result = merge_sorted_dedup(vec![a, b]);
        assert_eq!(result, vec![(1, 2), (2, 3), (3, 4), (5, 6), (7, 8)]);
    }

    #[test]
    fn handles_empty_streams() {
        let result = merge_sorted_dedup(vec![vec![], vec![(1, 2)], vec![]]);
        assert_eq!(result, vec![(1, 2)]);
    }
}
