//! OWL 2 RL conformance tests.
//!
//! Derived from W3C OWL 2 conformance test cases
//! (<https://www.w3.org/2009/11/owl-test/approved/profile-RL.rdf>),
//! adapted with neutral example domains. Each test is split into data
//! (ABox as N-Triples) and ontology (TBox as OWL Functional Syntax).
//! Tests for unimplemented engine features are `#[ignore]` and tagged
//! with the plan step that enables them.

use std::fs;
use std::path::Path;

// ─── Helpers ────────────────────────────────────────────────────────────────

fn write(path: &Path, content: &str) {
    fs::write(path, content).expect("test fixture should be written");
}

fn count_triples(ntriples: &str) -> usize {
    ntriples.lines().filter(|l| !l.trim().is_empty()).count()
}

fn reason(data: &str, ontology: &str) -> String {
    let dir = tempfile::TempDir::new().unwrap();
    let data_path = dir.path().join("data.nt");
    let ont_path = dir.path().join("ontology.ofn");
    let out_path = dir.path().join("inferred.nt");

    write(&data_path, data);
    write(&ont_path, ontology);

    strix::run([
        "strix",
        "reason",
        data_path.to_str().unwrap(),
        "--ontology",
        ont_path.to_str().unwrap(),
        "--output",
        out_path.to_str().unwrap(),
    ])
    .expect("reasoning run should succeed");

    fs::read_to_string(&out_path).expect("output should exist")
}

fn reason_rdfxml(data: &str, ontology: &str) -> String {
    let dir = tempfile::TempDir::new().unwrap();
    let data_path = dir.path().join("data.nt");
    let ont_path = dir.path().join("ontology.owl");
    let out_path = dir.path().join("inferred.nt");

    write(&data_path, data);
    write(&ont_path, ontology);

    strix::run([
        "strix",
        "reason",
        data_path.to_str().unwrap(),
        "--ontology",
        ont_path.to_str().unwrap(),
        "--output",
        out_path.to_str().unwrap(),
    ])
    .expect("reasoning run should succeed");

    fs::read_to_string(&out_path).expect("output should exist")
}

fn reason_expect_inconsistency(data: &str, ontology: &str) -> bool {
    let dir = tempfile::TempDir::new().unwrap();
    let data_path = dir.path().join("data.nt");
    let ont_path = dir.path().join("ontology.ofn");
    let out_path = dir.path().join("inferred.nt");
    let report_path = dir.path().join("report.json");

    write(&data_path, data);
    write(&ont_path, ontology);

    let _ = strix::run([
        "strix",
        "reason",
        data_path.to_str().unwrap(),
        "--ontology",
        ont_path.to_str().unwrap(),
        "--output",
        out_path.to_str().unwrap(),
        "--report",
        report_path.to_str().unwrap(),
    ]);

    if let Ok(report) = fs::read_to_string(&report_path) {
        report.contains("inconsisten")
    } else {
        false
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Positive Entailment Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Derived from W3C: New-Feature-ObjectPropertyChain-001
///
/// SubObjectPropertyOf(ObjectPropertyChain(:memberOf :locatedIn) :basedIn)
/// :agent1 :memberOf :org1 . :org1 :locatedIn :city1 .
/// ⊨ :agent1 :basedIn :city1 .
#[test]
#[ignore = "Step 5: property chains not yet implemented"]
fn w3c_object_property_chain_001() {
    let inferred = reason(
        "\
<http://example.org/agent1> <http://example.org/memberOf> <http://example.org/org1> .
<http://example.org/org1> <http://example.org/locatedIn> <http://example.org/city1> .
",
        "\
Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/ontology>
Declaration(ObjectProperty(:memberOf))
Declaration(ObjectProperty(:locatedIn))
Declaration(ObjectProperty(:basedIn))
SubObjectPropertyOf(ObjectPropertyChain(:memberOf :locatedIn) :basedIn)
)",
    );
    assert!(
        inferred.contains(
            "<http://example.org/agent1> <http://example.org/basedIn> <http://example.org/city1> ."
        ),
        "chain(memberOf, locatedIn) → basedIn should fire: {inferred}"
    );
    assert_eq!(count_triples(&inferred), 1, "exactly one chain inference: {inferred}");
}

/// Derived from W3C: New-Feature-ObjectPropertyChain-BJP-003
///
/// SubObjectPropertyOf(ObjectPropertyChain(:linksTo :extends) :linksTo)
/// :a :linksTo :b . :b :extends :c .
/// ⊨ :a :linksTo :c .
#[test]
#[ignore = "Step 5: property chains not yet implemented"]
fn w3c_object_property_chain_bjp_003() {
    let inferred = reason(
        "\
<http://example.org/a> <http://example.org/linksTo> <http://example.org/b> .
<http://example.org/b> <http://example.org/extends> <http://example.org/c> .
",
        "\
Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/ontology>
Declaration(ObjectProperty(:linksTo))
Declaration(ObjectProperty(:extends))
SubObjectPropertyOf(ObjectPropertyChain(:linksTo :extends) :linksTo)
)",
    );
    assert!(
        inferred.contains(
            "<http://example.org/a> <http://example.org/linksTo> <http://example.org/c> ."
        ),
        "chain(linksTo, extends) → linksTo should fire: {inferred}"
    );
}

/// Derived from W3C: New-Feature-ObjectPropertyChain-BJP-004 (Negative Entailment)
///
/// SubObjectPropertyOf(ObjectPropertyChain(:linksTo :extends) :linksTo)
/// :a :extends :b . :b :extends :c .
/// ⊭ :a :linksTo :c   (no :linksTo assertions in data, chain cannot fire)
#[test]
#[ignore = "Step 5: property chains not yet implemented"]
fn w3c_object_property_chain_bjp_004_negative() {
    let inferred = reason(
        "\
<http://example.org/a> <http://example.org/extends> <http://example.org/b> .
<http://example.org/b> <http://example.org/extends> <http://example.org/c> .
",
        "\
Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/ontology>
Declaration(ObjectProperty(:linksTo))
Declaration(ObjectProperty(:extends))
SubObjectPropertyOf(ObjectPropertyChain(:linksTo :extends) :linksTo)
)",
    );
    assert!(
        !inferred.contains("<http://example.org/a> <http://example.org/linksTo> <http://example.org/c>"),
        "chain(linksTo, extends)→linksTo must not fire on extends-only data: {inferred}"
    );
    assert_eq!(count_triples(&inferred), 0, "no inferences from extends-only data: {inferred}");
}

/// Derived from W3C: DisjointClasses-001
///
/// DisjointClasses(:Circle :Triangle), ClassAssertion(:Circle :shape1)
/// ⊨ :shape1 is not a Triangle (complementOf)
#[test]
#[ignore = "Step 7: inconsistency detection not yet implemented"]
fn w3c_disjoint_classes_001() {
    let inferred = reason(
        "\
<http://example.org/shape1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Circle> .
",
        "\
Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/ontology>
Declaration(Class(:Circle))
Declaration(Class(:Triangle))
DisjointClasses(:Circle :Triangle)
)",
    );
    assert!(
        !inferred.contains("<http://example.org/Triangle>"),
        "disjoint class should not be inferred: {inferred}"
    );
}

/// Derived from W3C: DisjointClasses-003
///
/// DisjointClasses(:Circle :Triangle :Square), ClassAssertion(:Circle :shape1)
/// ⊨ :shape1 is neither Triangle nor Square
#[test]
#[ignore = "Step 7: inconsistency detection not yet implemented"]
fn w3c_disjoint_classes_003() {
    let inferred = reason(
        "\
<http://example.org/shape1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Circle> .
",
        "\
Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/ontology>
Declaration(Class(:Circle))
Declaration(Class(:Triangle))
Declaration(Class(:Square))
DisjointClasses(:Circle :Triangle :Square)
)",
    );
    assert!(
        !inferred.contains("<http://example.org/Triangle>"),
        "disjoint class Triangle should not be inferred: {inferred}"
    );
    assert!(
        !inferred.contains("<http://example.org/Square>"),
        "disjoint class Square should not be inferred: {inferred}"
    );
}

/// Derived from W3C: New-Feature-DisjointObjectProperties-001
///
/// DisjointObjectProperties(:above :below)
/// :x :above :y . :x :below :z .
/// ⊨ DifferentIndividuals(:y :z)
#[test]
#[ignore = "Step 7: disjoint properties + differentFrom not yet implemented"]
fn w3c_disjoint_object_properties_001() {
    let inferred = reason(
        "\
<http://example.org/x> <http://example.org/above> <http://example.org/y> .
<http://example.org/x> <http://example.org/below> <http://example.org/z> .
",
        "\
Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/ontology>
Declaration(ObjectProperty(:above))
Declaration(ObjectProperty(:below))
DisjointObjectProperties(:above :below)
)",
    );
    // When differentFrom is implemented, verify y ≠ z
    let _ = inferred;
}

// ═══════════════════════════════════════════════════════════════════════════
// Inconsistency Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Derived from W3C: DisjointClasses-002
///
/// DisjointClasses(:Circle :Triangle)
/// :shape1 type Circle, :shape1 type Triangle → INCONSISTENT
#[test]
#[ignore = "Step 7: inconsistency detection not yet implemented"]
fn w3c_disjoint_classes_002_inconsistency() {
    let is_inconsistent = reason_expect_inconsistency(
        "\
<http://example.org/shape1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Circle> .
<http://example.org/shape1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Triangle> .
",
        "\
Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/ontology>
Declaration(Class(:Circle))
Declaration(Class(:Triangle))
DisjointClasses(:Circle :Triangle)
)",
    );
    assert!(
        is_inconsistent,
        "Circle disjointWith Triangle + shape1 in both should be inconsistent"
    );
}

/// Derived from W3C: New-Feature-AsymmetricProperty-001
///
/// AsymmetricObjectProperty(:contains)
/// :box1 :contains :item1 . :item1 :contains :box1 .
/// → INCONSISTENT
#[test]
#[ignore = "AsymmetricObjectProperty deferred (not in Phase 2 scope)"]
fn w3c_asymmetric_property_001_inconsistency() {
    let is_inconsistent = reason_expect_inconsistency(
        "\
<http://example.org/box1> <http://example.org/contains> <http://example.org/item1> .
<http://example.org/item1> <http://example.org/contains> <http://example.org/box1> .
",
        "\
Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/ontology>
Declaration(ObjectProperty(:contains))
AsymmetricObjectProperty(:contains)
)",
    );
    assert!(
        is_inconsistent,
        "asymmetric property with both directions should be inconsistent"
    );
}

/// Derived from W3C: New-Feature-IrreflexiveProperty-001
///
/// IrreflexiveObjectProperty(:strictlyLargerThan)
/// :x :strictlyLargerThan :x .
/// → INCONSISTENT
#[test]
#[ignore = "IrreflexiveObjectProperty deferred (not in Phase 2 scope)"]
fn w3c_irreflexive_property_001_inconsistency() {
    let is_inconsistent = reason_expect_inconsistency(
        "\
<http://example.org/x> <http://example.org/strictlyLargerThan> <http://example.org/x> .
",
        "\
Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/ontology>
Declaration(ObjectProperty(:strictlyLargerThan))
IrreflexiveObjectProperty(:strictlyLargerThan)
)",
    );
    assert!(
        is_inconsistent,
        "irreflexive property with self-link should be inconsistent"
    );
}

/// Derived from W3C: New-Feature-NegativeObjectPropertyAssertion-001
///
/// NegativeObjectPropertyAssertion(:produces :factory1 :widget1)
/// ObjectPropertyAssertion(:produces :factory1 :widget1)
/// → INCONSISTENT
#[test]
#[ignore = "NegativeObjectPropertyAssertion deferred"]
fn w3c_negative_object_property_assertion_001_inconsistency() {
    let is_inconsistent = reason_expect_inconsistency(
        "\
<http://example.org/factory1> <http://example.org/produces> <http://example.org/widget1> .
",
        "\
Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/ontology>
Declaration(ObjectProperty(:produces))
NegativeObjectPropertyAssertion(:produces :factory1 :widget1)
ObjectPropertyAssertion(:produces :factory1 :widget1)
)",
    );
    assert!(
        is_inconsistent,
        "contradictory positive and negative assertion should be inconsistent"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Consistency Tests (ontology should parse and reason without error)
// ═══════════════════════════════════════════════════════════════════════════

/// Derived from W3C: owl2-rl-valid-oneof
///
/// ObjectOneOf in subclass position — valid RL construct.
#[test]
fn w3c_valid_oneof_consistency() {
    let inferred = reason_rdfxml(
        "",
        "\
<?xml version=\"1.0\"?>
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"
    xmlns:rdfs=\"http://www.w3.org/2000/01/rdf-schema#\"
    xmlns:owl=\"http://www.w3.org/2002/07/owl#\">
  <owl:Ontology/>
  <owl:Class rdf:about=\"http://owl2.test/rules#Category\"/>
  <owl:NamedIndividual rdf:about=\"http://owl2.test/rules#item1\"/>
  <owl:NamedIndividual rdf:about=\"http://owl2.test/rules#item2\"/>
  <rdf:Description>
    <rdfs:subClassOf rdf:resource=\"http://owl2.test/rules#Category\"/>
    <owl:oneOf rdf:parseType=\"Collection\">
      <owl:NamedIndividual rdf:about=\"http://owl2.test/rules#item1\"/>
      <owl:NamedIndividual rdf:about=\"http://owl2.test/rules#item2\"/>
    </owl:oneOf>
  </rdf:Description>
</rdf:RDF>
",
    );
    // OneOf({item1, item2}) ⊆ Category → type(item1, Category), type(item2, Category)
    // At minimum, the ontology should parse without error.
    let _ = inferred;
}

/// Derived from W3C: owl2-rl-valid-rightside-allvaluesfrom
///
/// AllValuesFrom in superclass position — valid RL construct.
#[test]
fn w3c_valid_allvaluesfrom_consistency() {
    let inferred = reason_rdfxml(
        "",
        "\
<?xml version=\"1.0\"?>
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"
    xmlns:rdfs=\"http://www.w3.org/2000/01/rdf-schema#\"
    xmlns:owl=\"http://www.w3.org/2002/07/owl#\">
  <owl:Ontology/>
  <owl:Class rdf:about=\"http://owl2.test/rules#Container\">
    <rdfs:subClassOf>
      <owl:Restriction>
        <owl:onProperty>
          <owl:ObjectProperty rdf:about=\"http://owl2.test/rules#holds\"/>
        </owl:onProperty>
        <owl:allValuesFrom>
          <owl:Class rdf:about=\"http://owl2.test/rules#Item\"/>
        </owl:allValuesFrom>
      </owl:Restriction>
    </rdfs:subClassOf>
  </owl:Class>
</rdf:RDF>
",
    );
    assert_eq!(
        count_triples(&inferred),
        0,
        "no data means no inferences: {inferred}"
    );
}

/// Derived from W3C: owl2-rl-anonymous-individual
///
/// Anonymous individual with property assertion — should be accepted.
#[test]
fn w3c_anonymous_individual_consistency() {
    let inferred = reason_rdfxml(
        "",
        "\
<?xml version=\"1.0\"?>
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"
    xmlns:owl=\"http://www.w3.org/2002/07/owl#\"
    xmlns:ex=\"http://owl2.test/rules#\">
  <owl:Ontology/>
  <owl:ObjectProperty rdf:about=\"http://owl2.test/rules#relatedTo\"/>
  <owl:NamedIndividual rdf:about=\"http://owl2.test/rules#item1\"/>
  <owl:NamedIndividual>
    <ex:relatedTo rdf:resource=\"http://owl2.test/rules#item1\"/>
  </owl:NamedIndividual>
</rdf:RDF>
",
    );
    let _ = inferred;
}

// ═══════════════════════════════════════════════════════════════════════════
// Equality / sameAs Tests
// ═══════════════════════════════════════════════════════════════════════════

/// Derived from W3C: New-Feature-Keys-003
///
/// HasKey(:RegisteredVehicle () (:hasVIN))
/// :car1 hasVIN "ABC123", type RegisteredVehicle
/// :car2 hasVIN "ABC123", type RegisteredVehicle
/// ⊨ SameIndividual(:car1 :car2)
#[test]
#[ignore = "HasKey not in Phase 2 scope"]
fn w3c_keys_003_same_individual() {
    // HasKey requires dedicated engine support not in current plan
}

/// Derived from W3C: New-Feature-ObjectQCR-002
///
/// ObjectMaxCardinality(1, :hasOrigin, :DomesticSource) for :product1
/// :product1 :hasOrigin :src1, :product1 :hasOrigin :src2
/// :src2 type :DomesticSource, DifferentIndividuals(:src1 :src2)
/// ⊨ :src1 is not a DomesticSource (complementOf)
#[test]
#[ignore = "Step 6-7: max cardinality equality + inconsistency detection not yet implemented"]
fn w3c_qualified_max_cardinality_002() {
    let inferred = reason(
        "\
<http://example.org/product1> <http://example.org/hasOrigin> <http://example.org/src1> .
<http://example.org/product1> <http://example.org/hasOrigin> <http://example.org/src2> .
<http://example.org/src2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/DomesticSource> .
",
        "\
Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/ontology>
Declaration(ObjectProperty(:hasOrigin))
Declaration(Class(:DomesticSource))
Declaration(NamedIndividual(:src1))
Declaration(NamedIndividual(:src2))
SubClassOf(owl:Thing ObjectMaxCardinality(1 :hasOrigin :DomesticSource))
DifferentIndividuals(:src1 :src2)
)",
    );
    assert!(
        !inferred.contains("<http://example.org/src1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/DomesticSource>"),
        "src1 should not be inferred as DomesticSource: {inferred}"
    );
}
