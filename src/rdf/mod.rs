mod input;
mod parser;

pub use parser::visit_path;

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
