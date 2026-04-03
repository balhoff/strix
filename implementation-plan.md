# Strix: Implementation Plan

## Design Decisions

This plan synthesizes the two design proposals, favoring Proposal 1's architecture with Proposal 2's phasing strategy. Key choices:

| Area | Decision | Source |
|------|----------|--------|
| Storage | Custom sorted-run relation store with predicate-partitioned relations from the start | Proposal 1 |
| Rule IR | Typed internal rule IR as the unified compilation target for OWL RL and SWRL | Proposal 1 |
| Crate structure | Single crate with module hierarchy | Proposal 2 |
| Phasing | End-to-end RDFS reasoning in Phase 1 | Proposal 2 |
| HDT | Ingest adapter only; reasoning never depends on HDT access patterns | Proposal 1 |
| Diagnostics | Structured run-report.json with three-way outcome categorization | Proposal 1 |
| Duplicate detection | Merge-based dedup as foundation; Bloom filters as tunable optimization | Both |
| Fact provenance | Keep asserted and derived ABox facts in separate segment sets from Phase 1 | This plan |
| SWRL | Designed into rule IR from Phase 1; parsing/execution in Phase 3 | Proposal 1 |
| `owl:sameAs` | Union-find canonicalization, not pairwise expansion | Both |
| OWL parsing | `horned-owl` crate (TBox/ontology only; ABox data via oxrdfio) | User choice |
| RDF parsing | `oxrdfio` + `oxrdf` (following hdtc; also used internally by horned-owl v1.4) | hdtc convention |
| CLI | `clap` 4 with derive macros (following hdtc) | hdtc convention |
| Logging | `tracing` + `tracing-subscriber` (following hdtc) | hdtc convention |
| Errors | `anyhow` + `thiserror` (following hdtc) | hdtc convention |

---

## Conventions (adopted from hdtc)

### CLI

Use `clap` 4 with derive macros. Top-level `Cli` struct with global flags, `Commands` enum for subcommands.

Global flags:
- `--verbose` / `-v` (count): 0=info, 1=debug, 2+=trace
- `--quiet` / `-q`: errors only
- `--benchmark`: per-stage timing and peak RSS

Custom `MemorySize` type for `--memory-budget` (parses "4G", "2000M", etc.).

### Logging

`tracing` with `tracing-subscriber`. Configure from verbose/quiet flags. Output to stderr. Disable module targets (`with_target(false)`).

```rust
let filter = match (cli.quiet, cli.verbose) {
    (true, _) => EnvFilter::new("error"),
    (_, 0)    => EnvFilter::new("info"),
    (_, 1)    => EnvFilter::new("debug"),
    (_, _)    => EnvFilter::new("trace"),
};
tracing_subscriber::fmt()
    .with_env_filter(filter)
    .with_writer(std::io::stderr)
    .with_target(false)
    .init();
```

Use `tracing::info!` for milestones, `debug!` for diagnostics, `warn!` for skipped/malformed input, `error!` for fatal conditions.

### RDF parsing

`oxrdfio` for multi-format RDF parsing (N-Triples, N-Quads, Turtle, TriG, RDF/XML). `oxrdf` for term model. Lenient parsing mode — skip malformed triples, log first N errors.

Auto-detect format from file extension. Support gzip/bzip2/xz decompression (detect from extension). Recursive input discovery for directories.

### OWL parsing

`horned-owl` for OWL 2 ontology and SWRL parsing. Supports OWL/XML, Functional Syntax, and RDF-based OWL serializations. Targets million-term ontologies. Used for TBox/ontology only — ABox data flows through oxrdfio and never touches horned-owl.

Published horned-owl (v1.4.0) uses oxrdf + oxrdfio internally for its RDF-based parsing path. This means both libraries share the same RDF term model.

**Ontology loading paths:**

- **Separate ontology file (`--ontology`)**: Hand the file directly to horned-owl's reader. It handles format detection and parsing internally via oxrdfio. Simplest path.

- **Extract from data (`--extract-ontology`)**: During ingest via oxrdfio, collect ontology triples from the data stream and buffer them in memory (TBox is bounded). When a separate `--ontology` file is not provided, extraction is the default. When both `--ontology` and `--extract-ontology` are provided, merge the extracted ontology graph with the separate ontology before normalization.

- **Extraction scope by phase**:
  - **Phase 1**: collect only the RDFS/RL schema triples needed for early reasoning: `rdf:type` declarations for classes/properties plus direct uses of `rdfs:subClassOf`, `rdfs:subPropertyOf`, `rdfs:domain`, and `rdfs:range`. This is sufficient for the Phase 1 compiler and avoids blank-node graph chasing.
  - **Phase 2+**: extend extraction to pull the reachable blank-node/list closure required for RDF-based OWL constructs such as restrictions, `owl:intersectionOf`, `owl:unionOf`, and `owl:propertyChainAxiom`.

- **Buffer handoff to horned-owl**: Serialize the buffered ontology triples to N-Triples into a `Vec<u8>` and pass a `Cursor<Vec<u8>>` to horned-owl's `from_bufread` reader. The re-serialization cost is negligible relative to ABox processing, and this avoids reimplementing horned-owl's RDF-to-OWL mapping.

- **Future optimization**: horned-owl's `OntologyParser::new(build, Vec<PosTriple<A>>, config)` accepts pre-built triples, but the `oxrdf::Triple` to `PosTriple` conversion (including vocabulary substitution) is currently private. An upstream PR to expose this would allow direct triple-passing without the serialize/reparse round-trip.

### Unix niceties

- Restore default SIGPIPE handler (clean pipe-to-head behavior)
- Raise file descriptor soft limit toward hard limit (external sort opens many files)
- Benchmark mode: per-stage `Instant::elapsed()` + peak RSS via `libc::getrusage()`

### Error handling

`anyhow::Result` at application boundaries. `thiserror` for typed errors in internal modules. No `unwrap()` on fallible operations in library code.

---

## Module Structure

```
strix/
  Cargo.toml
  src/
    main.rs                     # Entry point, logging setup, subcommand dispatch
    cli.rs                      # Clap derive structs, MemorySize type
    lib.rs                      # Public API surface

    rdf/
      mod.rs
      input.rs                  # Format/compression detection, input discovery
      parser.rs                 # Streaming RDF parsing via oxrdfio

    dict/
      mod.rs
      encoding.rs               # Term -> ID (forward map)
      decoding.rs               # ID -> Term (reverse map)
      well_known.rs             # Reserved IDs for rdf:type, owl:Class, etc.

    owl/
      mod.rs
      parse.rs                  # OWL ontology loading via horned-owl (separate file or buffered TBox triples)
      normalize.rs              # Axiom normalization (equivalentClass -> mutual subClassOf, etc.)
      closure.rs                # Schema closure (transitive subClassOf*, subPropertyOf*)

    store/
      mod.rs
      relation.rs               # Relation trait and predicate-partitioned relation types
      segment.rs                # Immutable sorted segments on disk
      manifest.rs               # Segment manifests and metadata
      merge.rs                  # K-way merge of sorted segments
      scan.rs                   # Sequential and prefix scans
      sort.rs                   # External merge sort with memory budget
      delta.rs                  # Delta relation for semi-naive iteration

    compile/
      mod.rs
      ir.rs                     # Typed rule IR (shared by OWL RL and SWRL)
      specialize.rs             # Template specialization against TBox
      stratify.rs               # Rule dependency analysis and stratification
      plan.rs                   # Physical join plans from compiled rules
      index_selection.rs        # Derive required sort orders from compiled plans

    engine/
      mod.rs
      driver.rs                 # Semi-naive evaluation loop
      join.rs                   # Merge join and hash join operators
      sameas.rs                 # Union-find for owl:sameAs canonicalization
      eval.rs                   # Rule evaluation (specialized fast paths + generic IR)

    swrl/
      mod.rs
      parse.rs                  # SWRL parsing via horned-owl
      validate.rs               # DL-safety checking, built-in validation
      lower.rs                  # SWRL -> rule IR lowering

    output/
      mod.rs
      serialize.rs              # Decode IDs, write RDF output
      report.rs                 # run-report.json generation

    bench/
      mod.rs                    # Benchmark instrumentation (timing, RSS)
```

---

## Storage Design

The storage layer implements Proposal 1's sorted-run relation store from the beginning. This is the right foundation for a predicate-driven rule workload and avoids a later migration from mmap-over-monolithic-triples.

### Predicate-partitioned relations

The engine does not use a monolithic triple table. Relations are shaped by purpose:

| Relation | Shape | Primary sort | Secondary sort |
|----------|-------|-------------|----------------|
| `type_assertion` | `(instance, class)` | `(class, instance)` | `(instance, class)` |
| `property_assertion` | `(subject, predicate, object)` | `(predicate, subject, object)` | `(predicate, object, subject)` |
| `subclass` | `(sub, super)` | `(sub, super)` | `(super, sub)` |
| `subproperty` | `(sub, super)` | `(sub, super)` | `(super, sub)` |
| `domain` | `(property, class)` | `(property, class)` | — |
| `range` | `(property, class)` | `(property, class)` | — |
| `inverse` | `(p1, p2)` | `(p1, p2)` | — |
| `property_chain` | `(chain_id, property, position)` | `(chain_id, position)` | — |

Sort orders are derived from compiled rule access patterns. The initial implementation starts with the orders listed above and refines as rules are implemented. Only indexes that compiled plans actually require are built.

### Sorted-run segments

Each relation is stored as a sequence of immutable sorted segments on disk:

- **Append**: Unsorted facts are buffered in memory up to a configurable threshold, then sorted (using rayon `par_sort_unstable`) and flushed as a new segment file.
- **Spill**: When memory budget is exceeded, flush the current buffer as a compressed segment (zstd level 1, following hdtc's approach for speed).
- **Merge**: K-way merge of segments into a new immutable segment. Duplicate elimination is a natural byproduct of the merge. Use a binary heap for ≤16 segments, parallel merge tree for more.
- **Scan**: Sequential scan over a segment or merged view of all segments.
- **Prefix scan**: Seek to a key prefix and scan forward — the primary access pattern for merge joins.
- **Set difference**: Merge-based difference check against existing segments (for delta dedup in semi-naive iteration).

### Fact provenance and visible views

For ABox relations, Phase 1 keeps asserted and derived facts separate:

- **Asserted segments**: immutable base facts loaded from input.
- **Derived segments**: facts produced by rule evaluation.
- **Known view**: a merged read view over asserted + derived segments.

This separation makes `--emit inferred` well-defined without adding per-rule provenance:

- `--emit closure`: serialize the known view.
- `--emit inferred`: serialize only derived segments.
- Delta difference checks compare candidates against the known view, not just derived segments.

TBox/schema relations are treated as asserted inputs plus precomputed closures; early phases do not track provenance within schema closure itself.

### External merge sort

Adopt hdtc's external sort pattern:
- `push()` items into an in-memory buffer
- When buffer exceeds memory budget, `par_sort_unstable` and flush as zstd-compressed segment
- Final merge via binary heap or parallel merge tree depending on segment count
- Raise file descriptor limit at startup to support high fan-in merges

### Bloom filter optimization (Phase 2+)

Add Bloom filters as a tunable fast-path for duplicate rejection during semi-naive iteration. This complements merge-based dedup — the Bloom filter avoids most disk lookups, the merge handles the rest with certainty.

---

## Reasoning Architecture

### Rule IR

A typed internal representation serves as the common compilation target. Every rule — whether from OWL RL axiom compilation or SWRL lowering — is represented in this IR.

```
Rule {
    id: RuleId,
    family: RuleFamily,        // RDFS, OwlClass, OwlProperty, OwlEquality, Swrl, ...
    strata: StratumId,
    body: Vec<BodyAtom>,       // relation + binding pattern
    head: HeadAtom,            // relation + binding pattern
    priority: Priority,        // for join ordering hints
}

BodyAtom {
    relation: RelationId,
    bindings: Vec<Binding>,    // Variable(id), Constant(term_id), Wildcard
    source: ScanSpec,          // which sort order to use
}
```

### Dual execution mode

- **Specialized fast paths** for high-frequency OWL RL patterns: subclass propagation, domain/range, inverse, type inference. These are hand-written operators that bypass the generic IR interpreter for the common cases.
- **Generic compiled plans** for everything else: multi-atom SWRL bodies, property chains, complex OWL RL fragments. These execute through the IR interpreter using the join plans produced by the compiler.

The specialized paths and generic path share the same relation store and delta management.

### Semi-naive evaluation

```
for each stratum:
    seed delta from known facts relevant to this stratum
    loop:
        next_delta = empty
        for each rule in stratum:
            evaluate rule with delta in at least one body position, all in others
            collect candidate head facts into next_delta buffer
        sort, dedup, and difference-check next_delta against known facts
        if next_delta is empty: break (fixpoint reached)
        merge next_delta into derived segments
        delta = next_delta
```

### Stratification

**Phase 1 operational strata:**

1. Schema closure (subClassOf*, subPropertyOf*)
2. RDFS-level ABox rules (domain, range, subclass/subproperty propagation)

**Phase 2+ execution model:**

`owl:sameAs` is not a one-shot terminal stratum. Equality needs to affect subsequent rule evaluation, so the engine uses an outer fixpoint:

1. Compute schema closure from the normalized ontology.
2. Run non-equality RDFS/OWL RL strata to fixpoint over the current canonicalized facts.
3. Evaluate equality-producing rules and add new `sameAs` pairs to the equality relation.
4. Union newly connected IDs, choose canonical representatives, and rewrite affected ABox facts before they become visible to the next outer round.
5. Repeat steps 2-4 until there are no new non-equality facts and no new unions.
6. Run inconsistency detection on the final canonicalized closure.

Phase 3 SWRL rules execute over canonicalized facts and participate in the same outer fixpoint if they can produce equality.

### Diagnostics (run-report.json)

Every run produces a structured report:

```json
{
  "version": 1,
  "input": { "triples": 1000000, "tbox_axioms": 50000, "swrl_rules": 12 },
  "rules": {
    "supported": ["rdfs-subclass", "rdfs-domain", "owl-inverse", ...],
    "unsupported_encountered": ["owl:hasKey (not implemented)"],
    "swrl_rejected": [{ "rule": "...", "reason": "not DL-safe: unbound variable ?x" }]
  },
  "reasoning": {
    "strata": [
      { "name": "schema-closure", "iterations": 3, "inferred": 12000, "time_ms": 450 },
      ...
    ],
    "total_inferred": 250000,
    "total_iterations": 15,
    "fixpoint_reached": true
  },
  "sameas": { "classes": 500, "max_class_size": 12, "rewrites": 3400 },
  "inconsistencies": [],
  "peak_rss_bytes": 2147483648,
  "wall_time_ms": 45000
}
```

---

## CLI Design

```
strix reason \
  data.nt.gz more-data/ \
  --ontology ontology.owl \
  --output inferred.nt \
  --work-dir ./work \
  --memory-budget 8G \
  --report run-report.json

strix reason \
  /path/to/rdf-directory/ \
  --output inferred.nt \
  --emit closure \
  --report run-report.json
```

### Subcommands

| Command | Purpose |
|---------|---------|
| `reason` | Full pipeline: ingest, compile, materialize, export |
| `ingest` | Parse and encode RDF to working relations (for debugging/testing) |
| `compile` | Load ontology and emit compiled rule plan (for debugging/testing) |
| `export` | Decode working relations back to RDF (for debugging/testing) |

`reason` is the primary command. `ingest`, `compile`, and `export` are useful development subcommands, but they are optional for Phase 1 and should not block the end-to-end path.

### `reason` arguments

The CLI below is the long-term target surface. For implementation, freeze a smaller stable subset in Phase 1 and add the deferred flags only when their backing behavior exists.

**Required in Phase 1:**
- `<path>...`: Input RDF datasets (files or directories, any supported format; at least one required)
- `--output <path>` / `-o`: Output file for inferred triples

**Available in Phase 1:**
- `--ontology <path>` / `-O`: Separate ontology file. If omitted, extract the Phase 1 schema subset from data.
- `--extract-ontology`: Merge extracted schema triples from data even when `--ontology` is provided.
- `--emit inferred|closure`: Output inferred-only (default) or full closure. Implemented via asserted/derived segment separation.
- `--output-format ntriples`: Output format (Phase 1 only)
- `--work-dir <path>` / `-w`: Directory for working files (default: temp)
- `--memory-budget <size>` / `-m`: Memory budget (default: 4G)
- `--report <path>`: Write run-report.json to this path
- `--max-iterations <n>`: Safety limit on fixpoint iterations

**Global flags** (on all subcommands):
- `--verbose` / `-v`: Increase log verbosity (repeatable)
- `--quiet` / `-q`: Errors only
- `--benchmark`: Emit per-stage timing and peak RSS

**Deferred until later phases:**
- `--rules <path>`: Phase 3 (SWRL)
- `--output-format turtle`: Phase 2+
- `--inconsistency-mode report|halt`: Phase 2
- `--threads <n>` / `-t`: Phase 4
- `--checkpoint`: Phase 4
- `--resume`: Phase 4

---

## Dependency Choices

| Crate | Purpose |
|-------|---------|
| `clap` 4 | CLI with derive macros |
| `oxrdfio` | Multi-format RDF parsing |
| `oxrdf` | RDF term model |
| `horned-owl` | OWL 2 ontology + SWRL parsing |
| `tracing` | Structured logging |
| `tracing-subscriber` | Log formatting and filtering |
| `anyhow` | Application-level error handling |
| `thiserror` | Typed errors in library modules |
| `rayon` | Parallel sorting |
| `crossbeam-channel` | Bounded channels for pipeline backpressure |
| `zstd` | Compression for temporary segment files |
| `flate2` | Gzip decompression for input |
| `bzip2` | Bzip2 decompression for input |
| `xz2` | XZ decompression for input |
| `tempfile` | Temporary directories for working files |
| `walkdir` | Recursive input discovery |
| `hashbrown` | Fast hash maps for dictionary and rule lookup |
| `serde` + `serde_json` | run-report.json serialization |
| `libc` | SIGPIPE, rlimit, getrusage |
| `memmap2` | Memory-mapped reading of input files (not for core store) |

Not using a general-purpose embedded database for ABox storage. The sorted-run store is purpose-built.

---

## Phased Roadmap

### Phase 1: Foundation + RDFS Reasoning

**Goal**: End-to-end working reasoner for RDFS, validating the entire pipeline from ingest through export.

**Deliverables:**

1. **Project scaffold**
   - Cargo project, module structure, CI
   - CLI with `reason` subcommand (clap derive)
   - Treat `ingest`, `compile`, and `export` as optional dev subcommands; do not block Phase 1 on them
   - Logging setup (tracing)
   - SIGPIPE, FD limit, benchmark instrumentation

2. **RDF ingest**
   - Streaming RDF parsing via oxrdfio (N-Triples first, then Turtle/RDF-XML)
   - Format and compression auto-detection
   - Dictionary encoding: term -> 64-bit ID, forward + reverse maps
   - Reserved well-known term IDs

3. **Sorted-run relation store**
   - Relation trait with predicate-partitioned implementations
   - External merge sort with memory budget
   - Immutable segment files (zstd compressed)
   - K-way merge
   - Sequential and prefix scans
   - Set difference for delta dedup
   - Segment manifests

4. **Ingest pipeline**
   - Parse RDF stream via oxrdfio -> dictionary encode -> classify TBox vs ABox
   - Write ABox facts into `type_assertion` and `property_assertion` relations
   - If `--ontology` is omitted: extract the Phase 1 schema subset from data by default
   - If `--extract-ontology`: merge the extracted schema subset with any separate ontology file
   - If `--ontology`: load separate file via horned-owl directly
   - For extracted TBox triples: serialize to N-Triples buffer, pass to horned-owl's `from_bufread` for OWL axiom construction (leverages horned-owl's RDF-to-OWL mapping rather than reimplementing it)
   - Unsupported OWL constructs encountered in extracted data are reported, not silently dropped

5. **Schema compiler (RDFS subset)**
   - Walk horned-owl's axiom model to extract subClassOf, subPropertyOf, domain, range
   - Compute transitive closures (subClassOf*, subPropertyOf*)
   - Template specialization: subClassOf(A,B) + type(x,A) -> type(x,B) becomes type(x,A) -> type(x,B)
   - Produce compiled rule set in the typed IR

6. **Rule IR and semi-naive driver**
   - Define rule IR types
   - Implement stratified semi-naive evaluation loop
   - Delta management: buffer -> sort -> flush -> merge -> difference check
   - Keep asserted and derived ABox segments separate so `--emit inferred` is exact
   - Single-threaded execution

7. **Specialized RDFS operators**
   - subClassOf type propagation
   - subPropertyOf triple propagation
   - domain inference
   - range inference
   - rdf:type propagation

8. **Export**
   - Decode IDs -> RDF terms
   - Write N-Triples output
   - Emit inferred-only from derived segments or full closure from the known view
   - Basic run-report.json

**Non-goals for Phase 1**: `owl:sameAs`, SWRL, checkpoint/resume, parallel rule evaluation, Turtle output, and full RDF-based OWL extraction via blank-node/list closure.

**Success criterion**: Correct RDFS inference on standard test cases. Ingest and reason over a multi-million triple dataset within the memory budget. The `reason` command is stable for the Phase 1 flag subset listed above.

### Phase 2: Full OWL RL

**Goal**: Complete OWL 2 RL entailment rule coverage.

**Deliverables:**

1. **Full OWL ontology parsing**
   - Parse OWL via horned-owl (OWL/XML, Functional Syntax, RDF-based) — building on Phase 1's loading paths
   - Walk horned-owl's full axiom model (beyond RDFS subset)
   - Normalize: equivalentClass -> mutual subClassOf, equivalentProperty -> mutual subPropertyOf

2. **OWL RL class axiom rules**
   - intersectionOf (type membership when member of all conjuncts)
   - unionOf (type membership from any disjunct)
   - hasValue restrictions
   - someValuesFrom / allValuesFrom restrictions
   - complementOf / disjointWith (inconsistency detection)

3. **OWL RL property axiom rules**
   - equivalentProperty
   - inverseOf
   - TransitiveProperty
   - SymmetricProperty
   - propertyChainAxiom
   - FunctionalProperty / InverseFunctionalProperty
   - propertyDisjointWith

4. **owl:sameAs handling**
   - Union-find equivalence classes
   - Canonical representative selection
   - Fact rewriting through canonical IDs
   - Integrate into the outer fixpoint, not as a terminal one-pass stratum

5. **Bloom filter optimization**
   - Tunable Bloom filter for fast duplicate rejection
   - Sized from memory budget allocation

6. **Full rule stratification**
   - All 7 strata operational
   - Dependency analysis across rule families

7. **Inconsistency reporting**
   - Detect and report disjointWith violations, complementOf contradictions
   - `--inconsistency-mode report|halt`

8. **Complete run-report.json**
   - Supported rule coverage
   - Unsupported constructs encountered
   - Per-stratum statistics

**Success criterion**: Correct results on OWL 2 RL conformance tests. Structured run report distinguishes supported, unsupported, and inconsistent outcomes.

### Phase 3: SWRL + Equality Refinement

**Goal**: DL-safe SWRL execution through the same rule IR.

**Deliverables:**

1. **SWRL parsing and validation**
   - Parse SWRL rules via horned-owl
   - DL-safety checking: all variables must bind to named individuals
   - Supported built-in validation (start with comparison only, extend as needed)
   - Reject unsafe rules with clear error messages in run-report

2. **SWRL -> IR lowering**
   - Translate DL-safe SWRL rules into the typed rule IR
   - Generate join plans for multi-atom SWRL bodies
   - DL-safety enforced by filtering against known named individuals set

3. **Generic IR execution**
   - Multi-atom body evaluation through the IR interpreter
   - Join order selection based on relation size estimates
   - Built-in atom evaluation (comparison operators)

4. **Equality refinement**
   - sameAs-aware join operators
   - Canonical ID rewriting integrated with SWRL evaluation
   - Output options: emit sameAs pairs, canonical-only with mapping file

**Success criterion**: Accepted DL-safe SWRL rules execute correctly. Rejected rules fail explicitly with reasons in the run report. sameAs does not cause quadratic blowup.

### Phase 4: Performance + Production Hardening

**Goal**: Competitive throughput. Production-quality diagnostics and resilience.

**Deliverables:**

1. **Parallelism**
   - Multi-threaded rule evaluation for independent rules within a stratum
   - Parallel disk I/O for scans across different relations
   - Concurrent delta buffer with lock-free append

2. **Join planning**
   - Selectivity estimation from relation statistics
   - Adaptive join order based on actual cardinalities
   - Index selection refinement: derive minimal required sort orders from the full compiled plan

3. **HDT ingest adapter**
   - Read HDT as an input format (dictionary + triples)
   - Convert HDT dictionary entries to internal IDs
   - Stream HDT triples into the relation store
   - HDT does NOT replace the internal store — it is purely an ingest path

4. **Checkpoint / resume**
   - Write checkpoint after each stratum completes
   - Resume from last completed stratum

5. **Memory budget enforcement**
   - Allocate budget across: dictionary, relation buffers, Bloom filter, TBox/rules, delta buffers
   - Back-pressure when approaching limit
   - Phased budget allocation (following hdtc's pattern of temporally disjoint phases sharing budget)

6. **Benchmarking**
   - LUBM and UOBM benchmark suites
   - At least one biomedical ontology workload (SNOMED CT or Gene Ontology)
   - Measure: triples/sec ingested, triples/sec inferred, peak RSS, disk footprint, iterations to fixpoint

**Success criterion**: No quadratic sameAs explosion. Broad OWL RL coverage. Throughput competitive for a single-node batch reasoner. Benchmarked on standard suites.

---

## Testing Strategy

### Layers

| Layer | Scope | Examples |
|-------|-------|---------|
| Unit | Individual operators | Dictionary encode/decode roundtrip, external sort correctness, merge join correctness, union-find operations |
| Golden | Compiler output | Ontology normalization produces expected rule IR, SWRL lowering produces expected plans |
| Integration | End-to-end pipeline | Small RDF + ontology -> expected inferred triples |
| Conformance | OWL 2 RL spec | W3C OWL RL test suite (adapted for batch materialization) |
| SWRL | Acceptance/rejection | DL-safe rules execute correctly, non-DL-safe rules rejected with reason |
| Differential | Cross-reference | Compare output against a trusted reference reasoner (e.g., RDFox) on datasets that fit in RAM |
| Scale | Memory + throughput | LUBM at increasing sizes under strict memory cap |

### Benchmark metrics

- Triples per second ingested
- Triples per second inferred
- Peak RSS
- Disk footprint by stage
- Iteration count to fixpoint
- sameAs class counts and rewrite rates
- Per-stratum wall time

---

## Insights from hdtc

The hdtc codebase demonstrates several patterns directly applicable to this project:

**External merge sort with memory budgeting** — hdtc's `ExternalSorter` pattern (push items, spill when budget exceeded, k-way merge at end) maps directly to the sorted-run segment store. Adopt the zstd-level-1 compression for temp segments and the adaptive merge strategy (heap for ≤16 segments, parallel tree for more).

**Phased memory budget allocation** — hdtc allocates different budget fractions to temporally disjoint pipeline stages. The same pattern applies here: ingest and schema compilation can share budget that is later reclaimed for reasoning. Don't try to size everything for concurrent peak — budget by phase.

**Bounded channels for backpressure** — If the ingest pipeline becomes multi-stage (parse -> encode -> classify -> write), use crossbeam bounded channels to prevent memory blowup. This matters when parsing is faster than disk writes.

**Arena allocation for batch processing** — hdtc uses bumpalo arenas for batch vocabulary building, avoiding per-item allocation overhead. Consider the same for batch dictionary encoding during ingest.

**Benchmark instrumentation as a first-class feature** — `--benchmark` with per-stage timing and RSS tracking is invaluable for performance work. Build this in from Phase 1, not as an afterthought.

Patterns from hdtc that do NOT directly apply:
- hdtc's 6-stage pipeline is specific to HDT construction; the reasoning pipeline has different stages
- hdtc's parallel N-Triples chunking is an optimization worth deferring — sequential streaming parsing is sufficient for Phase 1
- hdtc's PFC dictionary compression is HDT-specific; the reasoner's dictionary has different access patterns (frequent random-access lookups during export, not sequential encoding)
