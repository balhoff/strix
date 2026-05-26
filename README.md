# Strix

> [!WARNING]
> Strix is under active development and inferences are still being validated.

Forward-chaining OWL 2 RL reasoner for RDF datasets.

Strix reads RDF data and an OWL ontology, materializes all entailed triples
using OWL 2 RL rules, and writes the results as N-Triples. It uses a
semi-naive fixpoint evaluation strategy with disk-backed relations to bound
memory usage.

## Usage

```
strix reason <DATA>... --output <PATH> [OPTIONS]
```

### Required arguments

| Argument | Description |
|---|---|
| `<DATA>...` | One or more RDF input files or directories |
| `--output`, `-o` | Output file for inferred triples |

### Options

| Option | Default | Description |
|---|---|---|
| `--ontology`, `-O` | none | Separate ontology file or directory (`.ofn`, `.owx`, `.owl`, `.rdf`, `.ttl`, `.nt`) |
| `--extract-ontology` | off | Also extract schema axioms from the data files |
| `--emit` | `inferred` | `inferred` (new triples only) or `closure` (all triples) |
| `--output-format` | `ntriples` | Output serialization format |
| `--memory-budget`, `-m` | `4G` | Memory budget (e.g. `4G`, `512M`, `2048K`) |
| `--work-dir`, `-w` | system temp | Directory for intermediate disk-backed relations |
| `--report` | none | Write a JSON run report to this path |
| `--max-iterations` | none | Safety cap on fixpoint iterations |
| `--inconsistency-mode` | `report` | `report` (log warnings) or `halt` (return error) |
| `--ignore-annotation-axioms` | off | Skip annotation property schema axioms |
| `--verbose`, `-v` | off | Increase log verbosity (`-v` debug, `-vv` trace) |
| `--quiet`, `-q` | off | Suppress all output except errors |
| `--benchmark` | off | Include peak RSS in the run report |

### Input formats

**Data** (RDF ABox): N-Triples (`.nt`), Turtle (`.ttl`). Gzip (`.gz`) and
bzip2 (`.bz2`) compression are detected automatically. Directories are
traversed recursively.

**Ontology** (TBox): OWL Functional Syntax (`.ofn`), OWL/XML (`.owx`),
RDF/XML (`.owl`, `.rdf`), N-Triples (`.nt`), Turtle (`.ttl`). Gzip
compression supported. Directories traversed recursively.

### Examples

```sh
# Reason over data with a separate ontology
strix reason data.nt --ontology ontology.ofn --output inferred.nt

# Full closure output with a memory budget
strix reason data.nt.gz --ontology schema.owl --emit closure -m 8G -o full.nt

# Extract schema from data (no separate ontology file)
strix reason data.ttl --extract-ontology --output inferred.nt

# Write a JSON run report
strix reason data.nt -O ontology.ofn -o inferred.nt --report report.json
```

## Supported OWL 2 RL constructs

### Axiom types

| Axiom | Status |
|---|---|
| SubClassOf | supported |
| EquivalentClasses | supported (decomposed to mutual SubClassOf) |
| DisjointClasses | supported (inconsistency detection) |
| DisjointUnion | supported (union decomposed to SubClassOf, pairwise disjointness checked) |
| SubObjectPropertyOf | supported (including property chains) |
| EquivalentObjectProperties | supported (decomposed to mutual SubPropertyOf) |
| DisjointObjectProperties | supported (inconsistency detection) |
| InverseObjectProperties | supported |
| FunctionalObjectProperty | supported (equality via owl:sameAs) |
| InverseFunctionalObjectProperty | supported (equality via owl:sameAs) |
| SymmetricObjectProperty | supported |
| IrreflexiveObjectProperty | supported (inconsistency detection) |
| AsymmetricObjectProperty | supported (inconsistency detection) |
| TransitiveObjectProperty | supported |
| ObjectPropertyDomain | supported |
| ObjectPropertyRange | supported |
| SubDataPropertyOf | supported |
| EquivalentDataProperties | supported (decomposed to mutual SubPropertyOf) |
| DataPropertyDomain | supported |
| DataPropertyRange | supported (named datatypes and DataIntersectionOf) |
| FunctionalDataProperty | supported (equality via owl:sameAs) |
| DisjointDataProperties | supported (inconsistency detection) |
| DatatypeDefinition | supported (mutual subclass equivalence) |
| SubAnnotationPropertyOf | supported (unless `--ignore-annotation-axioms`) |
| AnnotationPropertyDomain/Range | supported (unless `--ignore-annotation-axioms`) |

### Class expressions (in SubClassOf / EquivalentClasses)

**In subclass (left) position:**

| Expression | Status |
|---|---|
| Named class | supported |
| ObjectIntersectionOf | supported (cls-int1: all conjuncts present implies superclass) |
| ObjectUnionOf | supported (decomposed: each disjunct becomes subclass of superclass) |
| ObjectSomeValuesFrom | supported (cls-svf1: property link to filler-typed individual implies superclass) |
| ObjectHasValue | supported (cls-hv1: property with specific value implies superclass) |
| ObjectOneOf | supported (each named individual gets the superclass type) |
| DataSomeValuesFrom | supported (typed literal with matching datatype implies superclass) |
| DataHasValue | supported (property with specific literal implies superclass) |

**In superclass (right) position:**

| Expression | Status |
|---|---|
| Named class | supported |
| ObjectIntersectionOf | supported (decomposed: subclass implies each conjunct) |
| ObjectAllValuesFrom | supported (cls-avf: all property successors get the filler type) |
| ObjectHasValue | supported (cls-hv2: class membership implies property assertion) |
| ObjectMaxCardinality 0 | supported (inconsistency detection) |
| ObjectMaxCardinality 1 | supported (equality via owl:sameAs) |
| ObjectComplementOf | supported (inconsistency detection) |
| DataAllValuesFrom | supported (all data property values get the filler datatype) |
| DataHasValue | supported (class membership implies data property assertion) |
| DataMaxCardinality 0 | supported (inconsistency detection) |
| DataMaxCardinality 1 | supported (equality via owl:sameAs) |

### Not yet implemented

| Construct | Notes |
|---|---|
| owl:imports | Not supported |

### Filtering

Strix suppresses materialization of trivially true type assertions
(`owl:Thing`, `owl:Nothing` as superclass targets) to avoid generating
useless triples that every individual satisfies by definition.

## Architecture

The reasoning pipeline has four stages:

1. **Ingest** -- Parse RDF data into a `FactStore` with asserted type and
   property relations backed by sorted, disk-spillable segments.
2. **Compile** -- Compute transitive closures of subclass/subproperty
   hierarchies and build indexed lookup tables for all OWL axioms.
3. **Materialize** -- Semi-naive fixpoint loop: each iteration computes
   candidate facts from delta relations, differences them against known
   facts, and feeds new deltas into the next round. Join-based rules
   (transitive properties, class restrictions) use in-memory indexes
   filtered to only the predicates and classes that participate in joins.
   An outer equality fixpoint wraps the inner loop: after convergence,
   equality-producing rules (FunctionalProperty, InverseFunctionalProperty,
   MaxCardinality 1, asserted owl:sameAs) are evaluated via a union-find.
   New equalities trigger fact expansion across equivalence classes and
   another inner fixpoint pass.
4. **Export** -- Serialize inferred (or full closure) triples as N-Triples.

Memory is bounded by a configurable budget. Relations spill to disk as
sorted segments when the in-memory buffer is full, and are merged via
streaming k-way merge iterators.
