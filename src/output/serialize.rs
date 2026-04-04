use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::anyhow;

use crate::cli::EmitMode;
use crate::dict::Dictionary;
use crate::error::Result;
use crate::owl::RDF_TYPE_IRI;
use crate::rdf::Term;
use crate::store::FactStore;

/// Write inferred or closure triples to an N-Triples file.
pub fn write_ntriples(
    path: &Path,
    emit: EmitMode,
    dictionary: &Dictionary,
    store: &mut FactStore,
) -> Result<usize> {
    let writer = File::create(path)?;
    let mut writer = BufWriter::with_capacity(256 * 1024, writer);
    let mut written = 0usize;

    match emit {
        EmitMode::Inferred => {
            for result in store.derived_types_iter()? {
                let (instance, class) = result?;
                writeln!(
                    writer,
                    "{}",
                    format_type_triple(dictionary, instance, class)?
                )?;
                written += 1;
            }
            for result in store.derived_properties_iter()? {
                let (subject, predicate, object) = result?;
                writeln!(
                    writer,
                    "{}",
                    format_property_triple(dictionary, subject, predicate, object)?
                )?;
                written += 1;
            }
        }
        EmitMode::Closure => {
            for result in store.known_types_iter()? {
                let (instance, class) = result?;
                writeln!(
                    writer,
                    "{}",
                    format_type_triple(dictionary, instance, class)?
                )?;
                written += 1;
            }
            for result in store.known_properties_iter()? {
                let (subject, predicate, object) = result?;
                writeln!(
                    writer,
                    "{}",
                    format_property_triple(dictionary, subject, predicate, object)?
                )?;
                written += 1;
            }
        }
    }

    writer.flush()?;
    Ok(written)
}

fn format_type_triple(dictionary: &Dictionary, instance: u64, class: u64) -> Result<String> {
    let subject = decode_term(dictionary, instance, "subject")?.to_ntriples();
    let object = decode_term(dictionary, class, "class")?.to_ntriples();
    Ok(format!("{subject} <{RDF_TYPE_IRI}> {object} ."))
}

fn format_property_triple(
    dictionary: &Dictionary,
    subject: u64,
    predicate: u64,
    object: u64,
) -> Result<String> {
    let subject = decode_term(dictionary, subject, "subject")?.to_ntriples();
    let predicate = match decode_term(dictionary, predicate, "predicate")? {
        Term::Iri(iri) => format!("<{iri}>"),
        other => {
            return Err(anyhow!(
                "predicate id {predicate} decoded to non-IRI term {}",
                other.to_ntriples()
            ));
        }
    };
    let object = decode_term(dictionary, object, "object")?.to_ntriples();
    Ok(format!("{subject} {predicate} {object} ."))
}

fn decode_term<'a>(dictionary: &'a Dictionary, identifier: u64, role: &str) -> Result<&'a Term> {
    dictionary
        .decode(identifier)
        .ok_or_else(|| anyhow!("missing dictionary entry for {role} id {identifier}"))
}
