use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::dict::TermId;
use crate::error::Result;

/// An immutable sorted segment of tuples stored on disk with zstd compression.
#[derive(Debug)]
pub struct Segment {
    pub path: PathBuf,
    pub len: usize,
    pub arity: u8,
}

const ZSTD_LEVEL: i32 = 1;

/// Write a sorted slice of binary tuples to a zstd-compressed segment file.
pub fn write_binary_segment(path: &Path, tuples: &[(TermId, TermId)]) -> Result<Segment> {
    let file = File::create(path)?;
    let mut writer = zstd::Encoder::new(BufWriter::new(file), ZSTD_LEVEL)?;
    for (a, b) in tuples {
        writer.write_all(&a.to_le_bytes())?;
        writer.write_all(&b.to_le_bytes())?;
    }
    writer.finish()?;
    Ok(Segment {
        path: path.to_path_buf(),
        len: tuples.len(),
        arity: 2,
    })
}

/// Write a sorted slice of ternary tuples to a zstd-compressed segment file.
pub fn write_ternary_segment(path: &Path, tuples: &[(TermId, TermId, TermId)]) -> Result<Segment> {
    let file = File::create(path)?;
    let mut writer = zstd::Encoder::new(BufWriter::new(file), ZSTD_LEVEL)?;
    for (a, b, c) in tuples {
        writer.write_all(&a.to_le_bytes())?;
        writer.write_all(&b.to_le_bytes())?;
        writer.write_all(&c.to_le_bytes())?;
    }
    writer.finish()?;
    Ok(Segment {
        path: path.to_path_buf(),
        len: tuples.len(),
        arity: 3,
    })
}

/// Read all binary tuples from a zstd-compressed segment file.
pub fn read_binary_segment(path: &Path) -> Result<Vec<(TermId, TermId)>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(zstd::Decoder::new(file)?);
    let mut tuples = Vec::new();
    let mut buf = [0u8; 16];
    loop {
        match reader.read_exact(&mut buf) {
            Ok(()) => {
                let a = u64::from_le_bytes(buf[..8].try_into().unwrap());
                let b = u64::from_le_bytes(buf[8..16].try_into().unwrap());
                tuples.push((a, b));
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(tuples)
}

/// Read all ternary tuples from a zstd-compressed segment file.
pub fn read_ternary_segment(path: &Path) -> Result<Vec<(TermId, TermId, TermId)>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(zstd::Decoder::new(file)?);
    let mut tuples = Vec::new();
    let mut buf = [0u8; 24];
    loop {
        match reader.read_exact(&mut buf) {
            Ok(()) => {
                let a = u64::from_le_bytes(buf[..8].try_into().unwrap());
                let b = u64::from_le_bytes(buf[8..16].try_into().unwrap());
                let c = u64::from_le_bytes(buf[16..24].try_into().unwrap());
                tuples.push((a, b, c));
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(tuples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_segment_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.seg");
        let data = vec![(1u64, 2u64), (3, 4), (5, 6)];
        write_binary_segment(&path, &data).unwrap();
        let read = read_binary_segment(&path).unwrap();
        assert_eq!(data, read);
    }

    #[test]
    fn ternary_segment_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.seg");
        let data = vec![(1u64, 2u64, 3u64), (4, 5, 6)];
        write_ternary_segment(&path, &data).unwrap();
        let read = read_ternary_segment(&path).unwrap();
        assert_eq!(data, read);
    }

    #[test]
    fn empty_segment_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("empty.seg");
        let data: Vec<(u64, u64)> = vec![];
        write_binary_segment(&path, &data).unwrap();
        let read = read_binary_segment(&path).unwrap();
        assert_eq!(data, read);
    }
}
