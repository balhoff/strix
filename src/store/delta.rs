use crate::dict::TermId;

/// Compute the set difference: candidates that are NOT in `known`.
/// Both inputs must be sorted.
pub fn difference_binary(
    candidates: &[(TermId, TermId)],
    known: &[(TermId, TermId)],
) -> Vec<(TermId, TermId)> {
    let mut result = Vec::new();
    let mut ki = 0;
    for &c in candidates {
        while ki < known.len() && known[ki] < c {
            ki += 1;
        }
        if ki >= known.len() || known[ki] != c {
            result.push(c);
        }
    }
    result
}

/// Compute the set difference for ternary tuples.
/// Both inputs must be sorted.
pub fn difference_ternary(
    candidates: &[(TermId, TermId, TermId)],
    known: &[(TermId, TermId, TermId)],
) -> Vec<(TermId, TermId, TermId)> {
    let mut result = Vec::new();
    let mut ki = 0;
    for &c in candidates {
        while ki < known.len() && known[ki] < c {
            ki += 1;
        }
        if ki >= known.len() || known[ki] != c {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_difference_removes_known() {
        let candidates = vec![(1, 2), (3, 4), (5, 6)];
        let known = vec![(1, 2), (5, 6), (7, 8)];
        assert_eq!(difference_binary(&candidates, &known), vec![(3, 4)]);
    }

    #[test]
    fn ternary_difference_removes_known() {
        let candidates = vec![(1, 2, 3), (4, 5, 6)];
        let known = vec![(1, 2, 3)];
        assert_eq!(difference_ternary(&candidates, &known), vec![(4, 5, 6)]);
    }

    #[test]
    fn empty_candidates_returns_empty() {
        assert_eq!(difference_binary(&[], &[(1, 2)]), Vec::<(u64, u64)>::new());
    }
}
