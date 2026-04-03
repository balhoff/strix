use std::error::Error as StdError;
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::Path;
use std::time::Duration;

use oxrdf::{NamedOrBlankNode, Term as OxTerm};
use oxrdfio::{
    JsonLdProfile, JsonLdProfileSet, LoadedDocument, RdfFormat as OxRdfFormat, RdfParser,
};
use url::Url;

use crate::error::{Result, ResultExt};
use crate::rdf::{Literal, Term, Triple};

use super::input::{Compression, RdfFormat, RdfInput, discover_inputs};

pub fn visit_path<F>(path: &Path, mut visitor: F) -> Result<()>
where
    F: FnMut(Triple) -> Result<()>,
{
    let inputs = discover_inputs(path)?;
    let namespace_blank_nodes = inputs.len() > 1;

    for (file_index, input) in inputs.iter().enumerate() {
        visit_file(input, file_index, namespace_blank_nodes, &mut visitor)?;
    }

    Ok(())
}

fn visit_file<F>(
    input: &RdfInput,
    file_index: usize,
    namespace_blank_nodes: bool,
    visitor: &mut F,
) -> Result<()>
where
    F: FnMut(Triple) -> Result<()>,
{
    let reader = open_input(input)?;
    let parser = build_parser(input)?;
    let parser = configure_reader_parser(input, parser.for_reader(reader));

    for quad in parser {
        let quad = quad.context(format!("failed to parse {}", input.path.display()))?;
        visitor(Triple {
            subject: convert_subject(quad.subject, file_index, namespace_blank_nodes),
            predicate: quad.predicate.into_string(),
            object: convert_object(quad.object, file_index, namespace_blank_nodes),
        })?;
    }

    Ok(())
}

fn build_parser(input: &RdfInput) -> Result<RdfParser> {
    let base_iri = file_iri(&input.path)?;
    RdfParser::from_format(to_oxrdf_format(input.format))
        .with_base_iri(base_iri)
        .context(format!(
            "failed to derive parser base IRI for {}",
            input.path.display()
        ))
}

fn configure_reader_parser<R: Read>(
    input: &RdfInput,
    parser: oxrdfio::ReaderQuadParser<R>,
) -> oxrdfio::ReaderQuadParser<R> {
    if input.format == RdfFormat::JsonLd {
        parser.with_document_loader(load_jsonld_document)
    } else {
        parser
    }
}

fn file_iri(path: &Path) -> Result<String> {
    let canonical_path = path
        .canonicalize()
        .context(format!("failed to canonicalize {}", path.display()))?;
    Url::from_file_path(&canonical_path)
        .map(|url| url.to_string())
        .map_err(|()| {
            crate::error::AppError::new(format!(
                "failed to derive a file IRI from {}",
                canonical_path.display()
            ))
        })
}

fn load_jsonld_document(url: &str) -> LoaderResult<LoadedDocument> {
    let parsed_url = Url::parse(url)
        .map_err(|error| io::Error::other(format!("invalid JSON-LD context URL {url}: {error}")))?;
    match parsed_url.scheme() {
        "file" => load_file_jsonld_document(parsed_url),
        "http" | "https" => load_http_jsonld_document(url),
        scheme => Err(io::Error::other(format!(
            "unsupported JSON-LD context scheme {scheme} for {url}"
        ))
        .into()),
    }
}

fn load_file_jsonld_document(url: Url) -> LoaderResult<LoadedDocument> {
    let path = url.to_file_path().map_err(|()| {
        io::Error::other(format!(
            "failed to convert JSON-LD context URL {} to a filesystem path",
            url
        ))
    })?;
    let content = fs::read(&path).map_err(|error| {
        io::Error::other(format!(
            "failed to read JSON-LD context {} from {}: {error}",
            url,
            path.display()
        ))
    })?;
    Ok(LoadedDocument {
        url: url.to_string(),
        content,
        format: jsonld_context_format(),
    })
}

fn load_http_jsonld_document(url: &str) -> LoaderResult<LoadedDocument> {
    let agent = ureq::builder()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build();
    let response = agent
        .get(url)
        .set(
            "Accept",
            "application/ld+json, application/json;q=0.9, */*;q=0.1",
        )
        .call()
        .map_err(|error| {
            io::Error::other(format!("failed to load JSON-LD context {url}: {error}"))
        })?;
    let final_url = response.get_url().to_string();
    let mut content = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut content)
        .map_err(|error| {
            io::Error::other(format!("failed to read JSON-LD context {url}: {error}"))
        })?;
    Ok(LoadedDocument {
        url: final_url,
        content,
        format: jsonld_context_format(),
    })
}

fn jsonld_context_format() -> OxRdfFormat {
    OxRdfFormat::JsonLd {
        profile: JsonLdProfile::Context.into(),
    }
}

type LoaderResult<T> = std::result::Result<T, Box<dyn StdError + Send + Sync>>;

fn open_input(input: &RdfInput) -> Result<Box<dyn Read>> {
    let file =
        File::open(&input.path).context(format!("failed to open {}", input.path.display()))?;
    let reader = BufReader::with_capacity(256 * 1024, file);

    let reader: Box<dyn Read> = match input.compression {
        Compression::None => Box::new(reader),
        Compression::Gzip => Box::new(flate2::read::MultiGzDecoder::new(reader)),
        Compression::Bzip2 => Box::new(bzip2::read::MultiBzDecoder::new(reader)),
        Compression::Xz => Box::new(xz2::read::XzDecoder::new_multi_decoder(reader)),
    };

    Ok(reader)
}

fn to_oxrdf_format(format: RdfFormat) -> OxRdfFormat {
    match format {
        RdfFormat::NTriples => OxRdfFormat::NTriples,
        RdfFormat::NQuads => OxRdfFormat::NQuads,
        RdfFormat::Turtle => OxRdfFormat::Turtle,
        RdfFormat::TriG => OxRdfFormat::TriG,
        RdfFormat::RdfXml => OxRdfFormat::RdfXml,
        RdfFormat::JsonLd => OxRdfFormat::JsonLd {
            profile: JsonLdProfileSet::empty(),
        },
        RdfFormat::N3 => OxRdfFormat::N3,
    }
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{self, Read, Write};
    use std::net::TcpListener;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use url::Url;

    use super::visit_path;
    use crate::rdf::{Term, Triple};

    #[test]
    fn resolves_relative_iris_against_the_source_document() {
        let temp_dir = TestDir::new("relative-iris");
        let data = temp_dir.path.join("data.ttl");
        let canonical_dir = temp_dir
            .path
            .canonicalize()
            .expect("test dir should canonicalize");
        write(&data, "<s> <p> <o> .\n");

        let triples = collect_triples(&data);

        assert_eq!(
            triples,
            vec![Triple {
                subject: Term::Iri(file_url(canonical_dir.join("s"))),
                predicate: file_url(canonical_dir.join("p")),
                object: Term::Iri(file_url(canonical_dir.join("o"))),
            }]
        );
    }

    #[test]
    fn loads_remote_jsonld_contexts() {
        let temp_dir = TestDir::new("remote-jsonld-context");
        let data = temp_dir.path.join("data.jsonld");
        let context_body = r#"{"@context":{"p":{"@id":"http://example.com/p","@type":"@id"}}}"#;
        let server = TestServer::new("/context.jsonld", context_body);
        write(
            &data,
            &format!(
                "{{\n  \"@context\": \"{}/context.jsonld\",\n  \"@id\": \"http://example.com/s\",\n  \"p\": \"http://example.com/o\"\n}}\n",
                server.base_url()
            ),
        );

        let triples = collect_triples(&data);
        server.join();

        assert_eq!(
            triples,
            vec![Triple {
                subject: Term::Iri("http://example.com/s".to_string()),
                predicate: "http://example.com/p".to_string(),
                object: Term::Iri("http://example.com/o".to_string()),
            }]
        );
    }

    fn collect_triples(path: &Path) -> Vec<Triple> {
        let mut triples = Vec::new();
        visit_path(path, |triple| {
            triples.push(triple);
            Ok(())
        })
        .expect("RDF input should parse successfully");
        triples
    }

    fn file_url(path: PathBuf) -> String {
        Url::from_file_path(path)
            .expect("test path should produce a file URL")
            .to_string()
    }

    fn write(path: &Path, content: &str) {
        fs::write(path, content).expect("test fixture should be written");
    }

    struct TestServer {
        base_url: String,
        handle: thread::JoinHandle<()>,
    }

    impl TestServer {
        fn new(path: &'static str, body: &'static str) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0")
                .expect("test server should bind an ephemeral port");
            listener
                .set_nonblocking(true)
                .expect("test server should become non-blocking");
            let address = listener
                .local_addr()
                .expect("test server should expose its bound address");
            let base_url = format!("http://{address}");
            let handle = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(5);
                loop {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let mut buffer = [0_u8; 2048];
                            let size = stream
                                .read(&mut buffer)
                                .expect("test server should read a request");
                            let request = String::from_utf8_lossy(&buffer[..size]);
                            assert!(
                                request.starts_with(&format!("GET {path} ")),
                                "unexpected request line: {request}"
                            );
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/ld+json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            stream
                                .write_all(response.as_bytes())
                                .expect("test server should write a response");
                            return;
                        }
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            assert!(
                                Instant::now() < deadline,
                                "test server did not receive a request in time"
                            );
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(error) => panic!("test server accept failed: {error}"),
                    }
                }
            });
            Self { base_url, handle }
        }

        fn base_url(&self) -> &str {
            &self.base_url
        }

        fn join(self) {
            self.handle
                .join()
                .expect("test server thread should complete");
        }
    }

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(label: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("strix-{label}-{unique}"));
            fs::create_dir_all(&path).expect("test temp dir should be created");
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
