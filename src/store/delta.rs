use crate::error::Result;

/// Advance a fallible iterator, converting `io::Error` to `anyhow::Error`.
fn next_ok<T>(iter: &mut impl Iterator<Item = std::io::Result<T>>) -> Result<Option<T>> {
    match iter.next() {
        Some(Ok(v)) => Ok(Some(v)),
        Some(Err(e)) => Err(e.into()),
        None => Ok(None),
    }
}

/// Streaming set difference: emit candidates NOT in `known` to a sink.
/// Both `candidates` and `known` must yield sorted, deduplicated tuples.
/// Returns the count of novel facts emitted to the sink.
pub fn difference_streaming_into<T, I1, I2, F>(
    mut candidates: I1,
    mut known: I2,
    mut sink: F,
) -> Result<usize>
where
    T: Copy + Ord,
    I1: Iterator<Item = std::io::Result<T>>,
    I2: Iterator<Item = std::io::Result<T>>,
    F: FnMut(T) -> Result<()>,
{
    let mut count = 0usize;
    let mut current_candidate: Option<T> = next_ok(&mut candidates)?;
    let mut current_known: Option<T> = next_ok(&mut known)?;

    while let Some(c) = current_candidate {
        while let Some(k) = current_known {
            if k >= c {
                break;
            }
            current_known = next_ok(&mut known)?;
        }
        if current_known != Some(c) {
            sink(c)?;
            count += 1;
        }
        current_candidate = next_ok(&mut candidates)?;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_novel_binary_tuples() {
        let candidates = vec![(1u64, 2u64), (3, 4), (5, 6)];
        let known = vec![(1u64, 2u64), (5, 6), (7, 8)];
        let mut result = Vec::new();
        let count = difference_streaming_into(
            candidates.into_iter().map(Ok),
            known.into_iter().map(Ok),
            |t| {
                result.push(t);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(result, vec![(3, 4)]);
        assert_eq!(count, 1);
    }

    #[test]
    fn emits_novel_ternary_tuples() {
        let candidates = vec![(1u64, 2u64, 3u64), (4, 5, 6)];
        let known = vec![(1u64, 2u64, 3u64)];
        let mut result = Vec::new();
        let count = difference_streaming_into(
            candidates.into_iter().map(Ok),
            known.into_iter().map(Ok),
            |t| {
                result.push(t);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(result, vec![(4, 5, 6)]);
        assert_eq!(count, 1);
    }

    #[test]
    fn empty_candidates_emits_nothing() {
        let candidates: Vec<(u64, u64)> = vec![];
        let known: Vec<(u64, u64)> = vec![(1, 2)];
        let mut result = Vec::new();
        let count = difference_streaming_into(
            candidates.into_iter().map(Ok),
            known.into_iter().map(Ok),
            |t| {
                result.push(t);
                Ok(())
            },
        )
        .unwrap();
        assert!(result.is_empty());
        assert_eq!(count, 0);
    }

    #[test]
    fn empty_known_emits_all_candidates() {
        let candidates = vec![(1u64, 2u64), (3, 4)];
        let known: Vec<(u64, u64)> = vec![];
        let mut result = Vec::new();
        let count = difference_streaming_into(
            candidates.into_iter().map(Ok),
            known.into_iter().map(Ok),
            |t| {
                result.push(t);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(result, vec![(1, 2), (3, 4)]);
        assert_eq!(count, 2);
    }
}
