use crate::error::Result;

/// Advance a fallible iterator, converting `io::Error` to `anyhow::Error`.
fn next_ok<T>(iter: &mut impl Iterator<Item = std::io::Result<T>>) -> Result<Option<T>> {
    match iter.next() {
        Some(Ok(v)) => Ok(Some(v)),
        Some(Err(e)) => Err(e.into()),
        None => Ok(None),
    }
}

/// Streaming set difference: candidates NOT in `known`.
/// `candidates` must be sorted. `known` must yield sorted tuples.
pub fn difference_streaming<T: Copy + Ord>(
    candidates: &[T],
    mut known: impl Iterator<Item = std::io::Result<T>>,
) -> Result<Vec<T>> {
    let mut result = Vec::new();
    let mut current_known: Option<T> = next_ok(&mut known)?;

    for &c in candidates {
        while let Some(k) = current_known {
            if k >= c {
                break;
            }
            current_known = next_ok(&mut known)?;
        }
        if current_known != Some(c) {
            result.push(c);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_difference_removes_known() {
        let candidates = vec![(1u64, 2u64), (3, 4), (5, 6)];
        let known = vec![(1u64, 2u64), (5, 6), (7, 8)];
        let result = difference_streaming(&candidates, known.into_iter().map(Ok)).unwrap();
        assert_eq!(result, vec![(3, 4)]);
    }

    #[test]
    fn ternary_difference_removes_known() {
        let candidates = vec![(1u64, 2u64, 3u64), (4, 5, 6)];
        let known = vec![(1u64, 2u64, 3u64)];
        let result = difference_streaming(&candidates, known.into_iter().map(Ok)).unwrap();
        assert_eq!(result, vec![(4, 5, 6)]);
    }

    #[test]
    fn empty_candidates_returns_empty() {
        let candidates: &[(u64, u64)] = &[];
        let known: Vec<(u64, u64)> = vec![(1, 2)];
        let result = difference_streaming(candidates, known.into_iter().map(Ok)).unwrap();
        assert!(result.is_empty());
    }
}
