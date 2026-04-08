use std::fs;
use std::io::Write;
use std::path::Path;

#[test]
fn reasons_with_separate_ontology() {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data = temp_dir.path().join("data.nt");
    let ontology = temp_dir.path().join("ontology.nt");
    let output = temp_dir.path().join("inferred.nt");
    let report = temp_dir.path().join("report.json");

    write(
        &data,
        "\
<http://example.com/alice> <http://example.com/knows> <http://example.com/bob> .
<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/Person> .
",
    );
    write(
        &ontology,
        "\
<http://example.com/FriendOf> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.com/Person> .
<http://example.com/knows> <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> <http://example.com/relatedTo> .
<http://example.com/relatedTo> <http://www.w3.org/2000/01/rdf-schema#domain> <http://example.com/SocialEntity> .
<http://example.com/relatedTo> <http://www.w3.org/2000/01/rdf-schema#range> <http://example.com/SocialEntity> .
<http://example.com/Person> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.com/Agent> .
",
    );

    strix::run([
        "strix",
        "reason",
        data.to_str().expect("data path should be UTF-8"),
        "--ontology",
        ontology.to_str().expect("ontology path should be UTF-8"),
        "--output",
        output.to_str().expect("output path should be UTF-8"),
        "--report",
        report.to_str().expect("report path should be UTF-8"),
    ])
    .expect("reasoning run should succeed");

    let inferred = fs::read_to_string(&output).expect("output should exist");
    assert!(inferred.contains(
        "<http://example.com/alice> <http://example.com/relatedTo> <http://example.com/bob> ."
    ));
    assert!(inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/Agent> ."));
    assert!(inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/SocialEntity> ."));
    assert!(inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/SocialEntity> ."));

    let report_json = fs::read_to_string(&report).expect("report should exist");
    assert!(report_json.contains("\"fixpoint_reached\": true"));
    assert!(report_json.contains("\"rdfs-subclass\""));
}

#[test]
fn reasons_with_rdf_xml_ontology() {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data = temp_dir.path().join("data.nt");
    let ontology = temp_dir.path().join("ontology.owl");
    let output = temp_dir.path().join("inferred.nt");

    write(
        &data,
        "\
<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/Person> .
",
    );
    write(
        &ontology,
        "\
<?xml version=\"1.0\"?>
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"
         xmlns:rdfs=\"http://www.w3.org/2000/01/rdf-schema#\"
         xmlns:owl=\"http://www.w3.org/2002/07/owl#\">
  <owl:Class rdf:about=\"http://example.com/Agent\" />
  <owl:Class rdf:about=\"http://example.com/Person\">
    <rdfs:subClassOf rdf:resource=\"http://example.com/Agent\" />
  </owl:Class>
</rdf:RDF>
",
    );

    strix::run([
        "strix",
        "reason",
        data.to_str().expect("data path should be UTF-8"),
        "--ontology",
        ontology.to_str().expect("ontology path should be UTF-8"),
        "--output",
        output.to_str().expect("output path should be UTF-8"),
    ])
    .expect("reasoning run should succeed");

    let inferred = fs::read_to_string(&output).expect("output should exist");
    assert!(inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/Agent> ."));
}

#[test]
fn reasons_with_turtle_ontology() {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data = temp_dir.path().join("data.nt");
    let ontology = temp_dir.path().join("ontology.ttl");
    let output = temp_dir.path().join("inferred.nt");

    write(
        &data,
        "\
<http://example.com/alice> <http://example.com/knows> <http://example.com/bob> .
",
    );
    write(
        &ontology,
        "\
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .

<http://example.com/relatedTo> a owl:ObjectProperty .
<http://example.com/knows> a owl:ObjectProperty ;
    rdfs:subPropertyOf <http://example.com/relatedTo> .
",
    );

    strix::run([
        "strix",
        "reason",
        data.to_str().expect("data path should be UTF-8"),
        "--ontology",
        ontology.to_str().expect("ontology path should be UTF-8"),
        "--output",
        output.to_str().expect("output path should be UTF-8"),
    ])
    .expect("reasoning run should succeed");

    let inferred = fs::read_to_string(&output).expect("output should exist");
    assert!(inferred.contains(
        "<http://example.com/alice> <http://example.com/relatedTo> <http://example.com/bob> ."
    ));
}

#[test]
fn reasons_with_owl_xml_ontology() {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data = temp_dir.path().join("data.nt");
    let ontology = temp_dir.path().join("ontology.owx");
    let output = temp_dir.path().join("inferred.nt");

    write(
        &data,
        "\
<http://example.com/alice> <http://example.com/knows> <http://example.com/bob> .
<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/Person> .
",
    );
    write(
        &ontology,
        "\
<?xml version=\"1.0\"?>
<Ontology xmlns=\"http://www.w3.org/2002/07/owl#\"
     xml:base=\"http://example.com/ontology\"
     xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"
     xmlns:xml=\"http://www.w3.org/XML/1998/namespace\"
     xmlns:xsd=\"http://www.w3.org/2001/XMLSchema#\"
     xmlns:rdfs=\"http://www.w3.org/2000/01/rdf-schema#\"
     ontologyIRI=\"http://example.com/ontology\">
    <Declaration>
        <Class IRI=\"http://example.com/Person\"/>
    </Declaration>
    <Declaration>
        <Class IRI=\"http://example.com/Agent\"/>
    </Declaration>
    <Declaration>
        <Class IRI=\"http://example.com/SocialEntity\"/>
    </Declaration>
    <Declaration>
        <ObjectProperty IRI=\"http://example.com/knows\"/>
    </Declaration>
    <Declaration>
        <ObjectProperty IRI=\"http://example.com/relatedTo\"/>
    </Declaration>
    <SubClassOf>
        <Class IRI=\"http://example.com/Person\"/>
        <Class IRI=\"http://example.com/Agent\"/>
    </SubClassOf>
    <SubObjectPropertyOf>
        <ObjectProperty IRI=\"http://example.com/knows\"/>
        <ObjectProperty IRI=\"http://example.com/relatedTo\"/>
    </SubObjectPropertyOf>
    <ObjectPropertyDomain>
        <ObjectProperty IRI=\"http://example.com/relatedTo\"/>
        <Class IRI=\"http://example.com/SocialEntity\"/>
    </ObjectPropertyDomain>
    <ObjectPropertyRange>
        <ObjectProperty IRI=\"http://example.com/relatedTo\"/>
        <Class IRI=\"http://example.com/SocialEntity\"/>
    </ObjectPropertyRange>
</Ontology>
",
    );

    strix::run([
        "strix",
        "reason",
        data.to_str().expect("data path should be UTF-8"),
        "--ontology",
        ontology.to_str().expect("ontology path should be UTF-8"),
        "--output",
        output.to_str().expect("output path should be UTF-8"),
    ])
    .expect("reasoning run should succeed");

    let inferred = fs::read_to_string(&output).expect("output should exist");
    assert!(inferred.contains(
        "<http://example.com/alice> <http://example.com/relatedTo> <http://example.com/bob> ."
    ));
    assert!(inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/Agent> ."));
    assert!(inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/SocialEntity> ."));
    assert!(inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/SocialEntity> ."));
}

#[test]
fn reasons_with_functional_syntax_ontology() {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data = temp_dir.path().join("data.nt");
    let ontology = temp_dir.path().join("ontology.ofn");
    let output = temp_dir.path().join("inferred.nt");

    write(
        &data,
        "\
<http://example.com/alice> <http://example.com/knows> <http://example.com/bob> .
<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/Person> .
",
    );
    write(
        &ontology,
        "\
Prefix(:=<http://example.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Prefix(rdf:=<http://www.w3.org/1999/02/22-rdf-syntax-ns#>)
Prefix(xml:=<http://www.w3.org/XML/1998/namespace>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Prefix(rdfs:=<http://www.w3.org/2000/01/rdf-schema#>)

Ontology(<http://example.com/ontology>

Declaration(Class(:Person))
Declaration(Class(:Agent))
Declaration(Class(:SocialEntity))
Declaration(ObjectProperty(:knows))
Declaration(ObjectProperty(:relatedTo))

SubClassOf(:Person :Agent)
SubObjectPropertyOf(:knows :relatedTo)
ObjectPropertyDomain(:relatedTo :SocialEntity)
ObjectPropertyRange(:relatedTo :SocialEntity)

)
",
    );

    strix::run([
        "strix",
        "reason",
        data.to_str().expect("data path should be UTF-8"),
        "--ontology",
        ontology.to_str().expect("ontology path should be UTF-8"),
        "--output",
        output.to_str().expect("output path should be UTF-8"),
    ])
    .expect("reasoning run should succeed");

    let inferred = fs::read_to_string(&output).expect("output should exist");
    assert!(inferred.contains(
        "<http://example.com/alice> <http://example.com/relatedTo> <http://example.com/bob> ."
    ));
    assert!(inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/Agent> ."));
    assert!(inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/SocialEntity> ."));
    assert!(inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/SocialEntity> ."));
}

#[test]
fn extracts_schema_from_data_and_can_emit_closure() {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let schema = temp_dir.path().join("schema.nt");
    let facts = temp_dir.path().join("facts.nt");
    let output = temp_dir.path().join("closure.nt");

    write(
        &schema,
        "\
<http://example.com/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.com/B> .
<http://example.com/p> <http://www.w3.org/2000/01/rdf-schema#domain> <http://example.com/A> .
",
    );
    write(
        &facts,
        "\
<http://example.com/x> <http://example.com/p> <http://example.com/y> .
",
    );

    strix::run([
        "strix",
        "reason",
        schema.to_str().expect("schema path should be UTF-8"),
        facts.to_str().expect("facts path should be UTF-8"),
        "--output",
        output.to_str().expect("output path should be UTF-8"),
        "--emit",
        "closure",
    ])
    .expect("reasoning run should succeed");

    let closure = fs::read_to_string(&output).expect("closure output should exist");
    assert!(
        closure.contains("<http://example.com/x> <http://example.com/p> <http://example.com/y> .")
    );
    assert!(closure.contains("<http://example.com/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> ."));
    assert!(closure.contains("<http://example.com/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> ."));
}

#[test]
fn preserves_non_schema_rdfs_assertions_in_data() {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data = temp_dir.path().join("data.nt");
    let ontology = temp_dir.path().join("ontology.nt");
    let output = temp_dir.path().join("closure.nt");

    write(
        &data,
        "\
<http://example.com/alice> <http://www.w3.org/2000/01/rdf-schema#label> \"Alice\" .
",
    );
    write(
        &ontology,
        "\
<http://www.w3.org/2000/01/rdf-schema#label> <http://www.w3.org/2000/01/rdf-schema#domain> <http://example.com/LabeledThing> .
",
    );

    strix::run([
        "strix",
        "reason",
        data.to_str().expect("data path should be UTF-8"),
        "--ontology",
        ontology.to_str().expect("ontology path should be UTF-8"),
        "--output",
        output.to_str().expect("output path should be UTF-8"),
        "--emit",
        "closure",
    ])
    .expect("reasoning run should succeed");

    let closure = fs::read_to_string(&output).expect("closure output should exist");
    assert!(closure.contains(
        "<http://example.com/alice> <http://www.w3.org/2000/01/rdf-schema#label> \"Alice\" ."
    ));
    assert!(closure.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/LabeledThing> ."));
}

#[test]
fn ignores_annotation_property_axioms_in_strict_mode() {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data = temp_dir.path().join("data.nt");
    let ontology = temp_dir.path().join("ontology.nt");
    let output = temp_dir.path().join("closure.nt");

    write(
        &data,
        "\
<http://example.com/alice> <http://www.w3.org/2000/01/rdf-schema#label> \"Alice\" .
",
    );
    write(
        &ontology,
        "\
<http://www.w3.org/2000/01/rdf-schema#label> <http://www.w3.org/2000/01/rdf-schema#domain> <http://example.com/LabeledThing> .
",
    );

    strix::run([
        "strix",
        "reason",
        data.to_str().expect("data path should be UTF-8"),
        "--ontology",
        ontology.to_str().expect("ontology path should be UTF-8"),
        "--ignore-annotation-axioms",
        "--output",
        output.to_str().expect("output path should be UTF-8"),
        "--emit",
        "closure",
    ])
    .expect("reasoning run should succeed");

    let closure = fs::read_to_string(&output).expect("closure output should exist");
    assert!(closure.contains(
        "<http://example.com/alice> <http://www.w3.org/2000/01/rdf-schema#label> \"Alice\" ."
    ));
    assert!(!closure.contains(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/LabeledThing> ."
    ));
}

#[test]
fn reports_and_ignores_incomplete_ontology_residue() {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data = temp_dir.path().join("data.nt");
    let ontology = temp_dir.path().join("ontology.nt");
    let output = temp_dir.path().join("inferred.nt");
    let report = temp_dir.path().join("report.json");

    write(
        &data,
        "\
<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> _:anon .
",
    );
    write(
        &ontology,
        "\
_:anon <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.com/Person> .
",
    );

    strix::run([
        "strix",
        "reason",
        data.to_str().expect("data path should be UTF-8"),
        "--ontology",
        ontology.to_str().expect("ontology path should be UTF-8"),
        "--output",
        output.to_str().expect("output path should be UTF-8"),
        "--report",
        report.to_str().expect("report path should be UTF-8"),
    ])
    .expect("reasoning run should succeed");

    let inferred = fs::read_to_string(&output).expect("output should exist");
    assert!(!inferred.contains("<http://example.com/Person>"));

    let report_json = fs::read_to_string(&report).expect("report should exist");
    assert!(report_json.contains("left unlowered horned-owl residue"));
    assert!(report_json.contains("ignored"));
}

#[test]
fn namespaces_blank_nodes_across_directory_inputs() {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data_dir = temp_dir.path().join("data");
    let nested_dir = data_dir.join("nested");
    let output = temp_dir.path().join("closure.nt");

    fs::create_dir_all(&nested_dir).expect("data dir should be created");
    write(
        &data_dir.join("a.nt"),
        "\
_:b0 <http://example.com/p> <http://example.com/o1> .
",
    );
    write(
        &nested_dir.join("b.nt"),
        "\
_:b0 <http://example.com/p> <http://example.com/o2> .
",
    );

    strix::run([
        "strix",
        "reason",
        data_dir.to_str().expect("data path should be UTF-8"),
        "--output",
        output.to_str().expect("output path should be UTF-8"),
        "--emit",
        "closure",
    ])
    .expect("reasoning run should succeed");

    let closure = fs::read_to_string(&output).expect("closure output should exist");
    assert!(closure.contains("_:f0_b0 <http://example.com/p> <http://example.com/o1> ."));
    assert!(closure.contains("_:f1_b0 <http://example.com/p> <http://example.com/o2> ."));
}

#[test]
fn loads_all_supported_rdf_formats_and_compressions_from_nested_directories() {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data_dir = temp_dir.path().join("data");
    let nested = data_dir.join("nested");
    let deep = nested.join("deep");
    let output = temp_dir.path().join("closure.nt");

    fs::create_dir_all(&deep).expect("nested data directories should be created");

    write_gzip(
        &data_dir.join("a.nt.gz"),
        "\
<http://example.com/s1> <http://example.com/p> <http://example.com/o1> .
",
    );
    write_xz(
        &nested.join("b.nq.xz"),
        "\
<http://example.com/s2> <http://example.com/p> <http://example.com/o2> <http://example.com/g> .
",
    );
    write_bzip2(
        &deep.join("c.trig.bz2"),
        "\
<http://example.com/g2> {
  <http://example.com/s3> <http://example.com/p> <http://example.com/o3> .
}
",
    );
    write(
        &data_dir.join("d.ttl"),
        "\
<http://example.com/s4> <http://example.com/p> <http://example.com/o4> .
",
    );
    write(
        &nested.join("e.rdf"),
        "\
<?xml version=\"1.0\"?>
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">
  <rdf:Description rdf:about=\"http://example.com/s5\">
    <p xmlns=\"http://example.com/\" rdf:resource=\"http://example.com/o5\" />
  </rdf:Description>
</rdf:RDF>
",
    );
    write(
        &deep.join("f.jsonld"),
        "\
{
  \"@context\": {
    \"p\": { \"@id\": \"http://example.com/p\", \"@type\": \"@id\" }
  },
  \"@id\": \"http://example.com/s6\",
  \"p\": \"http://example.com/o6\"
}
",
    );
    write(
        &deep.join("g.n3"),
        "\
<http://example.com/s7> <http://example.com/p> <http://example.com/o7> .
",
    );
    write(&data_dir.join("ignored.txt"), "not rdf");

    strix::run([
        "strix",
        "reason",
        data_dir.to_str().expect("data path should be UTF-8"),
        "--output",
        output.to_str().expect("output path should be UTF-8"),
        "--emit",
        "closure",
    ])
    .expect("reasoning run should succeed");

    let closure = fs::read_to_string(&output).expect("closure output should exist");
    assert!(
        closure
            .contains("<http://example.com/s1> <http://example.com/p> <http://example.com/o1> .")
    );
    assert!(
        closure
            .contains("<http://example.com/s2> <http://example.com/p> <http://example.com/o2> .")
    );
    assert!(
        closure
            .contains("<http://example.com/s3> <http://example.com/p> <http://example.com/o3> .")
    );
    assert!(
        closure
            .contains("<http://example.com/s4> <http://example.com/p> <http://example.com/o4> .")
    );
    assert!(
        closure
            .contains("<http://example.com/s5> <http://example.com/p> <http://example.com/o5> .")
    );
    assert!(
        closure
            .contains("<http://example.com/s6> <http://example.com/p> <http://example.com/o6> .")
    );
    assert!(
        closure
            .contains("<http://example.com/s7> <http://example.com/p> <http://example.com/o7> .")
    );
}

#[test]
fn escapes_control_characters_on_export() {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data = temp_dir.path().join("data.nt");
    let output = temp_dir.path().join("closure.nt");

    write(
        &data,
        "\
<http://example.com/s> <http://example.com/p> \"\\b\\f\\u0001\" .
",
    );

    strix::run([
        "strix",
        "reason",
        data.to_str().expect("data path should be UTF-8"),
        "--output",
        output.to_str().expect("output path should be UTF-8"),
        "--emit",
        "closure",
    ])
    .expect("reasoning run should succeed");

    let closure = fs::read_to_string(&output).expect("closure output should exist");
    assert!(closure.contains("\"\\b\\f\\u0001\""));
    assert!(!closure.as_bytes().contains(&0x08));
    assert!(!closure.as_bytes().contains(&0x0C));
    assert!(!closure.as_bytes().contains(&0x01));
}

#[test]
fn report_counts_only_abox_inferences() {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data = temp_dir.path().join("data.nt");
    let ontology = temp_dir.path().join("ontology.nt");
    let output = temp_dir.path().join("inferred.nt");
    let report = temp_dir.path().join("report.json");

    write(
        &data,
        "\
<http://example.com/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .
",
    );
    write(
        &ontology,
        "\
<http://example.com/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.com/B> .
<http://example.com/B> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.com/C> .
",
    );

    strix::run([
        "strix",
        "reason",
        data.to_str().expect("data path should be UTF-8"),
        "--ontology",
        ontology.to_str().expect("ontology path should be UTF-8"),
        "--output",
        output.to_str().expect("output path should be UTF-8"),
        "--report",
        report.to_str().expect("report path should be UTF-8"),
    ])
    .expect("reasoning run should succeed");

    let report_json = fs::read_to_string(&report).expect("report should exist");
    assert!(report_json.contains("\"total_inferred\": 2"));

    let schema_section = report_json
        .split("\"name\": \"schema-closure\"")
        .nth(1)
        .expect("schema stratum should be present")
        .split("}")
        .next()
        .expect("schema stratum should have a closing brace");
    assert!(schema_section.contains("\"inferred\": 0"));
}

#[test]
fn infers_inverse_property() {
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/knows> <http://example.com/bob> .\n",
        "\
Prefix(:=<http://example.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.com/ontology>
Declaration(ObjectProperty(:knows))
Declaration(ObjectProperty(:knownBy))
InverseObjectProperties(:knows :knownBy)
)",
    );
    assert!(inferred
        .contains("<http://example.com/bob> <http://example.com/knownBy> <http://example.com/alice> ."));
}

#[test]
fn infers_symmetric_property() {
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/friendOf> <http://example.com/bob> .\n",
        "\
Prefix(:=<http://example.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.com/ontology>
Declaration(ObjectProperty(:friendOf))
SymmetricObjectProperty(:friendOf)
)",
    );
    assert!(inferred
        .contains("<http://example.com/bob> <http://example.com/friendOf> <http://example.com/alice> ."));
}

#[test]
fn normalizes_equivalent_classes_to_mutual_subclass() {
    let inferred = reason(
        "\
<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/Person> .
<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/Human> .
",
        "\
Prefix(:=<http://example.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.com/ontology>
Declaration(Class(:Person))
Declaration(Class(:Human))
EquivalentClasses(:Person :Human)
)",
    );
    // alice is Person → also Human (via equivalence)
    assert!(inferred.contains(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/Human> ."
    ));
    // bob is Human → also Person (via equivalence)
    assert!(inferred.contains(
        "<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/Person> ."
    ));
}

#[test]
fn parses_property_chain_and_records_in_schema() {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data = temp_dir.path().join("data.nt");
    let ontology = temp_dir.path().join("ontology.ofn");
    let output = temp_dir.path().join("inferred.nt");
    let report = temp_dir.path().join("report.json");

    write(&data, "");
    write(
        &ontology,
        "\
Prefix(:=<http://example.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.com/ontology>
Declaration(ObjectProperty(:hasParent))
Declaration(ObjectProperty(:hasSibling))
Declaration(ObjectProperty(:hasUncle))
Declaration(ObjectProperty(:friendOf))
SubObjectPropertyOf(ObjectPropertyChain(:hasParent :hasSibling) :hasUncle)
TransitiveObjectProperty(:hasParent)
FunctionalObjectProperty(:friendOf)
)
",
    );

    strix::run([
        "strix",
        "reason",
        data.to_str().unwrap(),
        "--ontology",
        ontology.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--report",
        report.to_str().unwrap(),
    ])
    .expect("reasoning run should succeed");

    // Verify no unsupported constructs for these axioms
    let report_json = fs::read_to_string(&report).expect("report should exist");
    assert!(!report_json.contains("property chain"));
    assert!(!report_json.contains("TransitiveObjectProperty"));
}

#[test]
fn infers_inverse_with_subproperty_interaction() {
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/knows> <http://example.com/bob> .\n",
        "\
Prefix(:=<http://example.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.com/ontology>
Declaration(ObjectProperty(:knows))
Declaration(ObjectProperty(:knownBy))
Declaration(ObjectProperty(:relatedTo))
InverseObjectProperties(:knows :knownBy)
SubObjectPropertyOf(:knownBy :relatedTo)
ObjectPropertyDomain(:relatedTo :Entity)
)",
    );
    // alice knows bob → bob knownBy alice (inverse)
    assert!(inferred
        .contains("<http://example.com/bob> <http://example.com/knownBy> <http://example.com/alice> ."));
    // bob knownBy alice → bob relatedTo alice (subproperty)
    assert!(inferred
        .contains("<http://example.com/bob> <http://example.com/relatedTo> <http://example.com/alice> ."));
    // bob relatedTo alice → bob type Entity (domain)
    assert!(inferred.contains(
        "<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/Entity> ."
    ));
}

// ─── Rule isolation tests ───────────────────────────────────────────────────

#[test]
fn subclass_only() {
    let inferred = reason(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:A))
Declaration(Class(:B))
SubClassOf(:A :B)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."));
    assert_eq!(count_triples(&inferred), 1, "only one triple should be inferred: {inferred}");
}

#[test]
fn subproperty_only() {
    let inferred = reason(
        "<http://x.com/a> <http://x.com/p> <http://x.com/b> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
Declaration(ObjectProperty(:q))
SubObjectPropertyOf(:p :q)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://x.com/q> <http://x.com/b> ."));
    assert_eq!(count_triples(&inferred), 1, "only one triple should be inferred: {inferred}");
}

#[test]
fn domain_only() {
    let inferred = reason(
        "<http://x.com/a> <http://x.com/p> <http://x.com/b> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
Declaration(Class(:C))
ObjectPropertyDomain(:p :C)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."));
    // Domain should NOT type the object
    assert!(!inferred.contains("<http://x.com/b>"));
    assert_eq!(count_triples(&inferred), 1, "only domain for subject: {inferred}");
}

#[test]
fn range_only() {
    let inferred = reason(
        "<http://x.com/a> <http://x.com/p> <http://x.com/b> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
Declaration(Class(:C))
ObjectPropertyRange(:p :C)
)",
    );
    assert!(inferred.contains("<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."));
    // Range should NOT type the subject
    assert!(!inferred.contains("<http://x.com/a>"));
    assert_eq!(count_triples(&inferred), 1, "only range for object: {inferred}");
}

#[test]
fn domain_does_not_fire_for_unrelated_property() {
    let inferred = reason(
        "<http://x.com/a> <http://x.com/q> <http://x.com/b> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
Declaration(ObjectProperty(:q))
Declaration(Class(:C))
ObjectPropertyDomain(:p :C)
)",
    );
    assert_eq!(count_triples(&inferred), 0, "domain on :p should not fire for :q: {inferred}");
}

#[test]
fn multiple_domains_on_single_property() {
    let inferred = reason(
        "<http://x.com/a> <http://x.com/p> <http://x.com/b> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
Declaration(Class(:C))
Declaration(Class(:D))
ObjectPropertyDomain(:p :C)
ObjectPropertyDomain(:p :D)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."));
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/D> ."));
    assert_eq!(count_triples(&inferred), 2, "both domains should fire: {inferred}");
}

#[test]
fn multiple_ranges_on_single_property() {
    let inferred = reason(
        "<http://x.com/a> <http://x.com/p> <http://x.com/b> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
Declaration(Class(:C))
Declaration(Class(:D))
ObjectPropertyRange(:p :C)
ObjectPropertyRange(:p :D)
)",
    );
    assert!(inferred.contains("<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."));
    assert!(inferred.contains("<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/D> ."));
    assert_eq!(count_triples(&inferred), 2, "both ranges should fire: {inferred}");
}

#[test]
fn transitive_property_simple_chain() {
    let inferred = reason(
        "\
<http://x.com/a> <http://x.com/partOf> <http://x.com/b> .
<http://x.com/b> <http://x.com/partOf> <http://x.com/c> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:partOf))
TransitiveObjectProperty(:partOf)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://x.com/partOf> <http://x.com/c> ."));
    assert_eq!(count_triples(&inferred), 1, "one transitive inference: {inferred}");
}

#[test]
fn transitive_property_longer_chain() {
    let inferred = reason(
        "\
<http://x.com/a> <http://x.com/partOf> <http://x.com/b> .
<http://x.com/b> <http://x.com/partOf> <http://x.com/c> .
<http://x.com/c> <http://x.com/partOf> <http://x.com/d> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:partOf))
TransitiveObjectProperty(:partOf)
)",
    );
    // a→b→c→d should produce: a→c, a→d, b→d
    assert!(inferred.contains("<http://x.com/a> <http://x.com/partOf> <http://x.com/c> ."));
    assert!(inferred.contains("<http://x.com/a> <http://x.com/partOf> <http://x.com/d> ."));
    assert!(inferred.contains("<http://x.com/b> <http://x.com/partOf> <http://x.com/d> ."));
    assert_eq!(count_triples(&inferred), 3, "a→c, a→d, b→d: {inferred}");
}

#[test]
fn transitive_property_does_not_fire_for_non_transitive() {
    let inferred = reason(
        "\
<http://x.com/a> <http://x.com/p> <http://x.com/b> .
<http://x.com/b> <http://x.com/p> <http://x.com/c> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
)",
    );
    assert_eq!(count_triples(&inferred), 0, "non-transitive property should not chain: {inferred}");
}

#[test]
fn transitive_property_cyclic() {
    let inferred = reason(
        "\
<http://x.com/a> <http://x.com/partOf> <http://x.com/b> .
<http://x.com/b> <http://x.com/partOf> <http://x.com/a> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:partOf))
TransitiveObjectProperty(:partOf)
)",
    );
    // a→b, b→a. Transitive: a→b∧b→a → a→a, b→a∧a→b → b→b.
    // Reflexive results are novel but a→b and b→a are already asserted.
    assert!(inferred.contains("<http://x.com/a> <http://x.com/partOf> <http://x.com/a> ."));
    assert!(inferred.contains("<http://x.com/b> <http://x.com/partOf> <http://x.com/b> ."));
    assert_eq!(count_triples(&inferred), 2, "cyclic transitive converges: {inferred}");
}

// ─── Over-inference guards ──────────────────────────────────────────────────

#[test]
fn reflexive_subclass_produces_no_inferences() {
    let inferred = reason(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:A))
SubClassOf(:A :A)
)",
    );
    assert_eq!(count_triples(&inferred), 0, "reflexive subClassOf should not produce inferences: {inferred}");
}

#[test]
fn inverse_does_not_duplicate_existing_triple() {
    // If bob already has the triple, inverse should not re-emit it
    let inferred = reason(
        "\
<http://x.com/alice> <http://x.com/knows> <http://x.com/bob> .
<http://x.com/bob> <http://x.com/knownBy> <http://x.com/alice> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:knows))
Declaration(ObjectProperty(:knownBy))
InverseObjectProperties(:knows :knownBy)
)",
    );
    // The inverse of alice knows bob is bob knownBy alice — already asserted.
    // The inverse of bob knownBy alice is alice knows bob — already asserted.
    // So zero net new triples.
    assert_eq!(count_triples(&inferred), 0, "already-present inverse triples should not be re-emitted: {inferred}");
}

#[test]
fn symmetric_does_not_duplicate_existing_triple() {
    let inferred = reason(
        "\
<http://x.com/alice> <http://x.com/friendOf> <http://x.com/bob> .
<http://x.com/bob> <http://x.com/friendOf> <http://x.com/alice> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:friendOf))
SymmetricObjectProperty(:friendOf)
)",
    );
    assert_eq!(count_triples(&inferred), 0, "symmetric should not re-emit existing triples: {inferred}");
}

// ─── Multi-iteration convergence ────────────────────────────────────────────

#[test]
fn deep_subclass_chain_five_levels() {
    let inferred = reason(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:A))
Declaration(Class(:B))
Declaration(Class(:C))
Declaration(Class(:D))
Declaration(Class(:E))
SubClassOf(:A :B)
SubClassOf(:B :C)
SubClassOf(:C :D)
SubClassOf(:D :E)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."));
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."));
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/D> ."));
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/E> ."));
    assert_eq!(count_triples(&inferred), 4, "A→B→C→D→E produces 4 inferences: {inferred}");
}

#[test]
fn multi_level_subproperty_chain() {
    let inferred = reason(
        "<http://x.com/a> <http://x.com/p> <http://x.com/b> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
Declaration(ObjectProperty(:q))
Declaration(ObjectProperty(:r))
SubObjectPropertyOf(:p :q)
SubObjectPropertyOf(:q :r)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://x.com/q> <http://x.com/b> ."));
    assert!(inferred.contains("<http://x.com/a> <http://x.com/r> <http://x.com/b> ."));
    assert_eq!(count_triples(&inferred), 2, "p→q→r produces 2 inferences: {inferred}");
}

#[test]
fn cyclic_subclass_converges() {
    let inferred = reason(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> .
<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:A))
Declaration(Class(:B))
SubClassOf(:A :B)
SubClassOf(:B :A)
)",
    );
    // a:A → a:B, b:B → b:A, then a:B → a:A (already asserted), b:A → b:B (already asserted)
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."));
    assert!(inferred.contains("<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> ."));
    assert_eq!(count_triples(&inferred), 2, "cycle should converge with exactly 2 new triples: {inferred}");
}

#[test]
fn diamond_subclass_hierarchy() {
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D
    let inferred = reason(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/D> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:A))
Declaration(Class(:B))
Declaration(Class(:C))
Declaration(Class(:D))
SubClassOf(:D :B)
SubClassOf(:D :C)
SubClassOf(:B :A)
SubClassOf(:C :A)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."));
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."));
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> ."));
    // A should appear exactly once despite two paths to it
    assert_eq!(count_triples(&inferred), 3, "diamond should not duplicate A: {inferred}");
}

// ─── Edge cases ─────────────────────────────────────────────────────────────

#[test]
fn empty_data_with_schema_produces_no_output() {
    let inferred = reason(
        "",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:A))
Declaration(Class(:B))
SubClassOf(:A :B)
)",
    );
    assert_eq!(count_triples(&inferred), 0, "no data means no inferences: {inferred}");
}

#[test]
fn data_without_schema_produces_no_inferences() {
    let inferred = reason_data_only(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> .
<http://x.com/a> <http://x.com/p> <http://x.com/b> .
",
    );
    assert_eq!(count_triples(&inferred), 0, "no schema means no inferences: {inferred}");
}

// ─── Rule interaction tests ─────────────────────────────────────────────────

#[test]
fn subproperty_triggers_domain_on_superproperty() {
    let inferred = reason(
        "<http://x.com/a> <http://x.com/p> <http://x.com/b> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
Declaration(ObjectProperty(:q))
Declaration(Class(:C))
SubObjectPropertyOf(:p :q)
ObjectPropertyDomain(:q :C)
)",
    );
    // p→q infers (a, q, b), then domain(q) infers type(a, C)
    assert!(inferred.contains("<http://x.com/a> <http://x.com/q> <http://x.com/b> ."));
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."));
    assert_eq!(count_triples(&inferred), 2, "subproperty + domain interaction: {inferred}");
}

#[test]
fn domain_triggers_subclass() {
    let inferred = reason(
        "<http://x.com/a> <http://x.com/p> <http://x.com/b> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
Declaration(Class(:C))
Declaration(Class(:D))
ObjectPropertyDomain(:p :C)
SubClassOf(:C :D)
)",
    );
    // domain(p) → type(a, C), then C⊑D → type(a, D)
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."));
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/D> ."));
    assert_eq!(count_triples(&inferred), 2, "domain + subclass chain: {inferred}");
}

#[test]
fn symmetric_plus_subproperty_interaction() {
    let inferred = reason(
        "<http://x.com/alice> <http://x.com/friendOf> <http://x.com/bob> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:friendOf))
Declaration(ObjectProperty(:relatedTo))
SymmetricObjectProperty(:friendOf)
SubObjectPropertyOf(:friendOf :relatedTo)
)",
    );
    // symmetric: bob friendOf alice
    assert!(inferred.contains("<http://x.com/bob> <http://x.com/friendOf> <http://x.com/alice> ."));
    // subproperty on original: alice relatedTo bob
    assert!(inferred.contains("<http://x.com/alice> <http://x.com/relatedTo> <http://x.com/bob> ."));
    // subproperty on symmetric result: bob relatedTo alice
    assert!(inferred.contains("<http://x.com/bob> <http://x.com/relatedTo> <http://x.com/alice> ."));
    assert_eq!(count_triples(&inferred), 3, "symmetric + subproperty: {inferred}");
}

#[test]
fn transitive_plus_domain_interaction() {
    let inferred = reason(
        "\
<http://x.com/a> <http://x.com/partOf> <http://x.com/b> .
<http://x.com/b> <http://x.com/partOf> <http://x.com/c> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:partOf))
Declaration(Class(:Component))
TransitiveObjectProperty(:partOf)
ObjectPropertyDomain(:partOf :Component)
)",
    );
    // transitive: a partOf c
    assert!(inferred.contains("<http://x.com/a> <http://x.com/partOf> <http://x.com/c> ."));
    // domain fires on all 3 property assertions (2 asserted + 1 inferred):
    // a type Component, b type Component (from a→b and b→c), a type Component again (from a→c, dedup)
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Component> ."));
    assert!(inferred.contains("<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Component> ."));
    // 1 transitive property + 2 domain types = 3 inferences
    assert_eq!(count_triples(&inferred), 3, "transitive + domain: {inferred}");
}

#[test]
fn transitive_plus_subproperty_interaction() {
    let inferred = reason(
        "\
<http://x.com/a> <http://x.com/strictPartOf> <http://x.com/b> .
<http://x.com/b> <http://x.com/strictPartOf> <http://x.com/c> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:strictPartOf))
Declaration(ObjectProperty(:partOf))
SubObjectPropertyOf(:strictPartOf :partOf)
TransitiveObjectProperty(:partOf)
)",
    );
    // subproperty: a partOf b, b partOf c
    assert!(inferred.contains("<http://x.com/a> <http://x.com/partOf> <http://x.com/b> ."));
    assert!(inferred.contains("<http://x.com/b> <http://x.com/partOf> <http://x.com/c> ."));
    // transitive on partOf: a partOf c
    assert!(inferred.contains("<http://x.com/a> <http://x.com/partOf> <http://x.com/c> ."));
    assert_eq!(count_triples(&inferred), 3, "subproperty + transitive: {inferred}");
}

#[test]
fn double_inverse_is_identity() {
    // inverseOf(knows, knownBy) + inverseOf(knownBy, knows) → applying both is identity
    let inferred = reason(
        "<http://x.com/alice> <http://x.com/knows> <http://x.com/bob> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:knows))
Declaration(ObjectProperty(:knownBy))
InverseObjectProperties(:knows :knownBy)
)",
    );
    // alice knows bob → bob knownBy alice (inverse)
    // bob knownBy alice → alice knows bob (inverse back) — already asserted
    assert!(inferred.contains("<http://x.com/bob> <http://x.com/knownBy> <http://x.com/alice> ."));
    assert_eq!(count_triples(&inferred), 1, "double inverse should not produce extra triples: {inferred}");
}

#[test]
fn equivalent_classes_three_way() {
    let inferred = reason(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> .
<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> .
<http://x.com/c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:A))
Declaration(Class(:B))
Declaration(Class(:C))
EquivalentClasses(:A :B :C)
)",
    );
    // a:A → a:B, a:C
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."));
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."));
    // b:B → b:A, b:C
    assert!(inferred.contains("<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> ."));
    assert!(inferred.contains("<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."));
    // c:C → c:A, c:B
    assert!(inferred.contains("<http://x.com/c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> ."));
    assert!(inferred.contains("<http://x.com/c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."));
    assert_eq!(count_triples(&inferred), 6, "3-way equivalence: each instance gets 2 new types: {inferred}");
}

#[test]
fn multiple_instances_same_class_hierarchy() {
    let inferred = reason(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> .
<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> .
<http://x.com/c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:A))
Declaration(Class(:B))
SubClassOf(:A :B)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."));
    assert!(inferred.contains("<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."));
    assert!(inferred.contains("<http://x.com/c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."));
    assert_eq!(count_triples(&inferred), 3, "each instance inferred independently: {inferred}");
}

// ─── Class restriction rules (Step 4) ────────────────────────────────────────

#[test]
fn has_value_property_to_type() {
    // cls-hv1: SubClassOf(HasValue(hasPet, fido), PetOwner) — property(x,hasPet,fido) → type(x,PetOwner)
    let inferred = reason(
        "<http://x.com/alice> <http://x.com/hasPet> <http://x.com/fido> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasPet))
Declaration(Class(:PetOwner))
Declaration(NamedIndividual(:fido))
SubClassOf(ObjectHasValue(:hasPet :fido) :PetOwner)
)",
    );
    assert!(inferred.contains("<http://x.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/PetOwner> ."));
    assert_eq!(count_triples(&inferred), 1, "cls-hv1: {inferred}");
}

#[test]
fn has_value_type_to_property() {
    // cls-hv2: SubClassOf(PetOwner, HasValue(hasPet, fido)) — type(x,PetOwner) → property(x,hasPet,fido)
    let inferred = reason(
        "<http://x.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/PetOwner> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasPet))
Declaration(Class(:PetOwner))
Declaration(NamedIndividual(:fido))
SubClassOf(:PetOwner ObjectHasValue(:hasPet :fido))
)",
    );
    assert!(inferred.contains("<http://x.com/alice> <http://x.com/hasPet> <http://x.com/fido> ."));
    assert_eq!(count_triples(&inferred), 1, "cls-hv2: {inferred}");
}

#[test]
fn has_value_does_not_fire_for_wrong_value() {
    let inferred = reason(
        "<http://x.com/alice> <http://x.com/hasPet> <http://x.com/rex> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasPet))
Declaration(Class(:PetOwner))
Declaration(NamedIndividual(:fido))
SubClassOf(ObjectHasValue(:hasPet :fido) :PetOwner)
)",
    );
    assert_eq!(count_triples(&inferred), 0, "wrong value should not trigger hasValue: {inferred}");
}

#[test]
fn some_values_from_basic() {
    // cls-svf1: SubClassOf(SomeValuesFrom(hasPet, Dog), DogOwner)
    // property(alice, hasPet, fido) ∧ type(fido, Dog) → type(alice, DogOwner)
    let inferred = reason(
        "\
<http://x.com/alice> <http://x.com/hasPet> <http://x.com/fido> .
<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasPet))
Declaration(Class(:Dog))
Declaration(Class(:DogOwner))
SubClassOf(ObjectSomeValuesFrom(:hasPet :Dog) :DogOwner)
)",
    );
    assert!(inferred.contains("<http://x.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/DogOwner> ."));
    assert_eq!(count_triples(&inferred), 1, "cls-svf1: {inferred}");
}

#[test]
fn some_values_from_missing_filler_type() {
    // fido is not typed as Dog → no inference
    let inferred = reason(
        "<http://x.com/alice> <http://x.com/hasPet> <http://x.com/fido> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasPet))
Declaration(Class(:Dog))
Declaration(Class(:DogOwner))
SubClassOf(ObjectSomeValuesFrom(:hasPet :Dog) :DogOwner)
)",
    );
    assert_eq!(count_triples(&inferred), 0, "missing filler type means no svf: {inferred}");
}

#[test]
fn some_values_from_type_triggered() {
    // Property assertion arrives first (in seed), type arrives via inference
    // SubClassOf(A, B) + SubClassOf(SVF(p, B), C)
    // type(y, A) → type(y, B) [subclass], then property(x, p, y) ∧ type(y, B) → type(x, C) [svf]
    let inferred = reason(
        "\
<http://x.com/x> <http://x.com/p> <http://x.com/y> .
<http://x.com/y> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
Declaration(Class(:A))
Declaration(Class(:B))
Declaration(Class(:C))
SubClassOf(:A :B)
SubClassOf(ObjectSomeValuesFrom(:p :B) :C)
)",
    );
    // y:A → y:B, then x p y ∧ y:B → x:C
    assert!(inferred.contains("<http://x.com/y> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."));
    assert!(inferred.contains("<http://x.com/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."));
    assert_eq!(count_triples(&inferred), 2, "svf type-triggered: {inferred}");
}

#[test]
fn all_values_from_basic() {
    // cls-avf: SubClassOf(DogOwner, AllValuesFrom(hasPet, Dog))
    // type(alice, DogOwner) ∧ property(alice, hasPet, fido) → type(fido, Dog)
    let inferred = reason(
        "\
<http://x.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/DogOwner> .
<http://x.com/alice> <http://x.com/hasPet> <http://x.com/fido> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasPet))
Declaration(Class(:Dog))
Declaration(Class(:DogOwner))
SubClassOf(:DogOwner ObjectAllValuesFrom(:hasPet :Dog))
)",
    );
    assert!(inferred.contains("<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> ."));
    assert_eq!(count_triples(&inferred), 1, "cls-avf: {inferred}");
}

#[test]
fn all_values_from_no_fire_without_class_membership() {
    // alice is NOT typed DogOwner, so AVF should not fire
    let inferred = reason(
        "<http://x.com/alice> <http://x.com/hasPet> <http://x.com/fido> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasPet))
Declaration(Class(:Dog))
Declaration(Class(:DogOwner))
SubClassOf(:DogOwner ObjectAllValuesFrom(:hasPet :Dog))
)",
    );
    assert_eq!(count_triples(&inferred), 0, "avf without class membership: {inferred}");
}

#[test]
fn all_values_from_property_triggered() {
    // type arrives first (in seed), property arrives via inference (inverse)
    // SubClassOf(A, AVF(p, B)) + inverse(q, p)
    // data: type(x, A), q(y, x) → p(x, y) [inverse], then type(x, A) ∧ p(x, y) → type(y, B)
    let inferred = reason(
        "\
<http://x.com/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> .
<http://x.com/y> <http://x.com/q> <http://x.com/x> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
Declaration(ObjectProperty(:q))
Declaration(Class(:A))
Declaration(Class(:B))
InverseObjectProperties(:q :p)
SubClassOf(:A ObjectAllValuesFrom(:p :B))
)",
    );
    // q(y, x) → p(x, y) [inverse], then type(x, A) ∧ p(x, y) → type(y, B) [avf]
    assert!(inferred.contains("<http://x.com/x> <http://x.com/p> <http://x.com/y> ."));
    assert!(inferred.contains("<http://x.com/y> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."));
}

#[test]
fn intersection_of_basic() {
    // cls-int1: SubClassOf(IntersectionOf(Dog, Worker), WorkingDog)
    // type(rex, Dog) ∧ type(rex, Worker) → type(rex, WorkingDog)
    let inferred = reason(
        "\
<http://x.com/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .
<http://x.com/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Worker> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Dog))
Declaration(Class(:Worker))
Declaration(Class(:WorkingDog))
SubClassOf(ObjectIntersectionOf(:Dog :Worker) :WorkingDog)
)",
    );
    assert!(inferred.contains("<http://x.com/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/WorkingDog> ."));
    assert_eq!(count_triples(&inferred), 1, "cls-int1: {inferred}");
}

#[test]
fn intersection_of_missing_conjunct() {
    // rex is only Dog, not Worker → no inference
    let inferred = reason(
        "<http://x.com/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Dog))
Declaration(Class(:Worker))
Declaration(Class(:WorkingDog))
SubClassOf(ObjectIntersectionOf(:Dog :Worker) :WorkingDog)
)",
    );
    assert_eq!(count_triples(&inferred), 0, "missing conjunct: {inferred}");
}

#[test]
fn intersection_of_conjunct_arrives_via_inference() {
    // rex is Dog; Worker arrives via subclass from Laborer
    let inferred = reason(
        "\
<http://x.com/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .
<http://x.com/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Laborer> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Dog))
Declaration(Class(:Worker))
Declaration(Class(:Laborer))
Declaration(Class(:WorkingDog))
SubClassOf(:Laborer :Worker)
SubClassOf(ObjectIntersectionOf(:Dog :Worker) :WorkingDog)
)",
    );
    // Laborer → Worker, then Dog ∧ Worker → WorkingDog
    assert!(inferred.contains("<http://x.com/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Worker> ."));
    assert!(inferred.contains("<http://x.com/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/WorkingDog> ."));
}

#[test]
fn intersection_of_decomposition() {
    // cls-int2: EquivalentClasses(WorkingDog, IntersectionOf(Dog, Worker))
    // The EquivalentClasses produces SubClassOf(WorkingDog, IntersectionOf(Dog, Worker))
    // which the parser decomposes to SubClassOf(WorkingDog, Dog) and SubClassOf(WorkingDog, Worker)
    // So type(rex, WorkingDog) → type(rex, Dog) ∧ type(rex, Worker)
    let inferred = reason(
        "<http://x.com/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/WorkingDog> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Dog))
Declaration(Class(:Worker))
Declaration(Class(:WorkingDog))
SubClassOf(:WorkingDog ObjectIntersectionOf(:Dog :Worker))
)",
    );
    assert!(inferred.contains("<http://x.com/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> ."));
    assert!(inferred.contains("<http://x.com/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Worker> ."));
    assert_eq!(count_triples(&inferred), 2, "cls-int2 decomposition: {inferred}");
}

#[test]
fn union_decomposition_via_subclass() {
    // EquivalentClasses(Animal, UnionOf(Cat, Dog)) decomposes to SubClassOf(Cat, Animal), SubClassOf(Dog, Animal)
    let inferred = reason(
        "\
<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .
<http://x.com/whiskers> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Cat> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Animal))
Declaration(Class(:Cat))
Declaration(Class(:Dog))
EquivalentClasses(:Animal ObjectUnionOf(:Cat :Dog))
)",
    );
    assert!(inferred.contains("<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Animal> ."));
    assert!(inferred.contains("<http://x.com/whiskers> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Animal> ."));
    assert_eq!(count_triples(&inferred), 2, "union decomposition: {inferred}");
}

#[test]
fn svf_plus_avf_interaction() {
    // SVF and AVF on the same property, creating a chain of inferences
    // SubClassOf(SVF(p, B), C) and SubClassOf(C, AVF(q, D))
    // property(x, p, y) ∧ type(y, B) → type(x, C) [svf]
    // type(x, C) ∧ property(x, q, z) → type(z, D) [avf]
    let inferred = reason(
        "\
<http://x.com/x> <http://x.com/p> <http://x.com/y> .
<http://x.com/y> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> .
<http://x.com/x> <http://x.com/q> <http://x.com/z> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
Declaration(ObjectProperty(:q))
Declaration(Class(:B))
Declaration(Class(:C))
Declaration(Class(:D))
SubClassOf(ObjectSomeValuesFrom(:p :B) :C)
SubClassOf(:C ObjectAllValuesFrom(:q :D))
)",
    );
    assert!(inferred.contains("<http://x.com/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."));
    assert!(inferred.contains("<http://x.com/z> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/D> ."));
}

#[test]
fn has_value_plus_subclass_interaction() {
    // cls-hv1 infers type, then subclass propagates it
    // HasValue(p, v) → C, C ⊑ D
    let inferred = reason(
        "<http://x.com/a> <http://x.com/p> <http://x.com/v> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
Declaration(Class(:C))
Declaration(Class(:D))
Declaration(NamedIndividual(:v))
SubClassOf(ObjectHasValue(:p :v) :C)
SubClassOf(:C :D)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."));
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/D> ."));
    assert_eq!(count_triples(&inferred), 2, "hv1 + subclass: {inferred}");
}

// ─── owl:Thing handling ─────────────────────────────────────────────────────

#[test]
fn owl_thing_not_materialized_as_superclass() {
    // SubClassOf(Dog, owl:Thing) should not produce type(fido, owl:Thing)
    let inferred = reason(
        "<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Dog))
SubClassOf(:Dog owl:Thing)
)",
    );
    assert_eq!(count_triples(&inferred), 0, "owl:Thing superclass should be suppressed: {inferred}");
}

#[test]
fn owl_thing_subclass_produces_universal_types() {
    // SubClassOf(owl:Thing, Existent) → every individual gets type(x, Existent)
    let inferred = reason(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .
<http://x.com/b> <http://x.com/p> <http://x.com/c> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Existent))
SubClassOf(owl:Thing :Existent)
)",
    );
    // a (from type assertion), b and c (from property assertion) should all be Existent
    assert!(inferred.contains("<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Existent> ."));
    assert!(inferred.contains("<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Existent> ."));
    assert!(inferred.contains("<http://x.com/c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Existent> ."));
    assert_eq!(count_triples(&inferred), 3, "universal types: {inferred}");
}

#[test]
fn svf_owl_thing_filler_becomes_property_existence() {
    // SubClassOf(SomeValuesFrom(hasPet, owl:Thing), PetOwner)
    // property(alice, hasPet, fido) → type(alice, PetOwner) without filler check
    let inferred = reason(
        "<http://x.com/alice> <http://x.com/hasPet> <http://x.com/fido> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasPet))
Declaration(Class(:PetOwner))
SubClassOf(ObjectSomeValuesFrom(:hasPet owl:Thing) :PetOwner)
)",
    );
    assert!(inferred.contains("<http://x.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/PetOwner> ."));
    assert_eq!(count_triples(&inferred), 1, "svf owl:Thing: {inferred}");
}

#[test]
fn avf_owl_thing_filler_is_dropped() {
    // SubClassOf(Keeper, AllValuesFrom(hasPet, owl:Thing)) is trivially true — no inference
    let inferred = reason(
        "\
<http://x.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Keeper> .
<http://x.com/alice> <http://x.com/hasPet> <http://x.com/fido> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasPet))
Declaration(Class(:Keeper))
SubClassOf(:Keeper ObjectAllValuesFrom(:hasPet owl:Thing))
)",
    );
    assert_eq!(count_triples(&inferred), 0, "avf owl:Thing should be dropped: {inferred}");
}

#[test]
fn domain_owl_thing_is_dropped() {
    // ObjectPropertyDomain(p, owl:Thing) is trivially true — no inference
    let inferred = reason(
        "<http://x.com/a> <http://x.com/p> <http://x.com/b> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
ObjectPropertyDomain(:p owl:Thing)
)",
    );
    assert_eq!(count_triples(&inferred), 0, "domain owl:Thing should be dropped: {inferred}");
}

#[test]
fn range_owl_thing_is_dropped() {
    // ObjectPropertyRange(p, owl:Thing) is trivially true — no inference
    let inferred = reason(
        "<http://x.com/a> <http://x.com/p> <http://x.com/b> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
ObjectPropertyRange(:p owl:Thing)
)",
    );
    assert_eq!(count_triples(&inferred), 0, "range owl:Thing should be dropped: {inferred}");
}

#[test]
fn intersection_owl_thing_conjunct_is_removed() {
    // SubClassOf(IntersectionOf(Dog, owl:Thing), GoodDog) simplifies to SubClassOf(Dog, GoodDog)
    let inferred = reason(
        "<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Dog))
Declaration(Class(:GoodDog))
SubClassOf(ObjectIntersectionOf(:Dog owl:Thing) :GoodDog)
)",
    );
    assert!(inferred.contains("<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/GoodDog> ."));
    assert_eq!(count_triples(&inferred), 1, "intersection with owl:Thing conjunct removed: {inferred}");
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn write(path: &Path, content: &str) {
    fs::write(path, content).expect("test fixture should be written");
}

/// Count non-empty lines in an N-Triples file (each line = one triple).
fn count_triples(ntriples: &str) -> usize {
    ntriples.lines().filter(|l| !l.trim().is_empty()).count()
}

/// Minimal harness: write data + ontology, run reasoner, return inferred N-Triples string.
/// Uses `--emit inferred` (default) so only new triples appear.
fn reason(data: &str, ontology: &str) -> String {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data_path = temp_dir.path().join("data.nt");
    let ontology_path = temp_dir.path().join("ontology.ofn");
    let output_path = temp_dir.path().join("inferred.nt");

    write(&data_path, data);
    write(&ontology_path, ontology);

    strix::run([
        "strix",
        "reason",
        data_path.to_str().unwrap(),
        "--ontology",
        ontology_path.to_str().unwrap(),
        "--output",
        output_path.to_str().unwrap(),
    ])
    .expect("reasoning run should succeed");

    fs::read_to_string(&output_path).expect("output should exist")
}

/// Like `reason` but with no ontology file (data-only, or extracted schema).
fn reason_data_only(data: &str) -> String {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data_path = temp_dir.path().join("data.nt");
    let output_path = temp_dir.path().join("inferred.nt");

    write(&data_path, data);

    strix::run([
        "strix",
        "reason",
        data_path.to_str().unwrap(),
        "--output",
        output_path.to_str().unwrap(),
    ])
    .expect("reasoning run should succeed");

    fs::read_to_string(&output_path).expect("output should exist")
}

fn write_gzip(path: &Path, content: &str) {
    let file = fs::File::create(path).expect("gzip test fixture should be created");
    let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    encoder
        .write_all(content.as_bytes())
        .expect("gzip test fixture should be written");
    encoder.finish().expect("gzip encoder should finish");
}

fn write_bzip2(path: &Path, content: &str) {
    let file = fs::File::create(path).expect("bzip2 test fixture should be created");
    let mut encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
    encoder
        .write_all(content.as_bytes())
        .expect("bzip2 test fixture should be written");
    encoder.finish().expect("bzip2 encoder should finish");
}

fn write_xz(path: &Path, content: &str) {
    let file = fs::File::create(path).expect("xz test fixture should be created");
    let mut encoder = xz2::write::XzEncoder::new(file, 6);
    encoder
        .write_all(content.as_bytes())
        .expect("xz test fixture should be written");
    encoder.finish().expect("xz encoder should finish");
}
