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
    assert!(inferred.contains(
        "<http://example.com/bob> <http://example.com/knownBy> <http://example.com/alice> ."
    ));
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
    assert!(inferred.contains(
        "<http://example.com/bob> <http://example.com/friendOf> <http://example.com/alice> ."
    ));
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
    assert!(inferred.contains(
        "<http://example.com/bob> <http://example.com/knownBy> <http://example.com/alice> ."
    ));
    // bob knownBy alice → bob relatedTo alice (subproperty)
    assert!(inferred.contains(
        "<http://example.com/bob> <http://example.com/relatedTo> <http://example.com/alice> ."
    ));
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
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."
    ));
    assert_eq!(
        count_triples(&inferred),
        1,
        "only one triple should be inferred: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        1,
        "only one triple should be inferred: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
    ));
    // Domain should NOT type the object
    assert!(!inferred.contains("<http://x.com/b>"));
    assert_eq!(
        count_triples(&inferred),
        1,
        "only domain for subject: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
    ));
    // Range should NOT type the subject
    assert!(!inferred.contains("<http://x.com/a>"));
    assert_eq!(
        count_triples(&inferred),
        1,
        "only range for object: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        0,
        "domain on :p should not fire for :q: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/D> ."
    ));
    assert_eq!(
        count_triples(&inferred),
        2,
        "both domains should fire: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/D> ."
    ));
    assert_eq!(
        count_triples(&inferred),
        2,
        "both ranges should fire: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        1,
        "one transitive inference: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        0,
        "non-transitive property should not chain: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        2,
        "cyclic transitive converges: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        0,
        "reflexive subClassOf should not produce inferences: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        0,
        "already-present inverse triples should not be re-emitted: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        0,
        "symmetric should not re-emit existing triples: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/D> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/E> ."
    ));
    assert_eq!(
        count_triples(&inferred),
        4,
        "A→B→C→D→E produces 4 inferences: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        2,
        "p→q→r produces 2 inferences: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> ."
    ));
    assert_eq!(
        count_triples(&inferred),
        2,
        "cycle should converge with exactly 2 new triples: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> ."
    ));
    // A should appear exactly once despite two paths to it
    assert_eq!(
        count_triples(&inferred),
        3,
        "diamond should not duplicate A: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        0,
        "no data means no inferences: {inferred}"
    );
}

#[test]
fn data_without_schema_produces_no_inferences() {
    let inferred = reason_data_only(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> .
<http://x.com/a> <http://x.com/p> <http://x.com/b> .
",
    );
    assert_eq!(
        count_triples(&inferred),
        0,
        "no schema means no inferences: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
    ));
    assert_eq!(
        count_triples(&inferred),
        2,
        "subproperty + domain interaction: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/D> ."
    ));
    assert_eq!(
        count_triples(&inferred),
        2,
        "domain + subclass chain: {inferred}"
    );
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
    assert!(
        inferred.contains("<http://x.com/alice> <http://x.com/relatedTo> <http://x.com/bob> .")
    );
    // subproperty on symmetric result: bob relatedTo alice
    assert!(
        inferred.contains("<http://x.com/bob> <http://x.com/relatedTo> <http://x.com/alice> .")
    );
    assert_eq!(
        count_triples(&inferred),
        3,
        "symmetric + subproperty: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        3,
        "transitive + domain: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        3,
        "subproperty + transitive: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        1,
        "double inverse should not produce extra triples: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
    ));
    // b:B → b:A, b:C
    assert!(inferred.contains(
        "<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
    ));
    // c:C → c:A, c:B
    assert!(inferred.contains(
        "<http://x.com/c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."
    ));
    assert_eq!(
        count_triples(&inferred),
        6,
        "3-way equivalence: each instance gets 2 new types: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."
    ));
    assert_eq!(
        count_triples(&inferred),
        3,
        "each instance inferred independently: {inferred}"
    );
}

// ─── Property chains (Step 5) ─────────────────────────────────────────────

#[test]
fn property_chain_basic() {
    // chain(hasParent, hasSibling) → hasUncle
    let inferred = reason(
        "\
<http://x.com/alice> <http://x.com/hasParent> <http://x.com/bob> .
<http://x.com/bob> <http://x.com/hasSibling> <http://x.com/charlie> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasParent))
Declaration(ObjectProperty(:hasSibling))
Declaration(ObjectProperty(:hasUncle))
SubObjectPropertyOf(ObjectPropertyChain(:hasParent :hasSibling) :hasUncle)
)",
    );
    assert!(
        inferred.contains("<http://x.com/alice> <http://x.com/hasUncle> <http://x.com/charlie> .")
    );
    assert_eq!(count_triples(&inferred), 1, "basic chain: {inferred}");
}

#[test]
fn property_chain_no_fire_without_match() {
    // chain(hasParent, hasSibling) → hasUncle, but data has hasSibling then hasParent (wrong order)
    let inferred = reason(
        "\
<http://x.com/alice> <http://x.com/hasSibling> <http://x.com/bob> .
<http://x.com/bob> <http://x.com/hasParent> <http://x.com/charlie> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasParent))
Declaration(ObjectProperty(:hasSibling))
Declaration(ObjectProperty(:hasUncle))
SubObjectPropertyOf(ObjectPropertyChain(:hasParent :hasSibling) :hasUncle)
)",
    );
    assert_eq!(
        count_triples(&inferred),
        0,
        "wrong order should not fire chain: {inferred}"
    );
}

#[test]
fn property_chain_recursive() {
    // chain(linksTo, extends) → linksTo — super property is in the chain
    // a linksTo b, b extends c → a linksTo c
    let inferred = reason(
        "\
<http://x.com/a> <http://x.com/linksTo> <http://x.com/b> .
<http://x.com/b> <http://x.com/extends> <http://x.com/c> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:linksTo))
Declaration(ObjectProperty(:extends))
SubObjectPropertyOf(ObjectPropertyChain(:linksTo :extends) :linksTo)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://x.com/linksTo> <http://x.com/c> ."));
    assert_eq!(count_triples(&inferred), 1, "recursive chain: {inferred}");
}

#[test]
fn property_chain_recursive_multi_hop() {
    // chain(linksTo, extends) → linksTo fires iteratively:
    // a linksTo b, b extends c, c extends d → a linksTo c (iter 1) → a linksTo d (iter 2)
    let inferred = reason(
        "\
<http://x.com/a> <http://x.com/linksTo> <http://x.com/b> .
<http://x.com/b> <http://x.com/extends> <http://x.com/c> .
<http://x.com/c> <http://x.com/extends> <http://x.com/d> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:linksTo))
Declaration(ObjectProperty(:extends))
SubObjectPropertyOf(ObjectPropertyChain(:linksTo :extends) :linksTo)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://x.com/linksTo> <http://x.com/c> ."));
    assert!(inferred.contains("<http://x.com/a> <http://x.com/linksTo> <http://x.com/d> ."));
    assert_eq!(
        count_triples(&inferred),
        2,
        "recursive multi-hop chain: {inferred}"
    );
}

#[test]
fn property_chain_length_three() {
    // chain(p1, p2, p3) → r
    let inferred = reason(
        "\
<http://x.com/a> <http://x.com/p1> <http://x.com/b> .
<http://x.com/b> <http://x.com/p2> <http://x.com/c> .
<http://x.com/c> <http://x.com/p3> <http://x.com/d> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p1))
Declaration(ObjectProperty(:p2))
Declaration(ObjectProperty(:p3))
Declaration(ObjectProperty(:r))
SubObjectPropertyOf(ObjectPropertyChain(:p1 :p2 :p3) :r)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://x.com/r> <http://x.com/d> ."));
    assert_eq!(count_triples(&inferred), 1, "length-3 chain: {inferred}");
}

#[test]
fn self_join_chain_normalized_to_transitive() {
    // chain(p, p) → p should behave identically to TransitiveProperty(p)
    let inferred = reason(
        "\
<http://x.com/a> <http://x.com/p> <http://x.com/b> .
<http://x.com/b> <http://x.com/p> <http://x.com/c> .
<http://x.com/c> <http://x.com/p> <http://x.com/d> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
SubObjectPropertyOf(ObjectPropertyChain(:p :p) :p)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://x.com/p> <http://x.com/c> ."));
    assert!(inferred.contains("<http://x.com/a> <http://x.com/p> <http://x.com/d> ."));
    assert!(inferred.contains("<http://x.com/b> <http://x.com/p> <http://x.com/d> ."));
    assert_eq!(
        count_triples(&inferred),
        3,
        "self-join chain as transitive: {inferred}"
    );
}

#[test]
fn chain_plus_domain_interaction() {
    // chain(p1, p2) → r, domain(r) = C
    // a p1 b, b p2 c → a r c → type(a, C)
    let inferred = reason(
        "\
<http://x.com/a> <http://x.com/p1> <http://x.com/b> .
<http://x.com/b> <http://x.com/p2> <http://x.com/c> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p1))
Declaration(ObjectProperty(:p2))
Declaration(ObjectProperty(:r))
Declaration(Class(:C))
SubObjectPropertyOf(ObjectPropertyChain(:p1 :p2) :r)
ObjectPropertyDomain(:r :C)
)",
    );
    assert!(inferred.contains("<http://x.com/a> <http://x.com/r> <http://x.com/c> ."));
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
    ));
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
    assert_eq!(
        count_triples(&inferred),
        0,
        "wrong value should not trigger hasValue: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        0,
        "missing filler type means no svf: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/y> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
    ));
    assert_eq!(
        count_triples(&inferred),
        2,
        "svf type-triggered: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> ."
    ));
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
    assert_eq!(
        count_triples(&inferred),
        0,
        "avf without class membership: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/y> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/B> ."
    ));
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
    assert!(inferred.contains(
        "<http://x.com/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> ."
    ));
    assert!(inferred.contains("<http://x.com/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Worker> ."));
    assert_eq!(
        count_triples(&inferred),
        2,
        "cls-int2 decomposition: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        2,
        "union decomposition: {inferred}"
    );
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
    assert!(inferred.contains(
        "<http://x.com/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/z> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/D> ."
    ));
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
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/D> ."
    ));
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
    assert_eq!(
        count_triples(&inferred),
        0,
        "owl:Thing superclass should be suppressed: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        0,
        "avf owl:Thing should be dropped: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        0,
        "domain owl:Thing should be dropped: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        0,
        "range owl:Thing should be dropped: {inferred}"
    );
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
    assert_eq!(
        count_triples(&inferred),
        1,
        "intersection with owl:Thing conjunct removed: {inferred}"
    );
}

// ─── owl:sameAs / equality rules (Step 6) ─────────────────────────────────

#[test]
fn functional_property_basic() {
    // FunctionalProperty(hasMother): alice hasMother beth, alice hasMother elizabeth
    // → beth sameAs elizabeth
    let inferred = reason(
        "\
<http://x.com/alice> <http://x.com/hasMother> <http://x.com/beth> .
<http://x.com/alice> <http://x.com/hasMother> <http://x.com/elizabeth> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasMother))
FunctionalObjectProperty(:hasMother)
)",
    );
    assert!(
        inferred.contains("<http://www.w3.org/2002/07/owl#sameAs>"),
        "should produce sameAs: {inferred}"
    );
    // Both directions of sameAs
    assert!(inferred.contains(
        "<http://x.com/beth> <http://www.w3.org/2002/07/owl#sameAs> <http://x.com/elizabeth> ."
    ));
    assert!(inferred.contains(
        "<http://x.com/elizabeth> <http://www.w3.org/2002/07/owl#sameAs> <http://x.com/beth> ."
    ));
}

#[test]
fn inverse_functional_property_basic() {
    // InverseFunctionalProperty(hasSSN): alice hasSSN x, bob hasSSN x
    // → alice sameAs bob
    let inferred = reason(
        "\
<http://x.com/alice> <http://x.com/hasSSN> <http://x.com/ssn1> .
<http://x.com/bob> <http://x.com/hasSSN> <http://x.com/ssn1> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasSSN))
InverseFunctionalObjectProperty(:hasSSN)
)",
    );
    assert!(
        inferred.contains("<http://www.w3.org/2002/07/owl#sameAs>"),
        "should produce sameAs: {inferred}"
    );
    assert!(
        inferred.contains(
            "<http://x.com/alice> <http://www.w3.org/2002/07/owl#sameAs> <http://x.com/bob> ."
        ) || inferred.contains(
            "<http://x.com/bob> <http://www.w3.org/2002/07/owl#sameAs> <http://x.com/alice> ."
        )
    );
}

#[test]
fn functional_property_type_propagation() {
    // FunctionalProperty(hasMother), type(beth, Human)
    // alice hasMother beth, alice hasMother elizabeth
    // → beth sameAs elizabeth → type(elizabeth, Human) (via canonical rewrite)
    let inferred = reason(
        "\
<http://x.com/alice> <http://x.com/hasMother> <http://x.com/beth> .
<http://x.com/alice> <http://x.com/hasMother> <http://x.com/elizabeth> .
<http://x.com/beth> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Human> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasMother))
Declaration(Class(:Human))
FunctionalObjectProperty(:hasMother)
)",
    );
    // elizabeth should get type Human through canonical rewrite
    assert!(
        inferred.contains("<http://x.com/elizabeth> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Human> ."),
        "type should propagate through sameAs: {inferred}"
    );
}

#[test]
fn max_cardinality_one_basic() {
    // SubClassOf(Person, MaxCardinality(1, hasMother))
    // type(alice, Person), alice hasMother beth, alice hasMother elizabeth
    // → beth sameAs elizabeth
    let inferred = reason(
        "\
<http://x.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Person> .
<http://x.com/alice> <http://x.com/hasMother> <http://x.com/beth> .
<http://x.com/alice> <http://x.com/hasMother> <http://x.com/elizabeth> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasMother))
Declaration(Class(:Person))
SubClassOf(:Person ObjectMaxCardinality(1 :hasMother))
)",
    );
    assert!(
        inferred.contains("<http://www.w3.org/2002/07/owl#sameAs>"),
        "MaxCard(1) should produce sameAs: {inferred}"
    );
}

#[test]
fn asserted_sameas_propagates_types() {
    // a sameAs b, type(a, C) → type(b, C) via equality expansion
    let inferred = reason(
        "\
<http://x.com/a> <http://www.w3.org/2002/07/owl#sameAs> <http://x.com/b> .
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:C))
)",
    );
    assert!(
        inferred.contains(
            "<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> ."
        ),
        "type should propagate through asserted sameAs: {inferred}"
    );
}

#[test]
fn asserted_sameas_propagates_properties() {
    // a sameAs b, property(a, p, c) → property(b, p, c) via equality expansion
    let inferred = reason(
        "\
<http://x.com/a> <http://www.w3.org/2002/07/owl#sameAs> <http://x.com/b> .
<http://x.com/a> <http://x.com/p> <http://x.com/c> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:p))
)",
    );
    assert!(
        inferred.contains("<http://x.com/b> <http://x.com/p> <http://x.com/c> ."),
        "property should propagate through asserted sameAs: {inferred}"
    );
}

#[test]
fn functional_property_no_equality_without_conflict() {
    // FunctionalProperty(hasMother): alice hasMother beth (only one value)
    // → no sameAs produced
    let inferred = reason(
        "<http://x.com/alice> <http://x.com/hasMother> <http://x.com/beth> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasMother))
FunctionalObjectProperty(:hasMother)
)",
    );
    assert!(
        !inferred.contains("sameAs"),
        "no equality conflict, no sameAs: {inferred}"
    );
}

// ─── Inconsistency detection (Step 7) ──────────────────────────────────────

#[test]
fn disjoint_classes_detected() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let inferred = reason_with_report(
        "<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Cat> .
<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Cat))
Declaration(Class(:Dog))
DisjointClasses(:Cat :Dog)
)",
        &report,
        &[],
    );
    let _ = inferred; // reasoning succeeds in report mode
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("DisjointClasses"),
        "report should contain disjoint class inconsistency: {report_json}"
    );
    assert!(report_json.contains("disjoint"));
}

#[test]
fn disjoint_classes_halt_mode() {
    let result = reason_expecting_result(
        "<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Cat> .
<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Cat))
Declaration(Class(:Dog))
DisjointClasses(:Cat :Dog)
)",
        &["--inconsistency-mode", "halt"],
    );
    assert!(
        result.is_err(),
        "halt mode should return error on inconsistency"
    );
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("inconsisten"),
        "error should mention inconsistency: {msg}"
    );
}

#[test]
fn disjoint_classes_no_conflict() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .
<http://x.com/whiskers> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Cat> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Cat))
Declaration(Class(:Dog))
DisjointClasses(:Cat :Dog)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("\"inconsistencies\": []"),
        "no inconsistencies expected: {report_json}"
    );
}

#[test]
fn complement_of_detected() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Mortal> .
<http://x.com/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Immortal> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Mortal))
Declaration(Class(:Immortal))
SubClassOf(:Mortal ObjectComplementOf(:Immortal))
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("ComplementOf"),
        "should detect complement inconsistency: {report_json}"
    );
}

#[test]
fn disjoint_properties_detected() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/a> <http://x.com/likes> <http://x.com/b> .
<http://x.com/a> <http://x.com/dislikes> <http://x.com/b> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:likes))
Declaration(ObjectProperty(:dislikes))
DisjointObjectProperties(:likes :dislikes)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("DisjointProperties"),
        "should detect disjoint properties inconsistency: {report_json}"
    );
}

#[test]
fn max_cardinality_zero_detected() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Childless> .
<http://x.com/alice> <http://x.com/hasChild> <http://x.com/bob> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:hasChild))
Declaration(Class(:Childless))
SubClassOf(:Childless ObjectMaxCardinality(0 :hasChild))
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("MaxCardinalityZero"),
        "should detect max card 0 inconsistency: {report_json}"
    );
}

#[test]
fn disjoint_classes_inferred_conflict() {
    // Conflict arises through inference: fido is Dog, Dog subClassOf Animal,
    // Animal disjoint with Machine, fido also typed Machine.
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .
<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Machine> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Dog))
Declaration(Class(:Animal))
Declaration(Class(:Machine))
SubClassOf(:Dog :Animal)
DisjointClasses(:Animal :Machine)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("DisjointClasses"),
        "should detect inferred disjoint conflict: {report_json}"
    );
}

#[test]
fn disjoint_classes_nary() {
    // Three-way disjointness: conflict between non-adjacent members A and C.
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/A> .
<http://x.com/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:A))
Declaration(Class(:B))
Declaration(Class(:C))
DisjointClasses(:A :B :C)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("DisjointClasses"),
        "n-ary disjoint should detect conflict between non-adjacent members: {report_json}"
    );
}

#[test]
fn max_cardinality_zero_with_filler() {
    // MaxCard(0, P, C) with filler class: violation only when object has the filler type.
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Vegan> .
<http://x.com/alice> <http://x.com/eats> <http://x.com/steak> .
<http://x.com/steak> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Meat> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Vegan))
Declaration(Class(:Meat))
Declaration(ObjectProperty(:eats))
SubClassOf(:Vegan ObjectMaxCardinality(0 :eats :Meat))
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("MaxCardinalityZero"),
        "max card 0 with matching filler should detect inconsistency: {report_json}"
    );
}

#[test]
fn max_cardinality_zero_filler_no_match() {
    // MaxCard(0, P, C): no violation when the object does NOT have the filler type.
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Vegan> .
<http://x.com/alice> <http://x.com/eats> <http://x.com/salad> .
<http://x.com/salad> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Vegetable> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Vegan))
Declaration(Class(:Meat))
Declaration(Class(:Vegetable))
Declaration(ObjectProperty(:eats))
SubClassOf(:Vegan ObjectMaxCardinality(0 :eats :Meat))
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("\"inconsistencies\": []"),
        "max card 0 with non-matching filler should not be inconsistent: {report_json}"
    );
}

#[test]
fn disjoint_properties_via_subproperty() {
    // Conflict arises through inference: :knows subPropertyOf :relatedTo,
    // :relatedTo disjointWith :unrelatedTo.
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/a> <http://x.com/knows> <http://x.com/b> .
<http://x.com/a> <http://x.com/unrelatedTo> <http://x.com/b> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:knows))
Declaration(ObjectProperty(:relatedTo))
Declaration(ObjectProperty(:unrelatedTo))
SubObjectPropertyOf(:knows :relatedTo)
DisjointObjectProperties(:relatedTo :unrelatedTo)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("DisjointProperties"),
        "disjoint properties via subproperty inference should be detected: {report_json}"
    );
}

#[test]
fn complement_of_via_inference() {
    // One type is inferred through subclass chain.
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Alive> .
<http://x.com/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Rock> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Alive))
Declaration(Class(:Inorganic))
Declaration(Class(:Rock))
SubClassOf(:Rock :Inorganic)
SubClassOf(:Alive ObjectComplementOf(:Inorganic))
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("ComplementOf"),
        "complement conflict via inferred type should be detected: {report_json}"
    );
}

#[test]
fn disjoint_union() {
    // DisjointUnion(:Animal :Cat :Dog) should:
    // 1. Infer SubClassOf(:Cat, :Animal) and SubClassOf(:Dog, :Animal)
    // 2. Detect disjointness between :Cat and :Dog
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let inferred = reason_with_report(
        "\
<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .
<http://x.com/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Cat> .
<http://x.com/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Animal))
Declaration(Class(:Cat))
Declaration(Class(:Dog))
DisjointUnion(:Animal :Cat :Dog)
)",
        &report,
        &[],
    );
    // rex should be inferred as Animal via SubClassOf(Dog, Animal)
    assert!(
        inferred.contains("<http://x.com/rex>") && inferred.contains("<http://x.com/Animal>"),
        "DisjointUnion should infer subclass membership: {inferred}"
    );
    // fido is both Cat and Dog, which are disjoint members
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("DisjointClasses"),
        "DisjointUnion should detect disjointness between members: {report_json}"
    );
}

// ─── Irreflexive / Asymmetric ───────────────────────────────────────────────

#[test]
fn irreflexive_property_detected() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "<http://x.com/a> <http://x.com/strictlyGreaterThan> <http://x.com/a> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:strictlyGreaterThan))
IrreflexiveObjectProperty(:strictlyGreaterThan)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("IrreflexiveProperty"),
        "should detect irreflexive self-link: {report_json}"
    );
}

#[test]
fn irreflexive_property_no_self_link() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "<http://x.com/a> <http://x.com/strictlyGreaterThan> <http://x.com/b> .\n",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:strictlyGreaterThan))
IrreflexiveObjectProperty(:strictlyGreaterThan)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("\"inconsistencies\": []"),
        "no inconsistency for non-self-link: {report_json}"
    );
}

#[test]
fn asymmetric_property_detected() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/a> <http://x.com/parentOf> <http://x.com/b> .
<http://x.com/b> <http://x.com/parentOf> <http://x.com/a> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:parentOf))
AsymmetricObjectProperty(:parentOf)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("AsymmetricProperty"),
        "should detect asymmetric violation: {report_json}"
    );
}

#[test]
fn asymmetric_property_no_conflict() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/a> <http://x.com/parentOf> <http://x.com/b> .
<http://x.com/c> <http://x.com/parentOf> <http://x.com/d> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:parentOf))
AsymmetricObjectProperty(:parentOf)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("\"inconsistencies\": []"),
        "no inconsistency for one-directional links: {report_json}"
    );
}

// ─── SameIndividual axiom ───────────────────────────────────────────────────

#[test]
fn same_individual_axiom_produces_sameas() {
    // SameIndividual(a, b) from the ontology should produce owl:sameAs and unify facts.
    let inferred = reason(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .
<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Cat> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(NamedIndividual(:a))
Declaration(NamedIndividual(:b))
SameIndividual(:a :b)
)",
    );
    // Should produce owl:sameAs triple
    assert!(
        inferred.contains("owl:sameAs") || inferred.contains("sameAs"),
        "SameIndividual axiom should produce owl:sameAs: {inferred}"
    );
}

#[test]
fn same_individual_axiom_unifies_types() {
    // SameIndividual(a, b): a has type Dog, b has type Cat.
    // After unification, the canonical individual should have both types.
    let inferred = reason(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Dog> .
<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Cat> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Dog))
Declaration(Class(:Cat))
Declaration(NamedIndividual(:a))
Declaration(NamedIndividual(:b))
SameIndividual(:a :b)
)",
    );
    // The canonical individual (lower TermId) should acquire both types.
    // At least one of a or b should be inferred as the type it didn't originally have.
    let has_cross_type = inferred.contains("Dog") || inferred.contains("Cat");
    assert!(
        has_cross_type,
        "SameIndividual should unify types across individuals: {inferred}"
    );
}

#[test]
fn same_individual_axiom_nary() {
    // SameIndividual(a, b, c) — all three should be merged.
    let inferred = reason(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/X> .
<http://x.com/c> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Y> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:X))
Declaration(Class(:Y))
Declaration(NamedIndividual(:a))
Declaration(NamedIndividual(:b))
Declaration(NamedIndividual(:c))
SameIndividual(:a :b :c)
)",
    );
    // a and c are merged (transitively through b), so types should be shared.
    assert!(
        inferred.contains("sameAs"),
        "n-ary SameIndividual should produce sameAs triples: {inferred}"
    );
}

// ─── DifferentIndividuals axiom ────────────────────────────────────────────

#[test]
fn different_individuals_merged_is_inconsistent() {
    // DifferentIndividuals(a, b) + sameAs merge → inconsistency.
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/a> <http://www.w3.org/2002/07/owl#sameAs> <http://x.com/b> .
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(NamedIndividual(:a))
Declaration(NamedIndividual(:b))
DifferentIndividuals(:a :b)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("DifferentIndividuals"),
        "should detect merged different individuals: {report_json}"
    );
}

#[test]
fn different_individuals_no_conflict() {
    // DifferentIndividuals(a, b) with no equality merge → no inconsistency.
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/C> .
<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/D> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(NamedIndividual(:a))
Declaration(NamedIndividual(:b))
DifferentIndividuals(:a :b)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("\"inconsistencies\": []"),
        "no inconsistency when different individuals remain separate: {report_json}"
    );
}

#[test]
fn different_individuals_nary() {
    // DifferentIndividuals(a, b, c) — merging any pair is inconsistent.
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/b> <http://www.w3.org/2002/07/owl#sameAs> <http://x.com/c> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(NamedIndividual(:a))
Declaration(NamedIndividual(:b))
Declaration(NamedIndividual(:c))
DifferentIndividuals(:a :b :c)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("DifferentIndividuals"),
        "should detect merged pair from n-ary DifferentIndividuals: {report_json}"
    );
}

// ─── NegativePropertyAssertion ─────────────────────────────────────────────

#[test]
fn negative_object_property_assertion_detected() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/a> <http://x.com/likes> <http://x.com/b> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:likes))
Declaration(NamedIndividual(:a))
Declaration(NamedIndividual(:b))
NegativeObjectPropertyAssertion(:likes :a :b)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("NegativePropertyAssertion"),
        "should detect contradictory positive and negative assertion: {report_json}"
    );
}

#[test]
fn negative_object_property_assertion_no_conflict() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/a> <http://x.com/likes> <http://x.com/c> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(ObjectProperty(:likes))
Declaration(NamedIndividual(:a))
Declaration(NamedIndividual(:b))
NegativeObjectPropertyAssertion(:likes :a :b)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("\"inconsistencies\": []"),
        "no inconsistency when negated triple not present: {report_json}"
    );
}

#[test]
fn negative_data_property_assertion_detected() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let _ = reason_with_report(
        "\
<http://x.com/a> <http://x.com/age> \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://x.com/o>
Declaration(DataProperty(:age))
Declaration(NamedIndividual(:a))
NegativeDataPropertyAssertion(:age :a \"42\"^^xsd:integer)
)",
        &report,
        &[],
    );
    let report_json = fs::read_to_string(&report).unwrap();
    assert!(
        report_json.contains("NegativePropertyAssertion"),
        "should detect contradictory positive and negative data assertion: {report_json}"
    );
}

// ─── HasKey ────────────────────────────────────────────────────────────────

#[test]
fn has_key_produces_sameas() {
    // HasKey(C, [P]): two instances of C with same P value are merged.
    let inferred = reason(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Person> .
<http://x.com/a> <http://x.com/ssn> \"123\" .
<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Person> .
<http://x.com/b> <http://x.com/ssn> \"123\" .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Person))
Declaration(DataProperty(:ssn))
HasKey(:Person () (:ssn))
)",
    );
    assert!(
        inferred.contains("sameAs"),
        "HasKey should merge instances with same key: {inferred}"
    );
}

#[test]
fn has_key_no_merge_different_values() {
    // HasKey(C, [P]): different P values → no merge.
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report = temp_dir.path().join("report.json");
    let inferred = reason_with_report(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Person> .
<http://x.com/a> <http://x.com/ssn> \"123\" .
<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Person> .
<http://x.com/b> <http://x.com/ssn> \"456\" .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Person))
Declaration(DataProperty(:ssn))
HasKey(:Person () (:ssn))
)",
        &report,
        &[],
    );
    assert!(
        !inferred.contains("sameAs"),
        "different key values should not merge: {inferred}"
    );
}

#[test]
fn has_key_no_merge_wrong_class() {
    // HasKey(C, [P]): instance not of class C should not participate.
    let inferred = reason(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Person> .
<http://x.com/a> <http://x.com/ssn> \"123\" .
<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Robot> .
<http://x.com/b> <http://x.com/ssn> \"123\" .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Person))
Declaration(Class(:Robot))
Declaration(DataProperty(:ssn))
HasKey(:Person () (:ssn))
)",
    );
    assert!(
        !inferred.contains("sameAs"),
        "HasKey should not merge instances of different classes: {inferred}"
    );
}

#[test]
fn has_key_missing_property_no_merge() {
    // HasKey(C, [P]): instance missing key property should not participate.
    let inferred = reason(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Person> .
<http://x.com/a> <http://x.com/ssn> \"123\" .
<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Person> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Person))
Declaration(DataProperty(:ssn))
HasKey(:Person () (:ssn))
)",
    );
    assert!(
        !inferred.contains("sameAs"),
        "HasKey should not merge when key property is missing: {inferred}"
    );
}

#[test]
fn has_key_object_property() {
    // HasKey with object property keys.
    let inferred = reason(
        "\
<http://x.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Employee> .
<http://x.com/a> <http://x.com/worksFor> <http://x.com/acme> .
<http://x.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://x.com/Employee> .
<http://x.com/b> <http://x.com/worksFor> <http://x.com/acme> .
",
        "\
Prefix(:=<http://x.com/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://x.com/o>
Declaration(Class(:Employee))
Declaration(ObjectProperty(:worksFor))
HasKey(:Employee (:worksFor) ())
)",
    );
    assert!(
        inferred.contains("sameAs"),
        "HasKey with object property should merge: {inferred}"
    );
}

// ─── SWRL parsing ──────────────────────────────────────────────────────────

#[test]
fn swrl_class_atom_rule_parses() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report_path = temp_dir.path().join("report.json");

    let _inferred = reason_with_report(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           DLSafeRule(\n\
             Body(ClassAtom(:A Variable(<urn:swrl:var#x>)))\n\
             Head(ClassAtom(:B Variable(<urn:swrl:var#x>)))\n\
           )\n\
         )",
        &report_path,
        &[],
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&report_path).unwrap()).unwrap();
    let unsupported = report["rules"]["unsupported_encountered"]
        .as_array()
        .unwrap();
    assert!(
        !unsupported
            .iter()
            .any(|v| v.as_str().unwrap().contains("SWRL")),
        "SWRL class atom rule should parse without unsupported warnings: {unsupported:?}"
    );
}

#[test]
fn swrl_property_atom_rule_parses() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report_path = temp_dir.path().join("report.json");

    let _inferred = reason_with_report(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           DLSafeRule(\n\
             Body(ObjectPropertyAtom(:P Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>)))\n\
             Head(ObjectPropertyAtom(:Q Variable(<urn:swrl:var#y>) Variable(<urn:swrl:var#x>)))\n\
           )\n\
         )",
        &report_path,
        &[],
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&report_path).unwrap()).unwrap();
    let unsupported = report["rules"]["unsupported_encountered"]
        .as_array()
        .unwrap();
    assert!(
        !unsupported
            .iter()
            .any(|v| v.as_str().unwrap().contains("SWRL")),
        "SWRL property atom rule should parse without unsupported warnings: {unsupported:?}"
    );
}

#[test]
fn swrl_multi_atom_rule_parses() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report_path = temp_dir.path().join("report.json");

    let _inferred = reason_with_report(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n\
         <http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           DLSafeRule(\n\
             Body(\n\
               ClassAtom(:A Variable(<urn:swrl:var#x>))\n\
               ObjectPropertyAtom(:P Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>))\n\
             )\n\
             Head(ClassAtom(:B Variable(<urn:swrl:var#y>)))\n\
           )\n\
         )",
        &report_path,
        &[],
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&report_path).unwrap()).unwrap();
    let unsupported = report["rules"]["unsupported_encountered"]
        .as_array()
        .unwrap();
    assert!(
        !unsupported
            .iter()
            .any(|v| v.as_str().unwrap().contains("SWRL")),
        "SWRL multi-atom rule should parse without unsupported warnings: {unsupported:?}"
    );
}

#[test]
fn swrl_data_property_atom_flagged_unsupported() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report_path = temp_dir.path().join("report.json");

    let _inferred = reason_with_report(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)\n\
         Ontology(\n\
           DLSafeRule(\n\
             Body(DataPropertyAtom(:dp Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#v>)))\n\
             Head(ClassAtom(:B Variable(<urn:swrl:var#x>)))\n\
           )\n\
         )",
        &report_path,
        &[],
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&report_path).unwrap()).unwrap();
    let unsupported = report["rules"]["unsupported_encountered"]
        .as_array()
        .unwrap();
    assert!(
        unsupported
            .iter()
            .any(|v| v.as_str().unwrap().contains("DataPropertyAtom")),
        "SWRL DataPropertyAtom should be flagged unsupported: {unsupported:?}"
    );
}

#[test]
fn swrl_same_individual_atom_parses() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report_path = temp_dir.path().join("report.json");

    let _inferred = reason_with_report(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           DLSafeRule(\n\
             Body(ObjectPropertyAtom(:P Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>)))\n\
             Head(SameIndividualAtom(Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>)))\n\
           )\n\
         )",
        &report_path,
        &[],
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&report_path).unwrap()).unwrap();
    let unsupported = report["rules"]["unsupported_encountered"]
        .as_array()
        .unwrap();
    assert!(
        !unsupported
            .iter()
            .any(|v| v.as_str().unwrap().contains("SWRL")),
        "SWRL SameIndividualAtom rule should parse: {unsupported:?}"
    );
}

#[test]
fn swrl_different_individuals_atom_in_body_parses() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report_path = temp_dir.path().join("report.json");

    let _inferred = reason_with_report(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           DLSafeRule(\n\
             Body(\n\
               ObjectPropertyAtom(:P Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>))\n\
               DifferentIndividualsAtom(Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>))\n\
             )\n\
             Head(ClassAtom(:Distinct Variable(<urn:swrl:var#x>)))\n\
           )\n\
         )",
        &report_path,
        &[],
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&report_path).unwrap()).unwrap();
    let unsupported = report["rules"]["unsupported_encountered"]
        .as_array()
        .unwrap();
    assert!(
        !unsupported
            .iter()
            .any(|v| v.as_str().unwrap().contains("SWRL")),
        "SWRL DifferentIndividualsAtom in body should parse: {unsupported:?}"
    );
}

#[test]
fn swrl_different_individuals_atom_in_head_parses() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report_path = temp_dir.path().join("report.json");

    let _inferred = reason_with_report(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           DLSafeRule(\n\
             Body(ObjectPropertyAtom(:P Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>)))\n\
             Head(DifferentIndividualsAtom(Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>)))\n\
           )\n\
         )",
        &report_path,
        &[],
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&report_path).unwrap()).unwrap();
    let unsupported = report["rules"]["unsupported_encountered"]
        .as_array()
        .unwrap();
    assert!(
        !unsupported
            .iter()
            .any(|v| v.as_str().unwrap().contains("SWRL")),
        "SWRL DifferentIndividualsAtom in head should parse: {unsupported:?}"
    );
}

// ─── SWRL inference tests ───────────────────────────────────────────────────

#[test]
fn swrl_class_to_class_inference() {
    // C(?x) → D(?x)  (similar to SubClassOf but via SWRL)
    let inferred = reason(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/C> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:C))\n\
           Declaration(Class(:D))\n\
           DLSafeRule(\n\
             Body(ClassAtom(:C Variable(<urn:swrl:var#x>)))\n\
             Head(ClassAtom(:D Variable(<urn:swrl:var#x>)))\n\
           )\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/D> ."),
        "SWRL C(?x) → D(?x) should infer alice:D: {inferred}"
    );
}

#[test]
fn swrl_property_to_inverse_property_inference() {
    // P(?x,?y) → Q(?y,?x)  (similar to InverseOf but via SWRL)
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(ObjectProperty(:P))\n\
           Declaration(ObjectProperty(:Q))\n\
           DLSafeRule(\n\
             Body(ObjectPropertyAtom(:P Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>)))\n\
             Head(ObjectPropertyAtom(:Q Variable(<urn:swrl:var#y>) Variable(<urn:swrl:var#x>)))\n\
           )\n\
         )",
    );
    assert!(
        inferred.contains(
            "<http://example.com/bob> <http://example.com/Q> <http://example.com/alice> ."
        ),
        "SWRL P(?x,?y) → Q(?y,?x) should infer bob Q alice: {inferred}"
    );
}

#[test]
fn swrl_multi_atom_join_inference() {
    // C(?x) ∧ P(?x,?y) → D(?y)
    let inferred = reason(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/C> .\n\
         <http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:C))\n\
           Declaration(Class(:D))\n\
           Declaration(ObjectProperty(:P))\n\
           DLSafeRule(\n\
             Body(\n\
               ClassAtom(:C Variable(<urn:swrl:var#x>))\n\
               ObjectPropertyAtom(:P Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>))\n\
             )\n\
             Head(ClassAtom(:D Variable(<urn:swrl:var#y>)))\n\
           )\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/D> ."),
        "SWRL C(?x) ∧ P(?x,?y) → D(?y) should infer bob:D: {inferred}"
    );
}

#[test]
fn swrl_multi_atom_join_missing_class() {
    // C(?x) ∧ P(?x,?y) → D(?y) — alice is NOT C, so no inference
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:C))\n\
           Declaration(Class(:D))\n\
           Declaration(ObjectProperty(:P))\n\
           DLSafeRule(\n\
             Body(\n\
               ClassAtom(:C Variable(<urn:swrl:var#x>))\n\
               ObjectPropertyAtom(:P Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>))\n\
             )\n\
             Head(ClassAtom(:D Variable(<urn:swrl:var#y>)))\n\
           )\n\
         )",
    );
    assert!(
        !inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/D> ."),
        "SWRL should not fire when class condition not met: {inferred}"
    );
}

#[test]
fn swrl_same_individual_head_produces_equality() {
    // P(?x,?y) → SameIndividual(?x,?y)
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n\
         <http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/C> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(ObjectProperty(:P))\n\
           DLSafeRule(\n\
             Body(ObjectPropertyAtom(:P Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>)))\n\
             Head(SameIndividualAtom(Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>)))\n\
           )\n\
         )",
    );
    // SWRL SameIndividual head should produce owl:sameAs and unify facts
    assert!(
        inferred.contains("<http://example.com/alice> <http://www.w3.org/2002/07/owl#sameAs> <http://example.com/bob> .")
        || inferred.contains("<http://example.com/bob> <http://www.w3.org/2002/07/owl#sameAs> <http://example.com/alice> ."),
        "SWRL SameIndividual head should produce sameAs: {inferred}"
    );
    // Equality reasoning should propagate bob's type to alice
    assert!(
        inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/C> ."),
        "SWRL SameIndividual should unify types: {inferred}"
    );
}

#[test]
fn swrl_different_individuals_head_inconsistency() {
    // P(?x,?y) → DifferentIndividuals(?x,?y)
    // Combined with SameIndividual(alice, bob) → inconsistency
    let temp_dir = tempfile::TempDir::new().unwrap();
    let report_path = temp_dir.path().join("report.json");

    let _inferred = reason_with_report(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Prefix(owl:=<http://www.w3.org/2002/07/owl#>)\n\
         Ontology(\n\
           Declaration(ObjectProperty(:P))\n\
           SameIndividual(:alice :bob)\n\
           DLSafeRule(\n\
             Body(ObjectPropertyAtom(:P Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>)))\n\
             Head(DifferentIndividualsAtom(Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>)))\n\
           )\n\
         )",
        &report_path,
        &["--inconsistency-mode", "report"],
    );

    let report_json = fs::read_to_string(&report_path).unwrap();
    assert!(
        report_json.contains("DifferentIndividuals"),
        "SWRL DifferentIndividuals head should produce inconsistency when merged: {report_json}"
    );
}

#[test]
fn swrl_chained_inference() {
    // C(?x) → D(?x), D(?x) → E(?x) — two SWRL rules chaining
    let inferred = reason(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/C> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:C))\n\
           Declaration(Class(:D))\n\
           Declaration(Class(:E))\n\
           DLSafeRule(\n\
             Body(ClassAtom(:C Variable(<urn:swrl:var#x>)))\n\
             Head(ClassAtom(:D Variable(<urn:swrl:var#x>)))\n\
           )\n\
           DLSafeRule(\n\
             Body(ClassAtom(:D Variable(<urn:swrl:var#x>)))\n\
             Head(ClassAtom(:E Variable(<urn:swrl:var#x>)))\n\
           )\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/D> ."),
        "first SWRL rule should fire: {inferred}"
    );
    assert!(
        inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/E> ."),
        "chained SWRL rule should fire: {inferred}"
    );
}

#[test]
fn swrl_constant_in_head() {
    // P(?x,?y) → ClassAtom(D, ?x) — class constant in head
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(ObjectProperty(:P))\n\
           Declaration(Class(:D))\n\
           DLSafeRule(\n\
             Body(ObjectPropertyAtom(:P Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>)))\n\
             Head(ClassAtom(:D Variable(<urn:swrl:var#x>)))\n\
           )\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/D> ."),
        "SWRL with property trigger and class head should fire: {inferred}"
    );
}

// ─── Anonymous CE positions ─────────────────────────────────────────────────

#[test]
fn anon_ce_both_anonymous_subclassof() {
    // SubClassOf(SomeValuesFrom(:P, :A), AllValuesFrom(:Q, :B))
    // property(x,P,y) ∧ type(y,A) → [type(x,proxy1)] ∧ [type(x,proxy1) → type(z,B) for property(x,Q,z)]
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n\
         <http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n\
         <http://example.com/alice> <http://example.com/Q> <http://example.com/carol> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(ObjectProperty(:P))\n\
           Declaration(ObjectProperty(:Q))\n\
           SubClassOf(ObjectSomeValuesFrom(:P :A) ObjectAllValuesFrom(:Q :B))\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/carol> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> ."),
        "Both-anonymous SubClassOf should infer type B on carol: {inferred}"
    );
}

#[test]
fn anon_ce_domain_anonymous() {
    // ObjectPropertyDomain(:P, AllValuesFrom(:Q, :A))
    // property(x,P,_) → type(x,proxy) and type(x,proxy) ∧ property(x,Q,y) → type(y,A)
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n\
         <http://example.com/alice> <http://example.com/Q> <http://example.com/carol> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(ObjectProperty(:P))\n\
           Declaration(ObjectProperty(:Q))\n\
           ObjectPropertyDomain(:P ObjectAllValuesFrom(:Q :A))\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/carol> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> ."),
        "Anonymous domain should infer type A on carol: {inferred}"
    );
}

#[test]
fn anon_ce_range_anonymous() {
    // ObjectPropertyRange(:P, IntersectionOf(:A, :B))
    // property(_,P,y) → type(y,proxy) → type(y,A) ∧ type(y,B)
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(ObjectProperty(:P))\n\
           ObjectPropertyRange(:P ObjectIntersectionOf(:A :B))\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> ."),
        "Anonymous range should infer type A: {inferred}"
    );
    assert!(
        inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> ."),
        "Anonymous range should infer type B: {inferred}"
    );
}

#[test]
fn anon_ce_class_assertion_named() {
    // ClassAssertion(:A, :alice) from ontology
    let inferred = reason(
        "",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(NamedIndividual(:alice))\n\
           SubClassOf(:A :B)\n\
           ClassAssertion(:A :alice)\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> ."),
        "ClassAssertion should produce type assertion and chain: {inferred}"
    );
}

#[test]
fn anon_ce_class_assertion_anonymous() {
    // ClassAssertion(AllValuesFrom(:P, :B), :alice)
    // type(alice, proxy) → type(alice, proxy) ∧ property(alice,P,y) → type(y,B)
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:B))\n\
           Declaration(ObjectProperty(:P))\n\
           Declaration(NamedIndividual(:alice))\n\
           ClassAssertion(ObjectAllValuesFrom(:P :B) :alice)\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> ."),
        "Anonymous ClassAssertion should produce inferences: {inferred}"
    );
}

#[test]
fn swrl_anonymous_ce_in_body() {
    // SWRL: ClassAtom(SomeValuesFrom(:P, :A), ?x) → ClassAtom(:B, ?x)
    // Reified: type(?x, proxy) → type(?x, B) where proxy ← SomeValuesFrom(:P, :A)
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n\
         <http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(ObjectProperty(:P))\n\
           DLSafeRule(\n\
             Body(ClassAtom(ObjectSomeValuesFrom(:P :A) Variable(<urn:swrl:var#x>)))\n\
             Head(ClassAtom(:B Variable(<urn:swrl:var#x>)))\n\
           )\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> ."),
        "SWRL with anonymous CE in body should fire: {inferred}"
    );
}

#[test]
fn swrl_anonymous_ce_in_head() {
    // SWRL: ClassAtom(:A, ?x) → ClassAtom(IntersectionOf(:B, :C), ?x)
    // Reified: type(?x, A) → type(?x, proxy) → type(?x, B) ∧ type(?x, C)
    let inferred = reason(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(Class(:C))\n\
           DLSafeRule(\n\
             Body(ClassAtom(:A Variable(<urn:swrl:var#x>)))\n\
             Head(ClassAtom(ObjectIntersectionOf(:B :C) Variable(<urn:swrl:var#x>)))\n\
           )\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> ."),
        "SWRL with anonymous CE in head should infer B: {inferred}"
    );
    assert!(
        inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/C> ."),
        "SWRL with anonymous CE in head should infer C: {inferred}"
    );
}

// ─── Nested anonymous CEs ───────────────────────────────────────────────────

#[test]
fn nested_anon_ce_all_values_from_intersection() {
    // SubClassOf(:A, AllValuesFrom(:P, IntersectionOf(:B, :C)))
    // type(x,A) ∧ property(x,P,y) → type(y,B) ∧ type(y,C)
    let inferred = reason(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n\
         <http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(Class(:C))\n\
           Declaration(ObjectProperty(:P))\n\
           SubClassOf(:A ObjectAllValuesFrom(:P ObjectIntersectionOf(:B :C)))\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> ."),
        "Nested IntersectionOf filler in AllValuesFrom should infer B: {inferred}"
    );
    assert!(
        inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/C> ."),
        "Nested IntersectionOf filler in AllValuesFrom should infer C: {inferred}"
    );
}

#[test]
fn nested_anon_ce_some_values_from_intersection() {
    // SubClassOf(SomeValuesFrom(:P, IntersectionOf(:A, :B)), :C)
    // property(x,P,y) ∧ type(y,A) ∧ type(y,B) → type(x,C)
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n\
         <http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n\
         <http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(Class(:C))\n\
           Declaration(ObjectProperty(:P))\n\
           SubClassOf(ObjectSomeValuesFrom(:P ObjectIntersectionOf(:A :B)) :C)\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/C> ."),
        "Nested IntersectionOf filler in SomeValuesFrom should infer C: {inferred}"
    );
}

#[test]
fn nested_anon_ce_some_values_from_intersection_missing() {
    // Same as above but bob only has type A (not B) — should NOT infer C
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n\
         <http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(Class(:C))\n\
           Declaration(ObjectProperty(:P))\n\
           SubClassOf(ObjectSomeValuesFrom(:P ObjectIntersectionOf(:A :B)) :C)\n\
         )",
    );
    assert!(
        !inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/C> ."),
        "Nested IntersectionOf should not fire when only one conjunct matched: {inferred}"
    );
}

#[test]
fn nested_anon_ce_complement_of_some_values_from() {
    // SubClassOf(:A, ComplementOf(SomeValuesFrom(:P, :B)))
    // type(x,A) ∧ type(x,proxy) → inconsistency (where proxy ← SomeValuesFrom(:P,:B))
    // That is: if x is type A and x has a P-link to some B, that's inconsistent.
    let inferred = reason(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n\
         <http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n\
         <http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(ObjectProperty(:P))\n\
           SubClassOf(:A ObjectComplementOf(ObjectSomeValuesFrom(:P :B)))\n\
         )",
    );
    // Should detect inconsistency — A and SomeValuesFrom(P,B) are disjoint
    // but alice is A and has P-link to bob who is B.
    // The output should be empty or have an inconsistency (check report).
    // For now just verify we don't crash and the proxy mechanism works.
    let _ = inferred;
}

// ─── ObjectInverseOf ────────────────────────────────────────────────────────

#[test]
fn inverse_of_in_all_values_from() {
    // SubClassOf(:A, AllValuesFrom(InverseOf(:P), :B))
    // type(x,A) ∧ property(y,P,x) → type(y,B)
    let inferred = reason(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n\
         <http://example.com/bob> <http://example.com/P> <http://example.com/alice> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(ObjectProperty(:P))\n\
           SubClassOf(:A ObjectAllValuesFrom(ObjectInverseOf(:P) :B))\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> ."),
        "InverseOf in AllValuesFrom should infer type: {inferred}"
    );
}

#[test]
fn inverse_of_functional_property() {
    // FunctionalObjectProperty(InverseOf(:P))
    // Means the inverse of P is functional: at most one subject per object.
    // property(x,P,z) ∧ property(y,P,z) → sameAs(x,y)
    let inferred = reason(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/target> .\n\
         <http://example.com/bob> <http://example.com/P> <http://example.com/target> .\n\
         <http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(ObjectProperty(:P))\n\
           FunctionalObjectProperty(ObjectInverseOf(:P))\n\
         )",
    );
    // Functional InverseOf(P) makes the proxy property functional.
    // property(alice, P, target) → property(target, proxy, alice)
    // property(bob, P, target) → property(target, proxy, bob)
    // Functional proxy: target has two proxy-values → sameAs(alice, bob)
    // So bob should get type A from alice.
    assert!(
        inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> ."),
        "FunctionalObjectProperty(InverseOf(:P)) should merge subjects: {inferred}"
    );
}

#[test]
fn inverse_of_in_property_chain() {
    // SubObjectPropertyOf(ObjectPropertyChain(InverseOf(:P) :Q) :R)
    // InverseOf(P)(x,y) ∧ Q(y,z) → R(x,z)
    // P(bob,alice) → InverseOf(P)(alice,bob), then Q(bob,carol) → R(alice,carol)
    let inferred = reason(
        "<http://example.com/bob> <http://example.com/P> <http://example.com/alice> .\n\
         <http://example.com/bob> <http://example.com/Q> <http://example.com/carol> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(ObjectProperty(:P))\n\
           Declaration(ObjectProperty(:Q))\n\
           Declaration(ObjectProperty(:R))\n\
           SubObjectPropertyOf(ObjectPropertyChain(ObjectInverseOf(:P) :Q) :R)\n\
         )",
    );
    assert!(
        inferred.contains(
            "<http://example.com/alice> <http://example.com/R> <http://example.com/carol> ."
        ),
        "InverseOf in property chain should infer: {inferred}"
    );
}

#[test]
fn inverse_of_in_some_values_from() {
    // SubClassOf(SomeValuesFrom(InverseOf(:P), :A), :B)
    // InverseOf(P)(x,y) ∧ type(y,A) → type(x,B)
    // P(bob,alice) → InverseOf(P)(alice,bob), type(bob,A) → type(alice,B)
    let inferred = reason(
        "<http://example.com/bob> <http://example.com/P> <http://example.com/alice> .\n\
         <http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(ObjectProperty(:P))\n\
           SubClassOf(ObjectSomeValuesFrom(ObjectInverseOf(:P) :A) :B)\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> ."),
        "InverseOf in SomeValuesFrom should infer type: {inferred}"
    );
}

#[test]
fn inverse_of_in_swrl_property_atom() {
    // SWRL: ObjectPropertyAtom(InverseOf(:P), ?x, ?y) → ClassAtom(:A, ?y)
    // property(y,P,x) → type(y,A)
    let inferred = reason(
        "<http://example.com/bob> <http://example.com/P> <http://example.com/alice> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(ObjectProperty(:P))\n\
           DLSafeRule(\n\
             Body(ObjectPropertyAtom(ObjectInverseOf(:P) Variable(<urn:swrl:var#x>) Variable(<urn:swrl:var#y>)))\n\
             Head(ClassAtom(:A Variable(<urn:swrl:var#y>)))\n\
           )\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> ."),
        "InverseOf in SWRL ObjectPropertyAtom should fire: {inferred}"
    );
}

// ─── DisjointClasses with anonymous CE ──────────────────────────────────────

#[test]
fn anon_ce_disjoint_classes_anonymous() {
    // DisjointClasses(SomeValuesFrom(:P, :A), :B)
    // If x has P-link to an A, x is type proxy (reified SomeValuesFrom).
    // proxy is disjoint with B, so x can't also be B.
    let (inferred, report) = reason_check_inconsistency(
        "<http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n\
         <http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n\
         <http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(ObjectProperty(:P))\n\
           DisjointClasses(ObjectSomeValuesFrom(:P :A) :B)\n\
         )",
    );
    assert!(
        report.contains("DisjointClasses"),
        "DisjointClasses with anonymous CE should detect inconsistency: {report}"
    );
    let _ = inferred;
}

// ─── HasKey with anonymous CE ───────────────────────────────────────────────

#[test]
fn anon_ce_has_key_anonymous() {
    // HasKey(IntersectionOf(:A, :B), [:P])
    // alice and bob are both A∧B with the same P value → sameAs
    let inferred = reason(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n\
         <http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> .\n\
         <http://example.com/alice> <http://example.com/P> <http://example.com/val> .\n\
         <http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n\
         <http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> .\n\
         <http://example.com/bob> <http://example.com/P> <http://example.com/val> .\n\
         <http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/C> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(Class(:C))\n\
           Declaration(ObjectProperty(:P))\n\
           HasKey(ObjectIntersectionOf(:A :B) (:P) ())\n\
         )",
    );
    // alice and bob should be merged, so bob gets type C
    assert!(
        inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/C> ."),
        "HasKey with anonymous CE should merge individuals: {inferred}"
    );
}

// ─── ObjectPropertyAssertion / DataPropertyAssertion from ontology ──────────

#[test]
fn ontology_object_property_assertion() {
    // ObjectPropertyAssertion(:P, :alice, :bob) from ontology + SubClassOf with domain
    let inferred = reason(
        "",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:C))\n\
           Declaration(ObjectProperty(:P))\n\
           Declaration(NamedIndividual(:alice))\n\
           Declaration(NamedIndividual(:bob))\n\
           ObjectPropertyDomain(:P :C)\n\
           ObjectPropertyAssertion(:P :alice :bob)\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/C> ."),
        "OPA from ontology should trigger domain inference: {inferred}"
    );
}

#[test]
fn ontology_data_property_assertion() {
    // DataPropertyAssertion(:dp, :alice, "hello") from ontology + domain
    let inferred = reason(
        "",
        "Prefix(:=<http://example.com/>)\n\
         Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)\n\
         Ontology(\n\
           Declaration(Class(:C))\n\
           Declaration(DataProperty(:dp))\n\
           Declaration(NamedIndividual(:alice))\n\
           DataPropertyDomain(:dp :C)\n\
           DataPropertyAssertion(:dp :alice \"hello\"^^xsd:string)\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/C> ."),
        "DPA from ontology should trigger domain inference: {inferred}"
    );
}

// ─── OneOf subclass consumption (pre-existing bug fix) ──────────────────────

#[test]
fn one_of_subclass_types_consumed() {
    // SubClassOf(OneOf(:a, :b), :C) should produce type(a,C) and type(b,C)
    let inferred = reason(
        "",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:C))\n\
           Declaration(Class(:D))\n\
           Declaration(NamedIndividual(:a))\n\
           Declaration(NamedIndividual(:b))\n\
           SubClassOf(ObjectOneOf(:a :b) :C)\n\
           SubClassOf(:C :D)\n\
         )",
    );
    assert!(
        inferred.contains("<http://example.com/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/D> ."),
        "OneOf subclass types should be consumed and chain: {inferred}"
    );
    assert!(
        inferred.contains("<http://example.com/b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/D> ."),
        "OneOf subclass types should be consumed and chain for b: {inferred}"
    );
}

// ─── Proxy filtering ────────────────────────────────────────────────────────

#[test]
fn proxy_terms_not_in_output() {
    // Use anonymous CEs that create proxies, verify no urn:strix:anon: in output
    let inferred = reason(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n\
         <http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(ObjectProperty(:P))\n\
           SubClassOf(:A ObjectAllValuesFrom(:P :B))\n\
         )",
    );
    assert!(
        !inferred.contains("urn:strix:anon:"),
        "Proxy terms should not appear in output: {inferred}"
    );
    assert!(
        inferred.contains("<http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> ."),
        "Real inferences should still appear: {inferred}"
    );
}

#[test]
fn proxy_terms_not_in_output_inverse_of() {
    // ObjectInverseOf creates a proxy property — verify it's filtered
    let inferred = reason(
        "<http://example.com/bob> <http://example.com/P> <http://example.com/alice> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(ObjectProperty(:P))\n\
           SubClassOf(ObjectSomeValuesFrom(ObjectInverseOf(:P) ObjectComplementOf(ObjectComplementOf(:A))) :B)\n\
         )",
    );
    assert!(
        !inferred.contains("urn:strix:anon:"),
        "Proxy terms from InverseOf should not appear in output: {inferred}"
    );
}

// ─── Complement inconsistency (fix for non-asserting test) ──────────────────

#[test]
fn nested_anon_ce_complement_of_some_values_from_inconsistency() {
    // SubClassOf(:A, ComplementOf(SomeValuesFrom(:P, :B)))
    // alice is A and has P-link to bob who is B → inconsistency
    let (_inferred, report) = reason_check_inconsistency(
        "<http://example.com/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/A> .\n\
         <http://example.com/alice> <http://example.com/P> <http://example.com/bob> .\n\
         <http://example.com/bob> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.com/B> .\n",
        "Prefix(:=<http://example.com/>)\n\
         Ontology(\n\
           Declaration(Class(:A))\n\
           Declaration(Class(:B))\n\
           Declaration(ObjectProperty(:P))\n\
           SubClassOf(:A ObjectComplementOf(ObjectSomeValuesFrom(:P :B)))\n\
         )",
    );
    assert!(
        report.contains("ComplementOf") || report.contains("DisjointClasses"),
        "ComplementOf(SomeValuesFrom) should detect inconsistency: {report}"
    );
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

/// Like `reason` but also returns the JSON report (for inconsistency checking).
/// Uses `--inconsistency-mode report` so the run doesn't bail on inconsistency.
fn reason_check_inconsistency(data: &str, ontology: &str) -> (String, String) {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let report_path = temp_dir.path().join("report.json");
    let inferred = reason_with_report(
        data,
        ontology,
        &report_path,
        &["--inconsistency-mode", "report"],
    );
    let report = fs::read_to_string(&report_path).unwrap_or_default();
    (inferred, report)
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

/// Like `reason` but also writes a report and accepts extra CLI args.
fn reason_with_report(
    data: &str,
    ontology: &str,
    report_path: &Path,
    extra_args: &[&str],
) -> String {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data_path = temp_dir.path().join("data.nt");
    let ontology_path = temp_dir.path().join("ontology.ofn");
    let output_path = temp_dir.path().join("inferred.nt");

    write(&data_path, data);
    write(&ontology_path, ontology);

    let mut args = vec![
        "strix".to_string(),
        "reason".to_string(),
        data_path.to_str().unwrap().to_string(),
        "--ontology".to_string(),
        ontology_path.to_str().unwrap().to_string(),
        "--output".to_string(),
        output_path.to_str().unwrap().to_string(),
        "--report".to_string(),
        report_path.to_str().unwrap().to_string(),
    ];
    for arg in extra_args {
        args.push(arg.to_string());
    }

    strix::run(args).expect("reasoning run should succeed");

    fs::read_to_string(&output_path).expect("output should exist")
}

/// Like `reason` but returns the Result instead of unwrapping.
fn reason_expecting_result(data: &str, ontology: &str, extra_args: &[&str]) -> anyhow::Result<()> {
    let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
    let data_path = temp_dir.path().join("data.nt");
    let ontology_path = temp_dir.path().join("ontology.ofn");
    let output_path = temp_dir.path().join("inferred.nt");

    write(&data_path, data);
    write(&ontology_path, ontology);

    let mut args = vec![
        "strix".to_string(),
        "reason".to_string(),
        data_path.to_str().unwrap().to_string(),
        "--ontology".to_string(),
        ontology_path.to_str().unwrap().to_string(),
        "--output".to_string(),
        output_path.to_str().unwrap().to_string(),
    ];
    for arg in extra_args {
        args.push(arg.to_string());
    }

    strix::run(args)
}
