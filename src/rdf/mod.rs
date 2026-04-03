use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use oxrdf::{GraphName, NamedOrBlankNode, Term as OxTerm};
use oxrdfio::{RdfFormat, RdfParser};

use crate::error::{AppError, Result, ResultExt};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Term {
    Iri(String),
    BlankNode(String),
    Literal(Literal),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Literal {
    pub lexical_form: String,
    pub language: Option<String>,
    pub datatype: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Triple {
    pub subject: Term,
    pub predicate: String,
    pub object: Term,
}

impl Term {
    pub fn as_iri(&self) -> Option<&str> {
        match self {
            Self::Iri(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn to_ntriples(&self) -> String {
        match self {
            Self::Iri(value) => format!("<{value}>"),
            Self::BlankNode(value) => format!("_:{value}"),
            Self::Literal(literal) => {
                let mut output = format!("\"{}\"", escape_literal(&literal.lexical_form));
                if let Some(language) = &literal.language {
                    output.push('@');
                    output.push_str(language);
                }
                if let Some(datatype) = &literal.datatype {
                    output.push_str("^^<");
                    output.push_str(datatype);
                    output.push('>');
                }
                output
            }
        }
    }
}

pub fn visit_path<F>(path: &Path, mut visitor: F) -> Result<()>
where
    F: FnMut(Triple) -> Result<()>,
{
    let mut files = Vec::new();
    collect_files(path, &mut files)?;
    files.sort();

    if files.is_empty() {
        return Err(AppError::new(format!(
            "no supported N-Triples files found under {}",
            path.display()
        )));
    }

    let namespace_blank_nodes = files.len() > 1;
    for (file_index, file) in files.iter().enumerate() {
        visit_file(file, file_index, namespace_blank_nodes, &mut visitor)?;
    }

    Ok(())
}

fn visit_file<F>(
    path: &Path,
    file_index: usize,
    namespace_blank_nodes: bool,
    visitor: &mut F,
) -> Result<()>
where
    F: FnMut(Triple) -> Result<()>,
{
    let reader =
        BufReader::new(File::open(path).context(format!("failed to open {}", path.display()))?);
    let parser = RdfParser::from_format(RdfFormat::NTriples).without_named_graphs();

    for quad in parser.for_reader(reader) {
        let quad = quad.context(format!("failed to parse {}", path.display()))?;
        if !matches!(quad.graph_name, GraphName::DefaultGraph) {
            return Err(AppError::new(format!(
                "unexpected named graph in {}",
                path.display()
            )));
        }

        visitor(Triple {
            subject: convert_subject(quad.subject, file_index, namespace_blank_nodes),
            predicate: quad.predicate.into_string(),
            object: convert_object(quad.object, file_index, namespace_blank_nodes),
        })?;
    }

    Ok(())
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }

    if !path.exists() {
        return Err(AppError::new(format!(
            "path does not exist: {}",
            path.display()
        )));
    }

    for entry in
        std::fs::read_dir(path).context(format!("failed to read directory {}", path.display()))?
    {
        let entry = entry.context(format!("failed to read entry under {}", path.display()))?;
        let child = entry.path();
        if child.is_dir() {
            collect_files(&child, files)?;
        } else if is_ntriples_file(&child) {
            files.push(child);
        }
    }

    Ok(())
}

fn is_ntriples_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            let extension = extension.to_ascii_lowercase();
            extension == "nt" || extension == "ntriples"
        })
        .unwrap_or(false)
}

fn convert_subject(
    subject: NamedOrBlankNode,
    file_index: usize,
    namespace_blank_nodes: bool,
) -> Term {
    match subject {
        NamedOrBlankNode::NamedNode(node) => Term::Iri(node.into_string()),
        NamedOrBlankNode::BlankNode(node) => Term::BlankNode(blank_node_label(
            node.as_str(),
            file_index,
            namespace_blank_nodes,
        )),
    }
}

fn convert_object(object: OxTerm, file_index: usize, namespace_blank_nodes: bool) -> Term {
    match object {
        OxTerm::NamedNode(node) => Term::Iri(node.into_string()),
        OxTerm::BlankNode(node) => Term::BlankNode(blank_node_label(
            node.as_str(),
            file_index,
            namespace_blank_nodes,
        )),
        OxTerm::Literal(literal) => {
            let (lexical_form, datatype, language) = literal.destruct();
            Term::Literal(Literal {
                lexical_form,
                language,
                datatype: datatype.map(|datatype| datatype.into_string()),
            })
        }
    }
}

fn blank_node_label(label: &str, file_index: usize, namespace_blank_nodes: bool) -> String {
    if namespace_blank_nodes {
        format!("f{file_index}_{label}")
    } else {
        label.to_string()
    }
}

fn escape_literal(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000C}' => output.push_str("\\f"),
            character if character.is_control() => {
                let codepoint = character as u32;
                if codepoint <= 0xFFFF {
                    output.push_str(&format!("\\u{codepoint:04X}"));
                } else {
                    output.push_str(&format!("\\U{codepoint:08X}"));
                }
            }
            _ => output.push(character),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{Literal, Term};

    #[test]
    fn escapes_all_literal_control_characters() {
        let literal = Term::Literal(Literal {
            lexical_form: "\u{0008}\u{000C}\u{0001}".to_string(),
            language: None,
            datatype: None,
        });

        assert_eq!(literal.to_ntriples(), "\"\\b\\f\\u0001\"");
    }
}
