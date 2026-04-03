use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
#[command(
    name = "strix",
    about = "Rule-based RDF/OWL reasoner",
    long_about = "Runs RDF ingestion, schema compilation, RDFS materialization, and export. \
                  The Phase 1 CLI currently exposes the end-to-end `reason` subcommand.",
    arg_required_else_help = true,
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Increase logging verbosity (-v for debug, -vv for trace)
    #[arg(short, long, action = ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Emit per-stage timing and peak RSS
    #[arg(long, global = true)]
    pub benchmark: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
pub enum Commands {
    /// Full pipeline: ingest, compile, materialize, export
    Reason(ReasonArgs),
}

#[derive(Clone, Debug, Eq, PartialEq, Parser)]
pub struct ReasonArgs {
    /// Input RDF datasets (supported RDF files or directories)
    #[arg(
        value_name = "PATH",
        num_args = 1..,
        required = true
    )]
    pub data: Vec<PathBuf>,

    /// Output file for inferred triples
    #[arg(short, long, value_name = "PATH", allow_hyphen_values = true)]
    pub output: PathBuf,

    /// Separate ontology input (supported RDF file or directory)
    #[arg(short = 'O', long, value_name = "PATH", allow_hyphen_values = true)]
    pub ontology: Option<PathBuf>,

    /// Merge schema extracted from data
    #[arg(long)]
    pub extract_ontology: bool,

    /// Emit inferred-only triples or the full closure
    #[arg(long, value_enum, default_value_t = EmitMode::Inferred)]
    pub emit: EmitMode,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::NTriples)]
    pub output_format: OutputFormat,

    /// Working directory
    #[arg(short, long, value_name = "PATH", allow_hyphen_values = true)]
    pub work_dir: Option<PathBuf>,

    /// Memory budget (for example: 4G, 2000M)
    #[arg(short = 'm', long, value_name = "SIZE", default_value = "4G")]
    pub memory_budget: MemorySize,

    /// Write run-report.json
    #[arg(long, value_name = "PATH", allow_hyphen_values = true)]
    pub report: Option<PathBuf>,

    /// Safety cap on ABox fixpoint rounds
    #[arg(long, value_name = "N")]
    pub max_iterations: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum EmitMode {
    Inferred,
    Closure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    #[value(name = "ntriples")]
    NTriples,
}

/// A memory size parsed from a human-readable string like "4G" or "2000M".
/// Stores the value in bytes internally.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemorySize(u64);

impl MemorySize {
    pub fn as_bytes(self) -> u64 {
        self.0
    }

    pub fn bytes(self) -> u64 {
        self.as_bytes()
    }
}

impl std::str::FromStr for MemorySize {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("memory size must not be empty".to_string());
        }

        let split_at = trimmed
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(trimmed.len());
        let (digits, suffix) = trimmed.split_at(split_at);
        if digits.is_empty() {
            return Err(format!("invalid memory size: {value}"));
        }

        let base = digits
            .parse::<u64>()
            .map_err(|_| format!("invalid memory size: {value}"))?;
        let multiplier = match suffix.trim().to_ascii_lowercase().as_str() {
            "" | "b" => 1,
            "k" | "kb" => 1024,
            "m" | "mb" => 1024_u64.pow(2),
            "g" | "gb" => 1024_u64.pow(3),
            "t" | "tb" => 1024_u64.pow(4),
            _ => return Err(format!("unsupported memory size suffix in {value}")),
        };

        let bytes = base
            .checked_mul(multiplier)
            .ok_or_else(|| format!("memory size is too large: {value}"))?;
        Ok(Self(bytes))
    }
}

impl Display for MemorySize {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 == 0 {
            return formatter.write_str("0B");
        }

        for (suffix, multiplier) in [
            ("T", 1024_u64.pow(4)),
            ("G", 1024_u64.pow(3)),
            ("M", 1024_u64.pow(2)),
            ("K", 1024_u64),
            ("B", 1),
        ] {
            if self.0.is_multiple_of(multiplier) {
                return write!(formatter, "{}{}", self.0 / multiplier, suffix);
            }
        }

        write!(formatter, "{}B", self.0)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::{Parser, error::ErrorKind};

    use super::{Cli, Commands, EmitMode, MemorySize, OutputFormat};

    #[test]
    fn parses_reason_command_with_defaults() {
        let cli = Cli::try_parse_from(["strix", "reason", "data.nt", "--output", "out.nt"])
            .expect("CLI should parse");

        assert_eq!(cli.verbose, 0);
        assert!(!cli.quiet);
        assert!(!cli.benchmark);

        match cli.command {
            Commands::Reason(args) => {
                assert_eq!(args.data, vec![PathBuf::from("data.nt")]);
                assert_eq!(args.output, PathBuf::from("out.nt"));
                assert_eq!(args.ontology, None);
                assert!(!args.extract_ontology);
                assert_eq!(args.emit, EmitMode::Inferred);
                assert_eq!(args.output_format, OutputFormat::NTriples);
                assert_eq!(args.work_dir, None);
                assert_eq!(args.memory_budget, MemorySize(4 * 1024_u64.pow(3)));
                assert_eq!(args.report, None);
                assert_eq!(args.max_iterations, None);
            }
        }
    }

    #[test]
    fn parses_global_flags_after_the_subcommand() {
        let cli = Cli::try_parse_from([
            "strix",
            "reason",
            "data.nt",
            "--output",
            "out.nt",
            "-vv",
            "--benchmark",
        ])
        .expect("CLI should parse");

        assert_eq!(cli.verbose, 2);
        assert!(!cli.quiet);
        assert!(cli.benchmark);
    }

    #[test]
    fn parses_multiple_data_inputs_and_hyphen_prefixed_option_paths() {
        let cli = Cli::try_parse_from([
            "strix",
            "reason",
            "data.nt",
            "more-data",
            "--output",
            "-out.nt",
            "--ontology",
            "-ontology.owl",
            "--work-dir",
            "-work",
            "--report",
            "-report.json",
        ])
        .expect("CLI should parse hyphen-prefixed paths");

        match cli.command {
            Commands::Reason(args) => {
                assert_eq!(
                    args.data,
                    vec![PathBuf::from("data.nt"), PathBuf::from("more-data")]
                );
                assert_eq!(args.output, PathBuf::from("-out.nt"));
                assert_eq!(args.ontology, Some(PathBuf::from("-ontology.owl")));
                assert_eq!(args.work_dir, Some(PathBuf::from("-work")));
                assert_eq!(args.report, Some(PathBuf::from("-report.json")));
            }
        }
    }

    #[test]
    fn parses_hyphen_prefixed_data_after_terminator() {
        let cli = Cli::try_parse_from(["strix", "reason", "--output", "out.nt", "--", "-data.nt"])
            .expect("CLI should parse hyphen-prefixed positional data after --");

        match cli.command {
            Commands::Reason(args) => {
                assert_eq!(args.data, vec![PathBuf::from("-data.nt")]);
                assert_eq!(args.output, PathBuf::from("out.nt"));
            }
        }
    }

    #[test]
    fn requires_at_least_one_data_input() {
        let error = Cli::try_parse_from(["strix", "reason", "--output", "out.nt"])
            .expect_err("missing positional data should fail");
        assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn requires_a_subcommand() {
        let error = Cli::try_parse_from(["strix"]).expect_err("missing subcommand should fail");
        assert_eq!(
            error.kind(),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }

    #[test]
    fn parses_memory_sizes_with_supported_suffixes() {
        assert_eq!(
            "4G".parse::<MemorySize>().expect("4G should parse").bytes(),
            4 * 1024_u64.pow(3)
        );
        assert_eq!(
            "512mb"
                .parse::<MemorySize>()
                .expect("512mb should parse")
                .bytes(),
            512 * 1024_u64.pow(2)
        );
        assert_eq!(
            "1024"
                .parse::<MemorySize>()
                .expect("raw bytes should parse")
                .bytes(),
            1024
        );
        assert!("12PB".parse::<MemorySize>().is_err());
    }
}
