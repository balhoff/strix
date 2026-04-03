use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::{Path, PathBuf};

use horned_owl::io::{ParserConfiguration, ofn, owx, rdf};
use horned_owl::model::{
    ClassExpression, Component, Kinded, ObjectPropertyExpression, RcStr,
    SubObjectPropertyExpression,
};
use horned_owl::ontology::set::SetOntology;
use oxrdfio::RdfFormat as OxRdfFormat;
use walkdir::WalkDir;

use crate::dict::{Dictionary, TermId};
use anyhow::Context;

use crate::error::Result;
use crate::rdf::Triple;

use super::{ExtractedSchema, RawSchema};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Compression {
    None,
    Gzip,
    Bzip2,
    Xz,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RdfOntologySyntax {
    NTriples,
    Turtle,
    RdfXml,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OntologySyntax {
    Rdf(RdfOntologySyntax),
    OwlXml,
    Functional,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OntologyInput {
    path: PathBuf,
    syntax: OntologySyntax,
    compression: Compression,
}

struct ParsedOntology {
    source: String,
    ontology: SetOntology<RcStr>,
    incomplete: Option<rdf::reader::IncompleteParse<RcStr>>,
}

pub fn load_ontology_path(
    path: &Path,
    dictionary: &mut Dictionary,
    schema: &mut RawSchema,
    ignore_annotation_axioms: bool,
) -> Result<()> {
    for input in discover_ontology_inputs(path)? {
        let parsed = parse_input(&input)?;
        absorb_parsed_ontology(parsed, dictionary, schema, ignore_annotation_axioms);
    }

    Ok(())
}

/// Normalize schema triples that were extracted from data during ingestion.
///
/// This round-trips extracted triples through horned-owl's RDF parser so that
/// both explicit ontology files and embedded schema axioms follow the same
/// lowering path. The serialize-then-reparse step is a temporary simplification;
/// a future revision should build the `SetOntology` directly from the extracted
/// triples, avoiding the intermediate N-Triples serialization.
pub fn load_extracted_schema(
    extracted: &ExtractedSchema,
    dictionary: &mut Dictionary,
    schema: &mut RawSchema,
    ignore_annotation_axioms: bool,
) -> Result<()> {
    if extracted.is_empty() {
        return Ok(());
    }

    let bytes = serialize_triples_to_ntriples(&extracted.triples);
    let parsed = parse_rdf_bytes(
        "extracted schema buffer",
        bytes,
        RdfOntologySyntax::NTriples,
    )?;
    absorb_parsed_ontology(parsed, dictionary, schema, ignore_annotation_axioms);
    Ok(())
}

fn discover_ontology_inputs(path: &Path) -> Result<Vec<OntologyInput>> {
    if !path.exists() {
        return Err(anyhow::anyhow!("path does not exist: {}", path.display()));
    }

    let mut inputs = Vec::new();
    if path.is_file() {
        let input = classify_input(path).ok_or_else(|| {
            anyhow::anyhow!("unsupported ontology input format: {}", path.display())
        })?;
        inputs.push(input);
    } else if path.is_dir() {
        for entry in WalkDir::new(path).follow_links(true) {
            let entry = entry.context(format!("failed to walk {}", path.display()))?;
            if entry.file_type().is_file()
                && let Some(input) = classify_input(entry.path())
            {
                inputs.push(input);
            }
        }
    } else {
        return Err(anyhow::anyhow!(
            "path is neither a file nor a directory: {}",
            path.display()
        ));
    }

    inputs.sort_by(|left, right| left.path.cmp(&right.path));
    if inputs.is_empty() {
        return Err(anyhow::anyhow!(
            "no supported ontology files found under {}",
            path.display()
        ));
    }

    Ok(inputs)
}

fn classify_input(path: &Path) -> Option<OntologyInput> {
    let (compression, base_path) = detect_compression(path);
    let syntax = detect_syntax(&base_path)?;
    Some(OntologyInput {
        path: path.to_path_buf(),
        syntax,
        compression,
    })
}

fn detect_compression(path: &Path) -> (Compression, PathBuf) {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("gz") {
        return (Compression::Gzip, path.with_extension(""));
    }
    if extension.eq_ignore_ascii_case("bz2") {
        return (Compression::Bzip2, path.with_extension(""));
    }
    if extension.eq_ignore_ascii_case("xz") {
        return (Compression::Xz, path.with_extension(""));
    }
    (Compression::None, path.to_path_buf())
}

fn detect_syntax(path: &Path) -> Option<OntologySyntax> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();

    if extension.eq_ignore_ascii_case("ofn") {
        return Some(OntologySyntax::Functional);
    }
    if extension.eq_ignore_ascii_case("owx") {
        return Some(OntologySyntax::OwlXml);
    }
    if extension.eq_ignore_ascii_case("nt") {
        return Some(OntologySyntax::Rdf(RdfOntologySyntax::NTriples));
    }
    if extension.eq_ignore_ascii_case("ttl") {
        return Some(OntologySyntax::Rdf(RdfOntologySyntax::Turtle));
    }
    if extension.eq_ignore_ascii_case("rdf") || extension.eq_ignore_ascii_case("owl") {
        return Some(OntologySyntax::Rdf(RdfOntologySyntax::RdfXml));
    }
    None
}

fn parse_input(input: &OntologyInput) -> Result<ParsedOntology> {
    let bytes = read_input_bytes(input)?;

    match input.syntax {
        OntologySyntax::OwlXml => {
            let mut reader = BufReader::new(Cursor::new(bytes));
            let config = ParserConfiguration::default();
            let (ontology, _) =
                owx::reader::read::<RcStr, SetOntology<RcStr>, _>(&mut reader, config).map_err(
                    |e| {
                        anyhow::anyhow!(
                            "failed to parse OWL/XML ontology {}: {e:?}",
                            input.path.display()
                        )
                    },
                )?;
            Ok(ParsedOntology {
                source: input.path.display().to_string(),
                ontology,
                incomplete: None,
            })
        }
        OntologySyntax::Functional => {
            let reader = BufReader::new(Cursor::new(bytes));
            let config = ParserConfiguration::default();
            let (ontology, _) = ofn::reader::read::<RcStr, SetOntology<RcStr>, _>(reader, config)
                .map_err(|e| {
                anyhow::anyhow!(
                    "failed to parse OWL Functional Syntax ontology {}: {e:?}",
                    input.path.display()
                )
            })?;
            Ok(ParsedOntology {
                source: input.path.display().to_string(),
                ontology,
                incomplete: None,
            })
        }
        OntologySyntax::Rdf(format) => {
            parse_rdf_bytes(&input.path.display().to_string(), bytes, format)
        }
    }
}

fn parse_rdf_bytes(
    source: &str,
    bytes: Vec<u8>,
    format: RdfOntologySyntax,
) -> Result<ParsedOntology> {
    let mut reader = BufReader::new(Cursor::new(bytes));
    let mut config = ParserConfiguration::default();
    config.rdf.format = Some(to_oxrdf_format(format));
    // Lax mode lets horned-owl infer entity kinds (ObjectProperty vs
    // DataProperty vs AnnotationProperty) when RDFS-only ontologies omit
    // explicit OWL type declarations.  Without it, bare rdfs:subPropertyOf
    // triples end up as unparsed residue.
    config.rdf.lax = true;

    let (ontology, incomplete) = rdf::reader::read(&mut reader, config)
        .map_err(|e| anyhow::anyhow!("failed to parse RDF ontology {source}: {e:?}"))?;
    let incomplete = if incomplete.is_complete() {
        None
    } else {
        Some(incomplete)
    };

    Ok(ParsedOntology {
        source: source.to_string(),
        ontology: ontology.into(),
        incomplete,
    })
}

fn to_oxrdf_format(format: RdfOntologySyntax) -> OxRdfFormat {
    match format {
        RdfOntologySyntax::NTriples => OxRdfFormat::NTriples,
        RdfOntologySyntax::Turtle => OxRdfFormat::Turtle,
        RdfOntologySyntax::RdfXml => OxRdfFormat::RdfXml,
    }
}

fn read_input_bytes(input: &OntologyInput) -> Result<Vec<u8>> {
    let file =
        File::open(&input.path).context(format!("failed to open {}", input.path.display()))?;
    let reader = BufReader::with_capacity(256 * 1024, file);

    let mut reader: Box<dyn Read> = match input.compression {
        Compression::None => Box::new(reader),
        Compression::Gzip => Box::new(flate2::read::MultiGzDecoder::new(reader)),
        Compression::Bzip2 => Box::new(bzip2::read::MultiBzDecoder::new(reader)),
        Compression::Xz => Box::new(xz2::read::XzDecoder::new_multi_decoder(reader)),
    };

    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .context(format!("failed to read {}", input.path.display()))?;
    Ok(bytes)
}

fn absorb_parsed_ontology(
    parsed: ParsedOntology,
    dictionary: &mut Dictionary,
    schema: &mut RawSchema,
    ignore_annotation_axioms: bool,
) {
    absorb_ontology(
        &parsed.ontology,
        dictionary,
        schema,
        ignore_annotation_axioms,
    );

    if let Some(incomplete) = parsed.incomplete {
        record_incomplete_parse(&parsed.source, &incomplete, schema);
    }
}

fn record_incomplete_parse(
    source: &str,
    incomplete: &rdf::reader::IncompleteParse<RcStr>,
    schema: &mut RawSchema,
) {
    if incomplete.is_complete() {
        return;
    }

    let mut residue = Vec::new();
    push_incomplete_residue(&mut residue, "simple triples", incomplete.simple.len());
    push_incomplete_residue(&mut residue, "blank-node triples", incomplete.bnode.len());
    push_incomplete_residue(&mut residue, "RDF lists", incomplete.bnode_seq.len());
    push_incomplete_residue(
        &mut residue,
        "anonymous class expressions",
        incomplete.class_expression.len(),
    );
    push_incomplete_residue(
        &mut residue,
        "object property expressions",
        incomplete.object_property_expression.len(),
    );
    push_incomplete_residue(&mut residue, "data ranges", incomplete.data_range.len());
    push_incomplete_residue(&mut residue, "rule atoms", incomplete.atom.len());
    push_incomplete_residue(
        &mut residue,
        "dangling annotations",
        incomplete.ann_map.len(),
    );

    let detail = residue.join(", ");
    schema.unsupported.insert(format!(
        "RDF ontology {source} left unlowered horned-owl residue ({detail}); ignored"
    ));
}

fn push_incomplete_residue(residue: &mut Vec<String>, label: &str, count: usize) {
    if count > 0 {
        residue.push(format!("{label}={count}"));
    }
}

fn absorb_ontology(
    ontology: &SetOntology<RcStr>,
    dictionary: &mut Dictionary,
    schema: &mut RawSchema,
    ignore_annotation_axioms: bool,
) {
    for annotated in ontology {
        match &annotated.component {
            Component::OntologyID(_) | Component::DocIRI(_) | Component::OntologyAnnotation(_) => {}
            Component::DeclareClass(axiom) => {
                schema
                    .classes
                    .insert(dictionary.encode_iri(axiom.0.as_ref()));
            }
            Component::DeclareAnnotationProperty(axiom) => {
                if !ignore_annotation_axioms {
                    schema
                        .properties
                        .insert(dictionary.encode_iri(axiom.0.as_ref()));
                }
            }
            Component::DeclareObjectProperty(axiom) => {
                schema
                    .properties
                    .insert(dictionary.encode_iri(axiom.0.as_ref()));
            }
            Component::DeclareDataProperty(axiom) => {
                schema
                    .properties
                    .insert(dictionary.encode_iri(axiom.0.as_ref()));
            }
            Component::SubClassOf(axiom) => match (
                encode_named_class(&axiom.sub, dictionary),
                encode_named_class(&axiom.sup, dictionary),
            ) {
                (Ok(subclass), Ok(superclass)) => {
                    schema.subclasses.insert((subclass, superclass));
                }
                _ => {
                    schema.unsupported.insert(
                        "anonymous subclass axioms are deferred beyond Phase 1".to_string(),
                    );
                }
            },
            Component::SubObjectPropertyOf(axiom) => match (
                encode_named_subobject_property(&axiom.sub, dictionary),
                encode_named_object_property(&axiom.sup, dictionary),
            ) {
                (Ok(subproperty), Ok(superproperty)) => {
                    schema.subproperties.insert((subproperty, superproperty));
                }
                _ => {
                    schema.unsupported.insert(
                        "complex object property axioms are deferred beyond Phase 1".to_string(),
                    );
                }
            },
            Component::SubDataPropertyOf(axiom) => {
                let subproperty = dictionary.encode_iri(axiom.sub.as_ref());
                let superproperty = dictionary.encode_iri(axiom.sup.as_ref());
                schema.subproperties.insert((subproperty, superproperty));
            }
            Component::SubAnnotationPropertyOf(axiom) => {
                if !ignore_annotation_axioms {
                    let subproperty = dictionary.encode_iri(axiom.sub.as_ref());
                    let superproperty = dictionary.encode_iri(axiom.sup.as_ref());
                    schema.subproperties.insert((subproperty, superproperty));
                }
            }
            Component::ObjectPropertyDomain(axiom) => match (
                encode_named_object_property(&axiom.ope, dictionary),
                encode_named_class(&axiom.ce, dictionary),
            ) {
                (Ok(property), Ok(class)) => {
                    schema.domains.insert((property, class));
                }
                _ => {
                    schema.unsupported.insert(
                        "anonymous object property domain axioms are deferred beyond Phase 1"
                            .to_string(),
                    );
                }
            },
            Component::ObjectPropertyRange(axiom) => match (
                encode_named_object_property(&axiom.ope, dictionary),
                encode_named_class(&axiom.ce, dictionary),
            ) {
                (Ok(property), Ok(class)) => {
                    schema.ranges.insert((property, class));
                }
                _ => {
                    schema.unsupported.insert(
                        "anonymous object property range axioms are deferred beyond Phase 1"
                            .to_string(),
                    );
                }
            },
            Component::DataPropertyDomain(axiom) => match encode_named_class(&axiom.ce, dictionary)
            {
                Ok(class) => {
                    let property = dictionary.encode_iri(axiom.dp.as_ref());
                    schema.domains.insert((property, class));
                }
                Err(_) => {
                    schema.unsupported.insert(
                        "anonymous data property domain axioms are deferred beyond Phase 1"
                            .to_string(),
                    );
                }
            },
            Component::AnnotationPropertyDomain(axiom) => {
                if !ignore_annotation_axioms {
                    let property = dictionary.encode_iri(axiom.ap.as_ref());
                    let class = dictionary.encode_iri(axiom.iri.as_ref());
                    schema.domains.insert((property, class));
                }
            }
            Component::AnnotationPropertyRange(axiom) => {
                if !ignore_annotation_axioms {
                    let property = dictionary.encode_iri(axiom.ap.as_ref());
                    let range = dictionary.encode_iri(axiom.iri.as_ref());
                    schema.ranges.insert((property, range));
                }
            }
            Component::Import(_) => {
                schema
                    .unsupported
                    .insert("owl:imports are not implemented in Phase 1".to_string());
            }
            Component::DataPropertyRange(_) => {
                schema
                    .unsupported
                    .insert("DataPropertyRange is not implemented in Phase 1".to_string());
            }
            component => {
                schema.unsupported.insert(format!(
                    "{} is not implemented in Phase 1",
                    component_kind_name(component)
                ));
            }
        }
    }
}

fn serialize_triples_to_ntriples(triples: &[Triple]) -> Vec<u8> {
    let mut output = Vec::new();
    for triple in triples {
        let mut line = String::new();
        let _ = writeln!(
            line,
            "{} <{}> {} .",
            triple.subject.to_ntriples(),
            triple.predicate,
            triple.object.to_ntriples()
        );
        output.extend_from_slice(line.as_bytes());
    }
    output
}

fn encode_named_class(
    expression: &ClassExpression<RcStr>,
    dictionary: &mut Dictionary,
) -> std::result::Result<TermId, ()> {
    match expression {
        ClassExpression::Class(class) => Ok(dictionary.encode_iri(class.as_ref())),
        _ => Err(()),
    }
}

fn encode_named_object_property(
    expression: &ObjectPropertyExpression<RcStr>,
    dictionary: &mut Dictionary,
) -> std::result::Result<TermId, ()> {
    match expression.as_property() {
        Some(property) => Ok(dictionary.encode_iri(property.as_ref())),
        None => Err(()),
    }
}

fn encode_named_subobject_property(
    expression: &SubObjectPropertyExpression<RcStr>,
    dictionary: &mut Dictionary,
) -> std::result::Result<TermId, ()> {
    match expression {
        SubObjectPropertyExpression::ObjectPropertyExpression(expression) => {
            encode_named_object_property(expression, dictionary)
        }
        SubObjectPropertyExpression::ObjectPropertyChain(_) => Err(()),
    }
}

fn component_kind_name(component: &Component<RcStr>) -> String {
    format!("{:?}", component.kind()).replace("ComponentKind::", "")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{
        OntologySyntax, RdfOntologySyntax, classify_input, detect_compression, detect_syntax,
    };

    #[test]
    fn detects_supported_ontology_syntaxes() {
        assert_eq!(
            detect_syntax(Path::new("ontology.ofn")),
            Some(OntologySyntax::Functional)
        );
        assert_eq!(
            detect_syntax(Path::new("ontology.owx")),
            Some(OntologySyntax::OwlXml)
        );
        assert_eq!(
            detect_syntax(Path::new("ontology.nt")),
            Some(OntologySyntax::Rdf(RdfOntologySyntax::NTriples))
        );
        assert_eq!(
            detect_syntax(Path::new("ontology.ttl")),
            Some(OntologySyntax::Rdf(RdfOntologySyntax::Turtle))
        );
        assert_eq!(
            detect_syntax(Path::new("ontology.rdf")),
            Some(OntologySyntax::Rdf(RdfOntologySyntax::RdfXml))
        );
        assert_eq!(
            detect_syntax(Path::new("ontology.owl")),
            Some(OntologySyntax::Rdf(RdfOntologySyntax::RdfXml))
        );
        assert_eq!(detect_syntax(Path::new("ontology.xml")), None);
        assert_eq!(detect_syntax(Path::new("ontology.owlxml")), None);
        assert_eq!(detect_syntax(Path::new("ontology.fss")), None);
        assert_eq!(detect_syntax(Path::new("ontology.txt")), None);
    }

    #[test]
    fn preserves_syntax_under_compression_suffixes() {
        let (compression, base) = detect_compression(Path::new("ontology.ofn.gz"));
        assert_eq!(compression, super::Compression::Gzip);
        assert_eq!(base, Path::new("ontology.ofn"));

        let input = classify_input(Path::new("ontology.ttl.xz")).expect("ttl should classify");
        assert_eq!(input.syntax, OntologySyntax::Rdf(RdfOntologySyntax::Turtle));
    }

    #[test]
    fn discovers_supported_ontology_files_in_directories() {
        let temp_dir = tempfile::TempDir::new().expect("should create temp dir");
        let nested = temp_dir.path().join("nested");

        fs::create_dir_all(&nested).expect("nested directory should exist");
        fs::write(temp_dir.path().join("schema.ofn"), "").expect("ofn fixture should be written");
        fs::write(temp_dir.path().join("schema.ttl"), "").expect("ttl fixture should be written");
        fs::write(nested.join("schema.owx"), "").expect("owx fixture should be written");
        fs::write(nested.join("ignored.xml"), "").expect("xml fixture should be written");

        let inputs =
            super::discover_ontology_inputs(temp_dir.path()).expect("ontology inputs should load");
        let discovered = inputs
            .into_iter()
            .map(|input| input.path)
            .collect::<Vec<_>>();

        assert_eq!(
            discovered,
            vec![
                temp_dir.path().join("nested/schema.owx"),
                temp_dir.path().join("schema.ofn"),
                temp_dir.path().join("schema.ttl"),
            ]
        );
    }
}
