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

fn write(path: &Path, content: &str) {
    fs::write(path, content).expect("test fixture should be written");
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
