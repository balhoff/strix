# Optimization Opportunities

Deferred performance improvements identified during development. These are not bugs — the current code is correct — but could improve throughput or reduce resource usage for large datasets.

## Engine: inline rule application during streaming difference

Instead of writing deltas to disk and reading them back, apply rules inline during `difference_streaming_into`. The sink callback would both push to derived storage and generate next-round candidates in a single pass, eliminating the delta write-then-read round-trip.

**Impact**: Removes one full disk write+read cycle per iteration for delta facts. Most significant for large datasets where deltas are substantial in early iterations.

## Engine: cache asserted segment iterators across iterations

`store.known_types_iter()` and `store.known_properties_iter()` re-open asserted segment files every iteration. Asserted segments never change during reasoning — only derived segments grow.

**Impact**: Eliminates redundant file open overhead per iteration. Requires restructuring borrows so asserted iterators can be held while derived segments are mutated.

## Store: unify BinaryRelation and TernaryRelation via generic Relation\<T\>

~100 lines of near-identical code between `BinaryRelation` and `TernaryRelation`. Could be unified into `Relation<T>` parameterized over tuple type, with a trait for serialization.

**Impact**: Reduces maintenance burden — any change to one currently must be mirrored in the other.
