use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::dict::TermId;
use crate::error::Result;

/// An immutable sorted segment of tuples stored on disk.
#[derive(Debug)]
pub struct Segment {
    pub path: PathBuf,
    pub len: usize,
    pub arity: u8,
}

/// Write a sorted slice of binary tuples to a segment file.
pub fn write_binary_segment(path: &Path, tuples: &[(TermId, TermId)]) -> Result<Segment> {
    write_binary_segment_streaming(path, tuples.iter().copied().map(Ok))
}

/// Write a sorted slice of ternary tuples to a segment file.
pub fn write_ternary_segment(
    path: &Path,
    tuples: &[(TermId, TermId, TermId)],
) -> Result<Segment> {
    write_ternary_segment_streaming(path, tuples.iter().copied().map(Ok))
}

/// Write binary tuples from a streaming iterator to a segment file.
pub fn write_binary_segment_streaming(
    path: &Path,
    iter: impl Iterator<Item = std::io::Result<(TermId, TermId)>>,
) -> Result<Segment> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let mut len = 0usize;
    for result in iter {
        let (a, b) = result?;
        writer.write_all(&a.to_le_bytes())?;
        writer.write_all(&b.to_le_bytes())?;
        len += 1;
    }
    writer.flush()?;
    Ok(Segment {
        path: path.to_path_buf(),
        len,
        arity: 2,
    })
}

/// Write ternary tuples from a streaming iterator to a segment file.
pub fn write_ternary_segment_streaming(
    path: &Path,
    iter: impl Iterator<Item = std::io::Result<(TermId, TermId, TermId)>>,
) -> Result<Segment> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let mut len = 0usize;
    for result in iter {
        let (a, b, c) = result?;
        writer.write_all(&a.to_le_bytes())?;
        writer.write_all(&b.to_le_bytes())?;
        writer.write_all(&c.to_le_bytes())?;
        len += 1;
    }
    writer.flush()?;
    Ok(Segment {
        path: path.to_path_buf(),
        len,
        arity: 3,
    })
}

/// Streaming iterator over binary tuples from a segment file.
pub struct BinarySegmentIter {
    reader: BufReader<File>,
    buf: [u8; 16],
}

impl BinarySegmentIter {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        Ok(Self {
            reader: BufReader::new(file),
            buf: [0u8; 16],
        })
    }
}

impl Iterator for BinarySegmentIter {
    type Item = std::io::Result<(TermId, TermId)>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_exact(&mut self.buf) {
            Ok(()) => {
                let a = u64::from_le_bytes(self.buf[..8].try_into().unwrap());
                let b = u64::from_le_bytes(self.buf[8..16].try_into().unwrap());
                Some(Ok((a, b)))
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => None,
            Err(e) => Some(Err(e)),
        }
    }
}

/// Streaming iterator over ternary tuples from a segment file.
pub struct TernarySegmentIter {
    reader: BufReader<File>,
    buf: [u8; 24],
}

impl TernarySegmentIter {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        Ok(Self {
            reader: BufReader::new(file),
            buf: [0u8; 24],
        })
    }
}

impl Iterator for TernarySegmentIter {
    type Item = std::io::Result<(TermId, TermId, TermId)>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_exact(&mut self.buf) {
            Ok(()) => {
                let a = u64::from_le_bytes(self.buf[..8].try_into().unwrap());
                let b = u64::from_le_bytes(self.buf[8..16].try_into().unwrap());
                let c = u64::from_le_bytes(self.buf[16..24].try_into().unwrap());
                Some(Ok((a, b, c)))
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_segment_iter_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("iter.seg");
        let data = vec![(1u64, 2u64), (3, 4), (5, 6)];
        write_binary_segment(&path, &data).unwrap();
        let iter = BinarySegmentIter::open(&path).unwrap();
        let read: Vec<_> = iter.map(|r| r.unwrap()).collect();
        assert_eq!(data, read);
    }

    #[test]
    fn ternary_segment_iter_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("iter.seg");
        let data = vec![(1u64, 2u64, 3u64), (4, 5, 6)];
        write_ternary_segment(&path, &data).unwrap();
        let iter = TernarySegmentIter::open(&path).unwrap();
        let read: Vec<_> = iter.map(|r| r.unwrap()).collect();
        assert_eq!(data, read);
    }

    #[test]
    fn empty_segment_iter() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("empty-iter.seg");
        let data: Vec<(u64, u64)> = vec![];
        write_binary_segment(&path, &data).unwrap();
        let iter = BinarySegmentIter::open(&path).unwrap();
        let read: Vec<_> = iter.map(|r| r.unwrap()).collect();
        assert_eq!(data, read);
    }
}
