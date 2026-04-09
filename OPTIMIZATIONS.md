# Optimization Opportunities

Deferred performance improvements identified during development. These are not bugs — the current code is correct — but could improve throughput or reduce resource usage for large datasets.

## Engine: inline rule application during streaming difference

Instead of writing deltas to disk and reading them back, apply rules inline during `difference_streaming_into`. The sink callback would both push to derived storage and generate next-round candidates in a single pass, eliminating the delta write-then-read round-trip.

**Impact**: Removes one full disk write+read cycle per iteration for delta facts. Most significant for large datasets where deltas are substantial in early iterations.

## Engine: cache asserted segment iterators across iterations

`store.known_types_iter()` and `store.known_properties_iter()` re-open asserted segment files every iteration. Asserted segments never change during reasoning — only derived segments grow.

**Impact**: Eliminates redundant file open overhead per iteration. Requires restructuring borrows so asserted iterators can be held while derived segments are mutated.

## Engine: deduplicate universal_types candidates per individual

When `universal_types` is non-empty (from `SubClassOf(owl:Thing, C)`), every individual emits `type(x, C)` for each fact it appears in — producing many duplicates that are later compacted and differenced away. A per-iteration `HashSet<TermId>` of already-seen individuals could skip redundant pushes, or a separate one-shot pass over distinct individuals could replace the per-fact emission.

**Impact**: Only matters when the ontology has `SubClassOf(owl:Thing, C)` (uncommon). Reduces candidate volume and associated sort/merge/difference I/O proportionally to entity fan-out.

## Engine: incremental index updates instead of full rebuild

`build_type_index` and `build_property_index` do a full scan of all known facts every iteration. An incremental approach (add delta entries to the existing index) would avoid rescanning the growing known store each round.

**Impact**: Reduces per-iteration overhead linearly with store size. Requires the index to persist across iterations and careful handling of new derived entries.

## Engine: reuse buffers in property chain walks

`apply_property_join_rules` allocates fresh `Vec`s for each step of the backward/forward chain walk, and for each chain trigger on each delta fact. Pre-allocated reusable buffers (e.g. two `Vec<TermId>` passed in or held in `JoinIndexes`) would eliminate per-walk heap allocations.

**Impact**: Property chains are pervasive in biomedical ontologies, making this a hot path in practice. For length-2 chains each walk is 0-1 steps, but the per-delta allocation cost adds up with large delta sets and many chain triggers.

## Engine: avoid re-seeding inner fixpoint on equality iterations

On the 2nd+ call to `inner_fixpoint` (after equality expansion), the seed pass re-scans all asserted facts and re-applies non-join rules, even though those were already applied in the first pass. The equality expansion only feeds new candidates — the seed rules generate no new work. Skipping the seed pass on subsequent calls (or passing a flag to suppress it) would avoid this redundant scan.

**Impact**: Proportional to the number of asserted facts × number of equality iterations. Only matters when equality rules fire (FunctionalProperty, InverseFunctionalProperty, MaxCardinality 1, asserted sameAs).

## Engine: O(n²) sameAs triple emission for large equivalence classes

`emit_sameas_triples` generates the full pairwise closure of each equivalence class, which is O(n²) per class. For very large classes (thousands of individuals mapped to the same canonical), this could generate millions of sameAs triples. Emitting only star-shaped sameAs (each member linked to the canonical representative) would be O(n) but technically incomplete for the full pairwise closure.

**Impact**: Only matters when equivalence classes are very large, which is unusual in practice.

## Engine: full property scan on every outer equality iteration

`evaluate_equality_rules` does a full scan of all known properties each outer loop iteration to group FP/IFP/MC1 values. An incremental approach that only re-scans properties whose subject or object gained new equivalents would be more efficient.

**Impact**: Proportional to total known properties × number of equality iterations. Significant only when there are many properties and multiple equality rounds.

## Engine: consolidate store scans in inconsistency checks

`check_disjoint_types` and `check_max_card_zero` each independently scan `known_types_iter` (2 scans), and `check_disjoint_properties`, `check_max_card_zero`, `check_irreflexive_properties`, and `check_asymmetric_properties` each independently scan `known_properties_iter` (4 scans). A single pass per relation that dispatches to all relevant checks would reduce I/O significantly.

**Impact**: Proportional to total known facts. Only matters for very large stores where the scan itself is the bottleneck, not the intersection checks.

## Store: Bloom filter for fast duplicate rejection during difference

Add a tunable Bloom filter sized from the memory budget, inserted before the streaming difference check in `difference_streaming_into`. If the Bloom filter says "not present", skip the disk lookup entirely. Only facts that pass the Bloom filter need the full sorted-segment membership test.

**Impact**: Reduces disk I/O during the difference step, which is the bottleneck for large delta sets. Most significant in early iterations when delta volume is high and the known store is large.

## Store: unify BinaryRelation and TernaryRelation via generic Relation\<T\>

~100 lines of near-identical code between `BinaryRelation` and `TernaryRelation`. Could be unified into `Relation<T>` parameterized over tuple type, with a trait for serialization.

**Impact**: Reduces maintenance burden — any change to one currently must be mirrored in the other.
