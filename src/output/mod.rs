pub mod report;
pub mod serialize;

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use crate::error::Result;

pub use serialize::write_ntriples;

pub fn write_run_report(path: &Path, report: &report::RunReport) -> Result<()> {
    let writer = BufWriter::new(File::create(path)?);
    serde_json::to_writer_pretty(writer, report)?;
    Ok(())
}
