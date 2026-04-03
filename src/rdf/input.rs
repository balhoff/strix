use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::error::{AppError, Result, ResultExt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RdfFormat {
    NTriples,
    NQuads,
    Turtle,
    TriG,
    RdfXml,
    JsonLd,
    N3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Compression {
    None,
    Gzip,
    Bzip2,
    Xz,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RdfInput {
    pub path: PathBuf,
    pub format: RdfFormat,
    pub compression: Compression,
}

pub fn discover_inputs(path: &Path) -> Result<Vec<RdfInput>> {
    if !path.exists() {
        return Err(AppError::new(format!(
            "path does not exist: {}",
            path.display()
        )));
    }

    let mut inputs = Vec::new();
    if path.is_file() {
        let input = classify_file(path).ok_or_else(|| {
            AppError::new(format!("unsupported RDF input format: {}", path.display()))
        })?;
        inputs.push(input);
    } else if path.is_dir() {
        for entry in WalkDir::new(path).follow_links(true) {
            let entry = entry.context(format!("failed to walk {}", path.display()))?;
            if entry.file_type().is_file()
                && let Some(input) = classify_file(entry.path())
            {
                inputs.push(input);
            }
        }
    } else {
        return Err(AppError::new(format!(
            "path is neither a file nor a directory: {}",
            path.display()
        )));
    }

    inputs.sort_by(|left, right| left.path.cmp(&right.path));
    if inputs.is_empty() {
        return Err(AppError::new(format!(
            "no supported RDF files found under {}",
            path.display()
        )));
    }

    Ok(inputs)
}

fn classify_file(path: &Path) -> Option<RdfInput> {
    let (compression, base_path) = detect_compression(path);
    let format = detect_format(&base_path)?;
    Some(RdfInput {
        path: path.to_path_buf(),
        format,
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

fn detect_format(path: &Path) -> Option<RdfFormat> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();

    if extension.eq_ignore_ascii_case("nt") || extension.eq_ignore_ascii_case("ntriples") {
        return Some(RdfFormat::NTriples);
    }
    if extension.eq_ignore_ascii_case("nq") || extension.eq_ignore_ascii_case("nquads") {
        return Some(RdfFormat::NQuads);
    }
    if extension.eq_ignore_ascii_case("ttl") || extension.eq_ignore_ascii_case("turtle") {
        return Some(RdfFormat::Turtle);
    }
    if extension.eq_ignore_ascii_case("trig") {
        return Some(RdfFormat::TriG);
    }
    if extension.eq_ignore_ascii_case("rdf")
        || extension.eq_ignore_ascii_case("xml")
        || extension.eq_ignore_ascii_case("owl")
    {
        return Some(RdfFormat::RdfXml);
    }
    if extension.eq_ignore_ascii_case("jsonld") || extension.eq_ignore_ascii_case("json") {
        return Some(RdfFormat::JsonLd);
    }
    if extension.eq_ignore_ascii_case("n3") {
        return Some(RdfFormat::N3);
    }
    None
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        Compression, RdfFormat, classify_file, detect_compression, detect_format, discover_inputs,
    };

    #[test]
    fn detects_all_supported_formats() {
        assert_eq!(
            detect_format(Path::new("data.nt")),
            Some(RdfFormat::NTriples)
        );
        assert_eq!(
            detect_format(Path::new("data.ntriples")),
            Some(RdfFormat::NTriples)
        );
        assert_eq!(detect_format(Path::new("data.nq")), Some(RdfFormat::NQuads));
        assert_eq!(
            detect_format(Path::new("data.nquads")),
            Some(RdfFormat::NQuads)
        );
        assert_eq!(
            detect_format(Path::new("data.ttl")),
            Some(RdfFormat::Turtle)
        );
        assert_eq!(
            detect_format(Path::new("data.turtle")),
            Some(RdfFormat::Turtle)
        );
        assert_eq!(detect_format(Path::new("data.trig")), Some(RdfFormat::TriG));
        assert_eq!(
            detect_format(Path::new("data.rdf")),
            Some(RdfFormat::RdfXml)
        );
        assert_eq!(
            detect_format(Path::new("data.xml")),
            Some(RdfFormat::RdfXml)
        );
        assert_eq!(
            detect_format(Path::new("data.owl")),
            Some(RdfFormat::RdfXml)
        );
        assert_eq!(
            detect_format(Path::new("data.jsonld")),
            Some(RdfFormat::JsonLd)
        );
        assert_eq!(
            detect_format(Path::new("data.json")),
            Some(RdfFormat::JsonLd)
        );
        assert_eq!(detect_format(Path::new("data.n3")), Some(RdfFormat::N3));
        assert_eq!(detect_format(Path::new("data.txt")), None);
        assert_eq!(detect_format(Path::new("data")), None);
    }

    #[test]
    fn detects_supported_compression_layers() {
        let (compression, base) = detect_compression(Path::new("data.nt.gz"));
        assert_eq!(compression, Compression::Gzip);
        assert_eq!(base, Path::new("data.nt"));

        let (compression, base) = detect_compression(Path::new("data.ttl.bz2"));
        assert_eq!(compression, Compression::Bzip2);
        assert_eq!(base, Path::new("data.ttl"));

        let (compression, base) = detect_compression(Path::new("data.nq.xz"));
        assert_eq!(compression, Compression::Xz);
        assert_eq!(base, Path::new("data.nq"));
    }

    #[test]
    fn classifies_compressed_inputs() {
        let input = classify_file(Path::new("data.nt.gz")).expect("gzip nt should be recognized");
        assert_eq!(input.format, RdfFormat::NTriples);
        assert_eq!(input.compression, Compression::Gzip);

        let input = classify_file(Path::new("data.trig.xz")).expect("xz trig should be recognized");
        assert_eq!(input.format, RdfFormat::TriG);
        assert_eq!(input.compression, Compression::Xz);
    }

    #[test]
    fn recursively_discovers_supported_files_only() {
        let temp_dir = TestDir::new("rdf-discovery");
        let nested = temp_dir.path.join("nested");

        fs::create_dir_all(&nested).expect("nested directory should exist");
        fs::write(temp_dir.path.join("root.nt"), "").expect("root nt should be written");
        fs::write(nested.join("child.ttl.gz"), "").expect("child ttl should be written");
        fs::write(nested.join("ignored.txt"), "").expect("ignored file should be written");

        let inputs = discover_inputs(&temp_dir.path).expect("directory should be discovered");
        let discovered = inputs
            .into_iter()
            .map(|input| input.path)
            .collect::<Vec<_>>();

        assert_eq!(
            discovered,
            vec![
                temp_dir.path.join("nested/child.ttl.gz"),
                temp_dir.path.join("root.nt")
            ]
        );
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
