use std::collections::{BTreeMap, HashMap};

use crate::dict::TermId;

/// Union-find for owl:sameAs equivalence classes.
///
/// Uses path compression and a lower-TermId-as-root policy so canonical
/// representatives are deterministic and stable across runs.
pub struct UnionFind {
    parent: HashMap<TermId, TermId>,
}

impl Default for UnionFind {
    fn default() -> Self {
        Self::new()
    }
}

impl UnionFind {
    pub fn new() -> Self {
        Self {
            parent: HashMap::new(),
        }
    }

    /// Find the canonical representative of `x` with path compression.
    pub fn find(&mut self, x: TermId) -> TermId {
        let p = match self.parent.get(&x) {
            Some(&p) => p,
            None => return x, // not in any equivalence class
        };
        if p == x {
            return x;
        }
        let root = self.find(p);
        self.parent.insert(x, root);
        root
    }

    /// Merge the equivalence classes of `x` and `y`.
    /// Returns `true` if a new merge occurred (they were in different classes).
    pub fn union(&mut self, x: TermId, y: TermId) -> bool {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry {
            return false;
        }
        // Lower TermId becomes the root for deterministic output.
        let (root, child) = if rx < ry { (rx, ry) } else { (ry, rx) };
        self.parent.insert(child, root);
        self.parent.entry(root).or_insert(root);
        true
    }

    /// Merge all items in the slice into the same equivalence class.
    /// Returns the number of new merges performed.
    pub fn union_all(&mut self, items: &[TermId]) -> usize {
        if items.len() <= 1 {
            return 0;
        }
        let mut count = 0;
        let first = self.canonical(items[0]);
        for &item in &items[1..] {
            if self.union(first, item) {
                count += 1;
            }
        }
        count
    }

    /// Alias for `find` — returns the canonical representative.
    pub fn canonical(&mut self, x: TermId) -> TermId {
        self.find(x)
    }

    /// Whether any non-trivial equivalence classes exist.
    pub fn has_merges(&self) -> bool {
        self.parent.iter().any(|(k, v)| k != v)
    }

    /// Collect equivalence classes as canonical → [members] (including canonical).
    pub fn equivalence_classes(&mut self) -> BTreeMap<TermId, Vec<TermId>> {
        // Snapshot keys to avoid borrow issues during find.
        let keys: Vec<TermId> = self.parent.keys().copied().collect();
        let mut classes: BTreeMap<TermId, Vec<TermId>> = BTreeMap::new();
        for term in keys {
            let root = self.find(term);
            classes.entry(root).or_default().push(term);
        }
        // Only keep non-trivial classes (size > 1).
        classes.retain(|_, members| members.len() > 1);
        for members in classes.values_mut() {
            members.sort_unstable();
        }
        classes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_union_find() {
        let mut uf = UnionFind::new();
        assert!(!uf.has_merges());
        assert_eq!(uf.find(10), 10);
        assert_eq!(uf.find(20), 20);

        assert!(uf.union(10, 20));
        assert!(uf.has_merges());
        assert_eq!(uf.find(10), 10); // lower ID is root
        assert_eq!(uf.find(20), 10);

        // Duplicate merge returns false
        assert!(!uf.union(10, 20));
    }

    #[test]
    fn transitive_merges() {
        let mut uf = UnionFind::new();
        uf.union(30, 20);
        uf.union(20, 10);
        // All should resolve to 10 (lowest)
        assert_eq!(uf.find(30), 10);
        assert_eq!(uf.find(20), 10);
        assert_eq!(uf.find(10), 10);
    }

    #[test]
    fn equivalence_classes_correct() {
        let mut uf = UnionFind::new();
        uf.union(1, 2);
        uf.union(3, 4);
        uf.union(2, 3);

        let classes = uf.equivalence_classes();
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[&1], vec![1, 2, 3, 4]);
    }

    #[test]
    fn union_all_merges_group() {
        let mut uf = UnionFind::new();
        assert_eq!(uf.union_all(&[]), 0);
        assert_eq!(uf.union_all(&[42]), 0);
        assert_eq!(uf.union_all(&[10, 20, 30]), 2);
        assert_eq!(uf.find(20), 10);
        assert_eq!(uf.find(30), 10);
        // Re-merging same group produces no new merges.
        assert_eq!(uf.union_all(&[10, 20, 30]), 0);
    }

    #[test]
    fn singleton_not_in_classes() {
        let mut uf = UnionFind::new();
        uf.union(1, 2);
        // 5 was never mentioned — not in equivalence classes
        assert_eq!(uf.find(5), 5);
        let classes = uf.equivalence_classes();
        assert_eq!(classes.len(), 1);
        assert!(!classes.contains_key(&5));
    }
}
