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

## Store: unify BinaryRelation and TernaryRelation via generic Relation\<T\>

~100 lines of near-identical code between `BinaryRelation` and `TernaryRelation`. Could be unified into `Relation<T>` parameterized over tuple type, with a trait for serialization.

**Impact**: Reduces maintenance burden — any change to one currently must be mirrored in the other.
