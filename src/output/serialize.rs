use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::anyhow;

use crate::cli::EmitMode;
use crate::dict::{Dictionary, TermId};
use crate::error::Result;
use crate::owl::RDF_TYPE_IRI;
use crate::rdf::Term;
use crate::store::FactStore;

/// Write inferred or closure triples to an N-Triples file.
///
/// Triples involving proxy terms (anonymous CE / ObjectInverseOf scaffolding)
/// are filtered out so only user-visible terms appear in the output.
pub fn write_ntriples(
    path: &Path,
    emit: EmitMode,
    dictionary: &Dictionary,
    store: &mut FactStore,
    proxy_terms: &BTreeSet<TermId>,
) -> Result<usize> {
    let writer = File::create(path)?;
    let mut writer = BufWriter::with_capacity(256 * 1024, writer);
    let mut written = 0usize;

    let is_proxy_type =
        |inst: TermId, cls: TermId| proxy_terms.contains(&inst) || proxy_terms.contains(&cls);
    let is_proxy_prop = |s: TermId, p: TermId, o: TermId| {
        proxy_terms.contains(&s) || proxy_terms.contains(&p) || proxy_terms.contains(&o)
    };

    match emit {
        EmitMode::Inferred => {
            for result in store.derived_types_iter()? {
                let (instance, class) = result?;
                if is_proxy_type(instance, class) {
                    continue;
                }
                if let Some(line) = format_type_triple(dictionary, instance, class)? {
                    writeln!(writer, "{line}")?;
                    written += 1;
                }
            }
            for result in store.derived_properties_iter()? {
                let (subject, predicate, object) = result?;
                if is_proxy_prop(subject, predicate, object) {
                    continue;
                }
                if let Some(line) = format_property_triple(dictionary, subject, predicate, object)? {
                    writeln!(writer, "{line}")?;
                    written += 1;
                }
            }
        }
        EmitMode::Closure => {
            for result in store.known_types_iter()? {
                let (instance, class) = result?;
                if is_proxy_type(instance, class) {
                    continue;
                }
                if let Some(line) = format_type_triple(dictionary, instance, class)? {
                    writeln!(writer, "{line}")?;
                    written += 1;
                }
            }
            for result in store.known_properties_iter()? {
                let (subject, predicate, object) = result?;
                if is_proxy_prop(subject, predicate, object) {
                    continue;
                }
                if let Some(line) = format_property_triple(dictionary, subject, predicate, object)? {
                    writeln!(writer, "{line}")?;
                    written += 1;
                }
            }
        }
    }

    writer.flush()?;
    Ok(written)
}

/// Format a type triple, returning `None` if the subject is a literal
/// (literal-as-subject type assertions are internal bookkeeping for data
/// range restrictions and must not appear in output).
fn format_type_triple(dictionary: &Dictionary, instance: u64, class: u64) -> Result<Option<String>> {
    let subject = decode_term(dictionary, instance, "subject")?;
    if matches!(subject, Term::Literal(_)) {
        return Ok(None);
    }
    let subject = subject.to_ntriples();
    let object = decode_term(dictionary, class, "class")?.to_ntriples();
    Ok(Some(format!("{subject} <{RDF_TYPE_IRI}> {object} .")))
}

/// Format a property triple, returning `None` if the subject is a literal.
fn format_property_triple(
    dictionary: &Dictionary,
    subject: u64,
    predicate: u64,
    object: u64,
) -> Result<Option<String>> {
    let subject = decode_term(dictionary, subject, "subject")?;
    if matches!(subject, Term::Literal(_)) {
        return Ok(None);
    }
    let subject = subject.to_ntriples();
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
    Ok(Some(format!("{subject} {predicate} {object} .")))
}

fn decode_term<'a>(dictionary: &'a Dictionary, identifier: u64, role: &str) -> Result<&'a Term> {
    dictionary
        .decode(identifier)
        .ok_or_else(|| anyhow!("missing dictionary entry for {role} id {identifier}"))
}
