use std::ffi::OsString;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use crate::error::Result;

#[derive(Clone, Debug)]
pub struct Cli {
    pub verbose: u8,
    pub quiet: bool,
    pub benchmark: bool,
    pub command: Command,
}

#[derive(Clone, Debug)]
pub enum Command {
    Help,
    Reason(ReasonArgs),
}

#[derive(Clone, Debug)]
pub struct ReasonArgs {
    pub data: PathBuf,
    pub output: PathBuf,
    pub ontology: Option<PathBuf>,
    pub extract_ontology: bool,
    pub emit: EmitMode,
    pub output_format: OutputFormat,
    pub work_dir: Option<PathBuf>,
    pub memory_budget: MemorySize,
    pub report: Option<PathBuf>,
    pub max_iterations: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmitMode {
    Inferred,
    Closure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    NTriples,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemorySize(u64);

impl MemorySize {
    pub fn parse(value: &str) -> std::result::Result<Self, CliError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(CliError::new("memory size must not be empty"));
        }

        let split_at = trimmed
            .find(|character: char| !character.is_ascii_digit())
            .unwrap_or(trimmed.len());
        let (digits, suffix) = trimmed.split_at(split_at);
        if digits.is_empty() {
            return Err(CliError::new(format!("invalid memory size: {value}")));
        }

        let base = digits
            .parse::<u64>()
            .map_err(|_| CliError::new(format!("invalid memory size: {value}")))?;
        let multiplier = match suffix.to_ascii_lowercase().as_str() {
            "" | "b" => 1,
            "k" | "kb" => 1024,
            "m" | "mb" => 1024_u64.pow(2),
            "g" | "gb" => 1024_u64.pow(3),
            "t" | "tb" => 1024_u64.pow(4),
            _ => {
                return Err(CliError::new(format!(
                    "unsupported memory size suffix in {value}"
                )));
            }
        };

        Ok(Self(base.saturating_mul(multiplier)))
    }

    pub fn bytes(self) -> u64 {
        self.0
    }
}

impl Cli {
    pub fn parse<I, S>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let arguments = args.into_iter().map(Into::into).collect::<Vec<_>>();
        Self::parse_strings(arguments).map_err(Into::into)
    }

    pub fn usage() -> String {
        let binary = std::env::args_os()
            .next()
            .unwrap_or_else(|| OsString::from("strix"));
        let name = binary.to_string_lossy();
        format!(
            "\
{name} <COMMAND> [OPTIONS]

Commands:
  reason    Run ingest, schema compilation, reasoning, and export

Global options:
  -v, --verbose      Increase logging verbosity
  -q, --quiet        Errors only
      --benchmark    Attempt to collect runtime metrics
  -h, --help         Show this help

Reason options:
  -d, --data <PATH>             Input RDF dataset (supported RDF file or directory)
  -o, --output <PATH>           Output file for inferred triples
  -O, --ontology <PATH>         Separate ontology input (supported RDF file or directory)
      --extract-ontology        Merge schema extracted from data
      --emit <MODE>             inferred (default) or closure
      --output-format <FORMAT>  ntriples (default)
  -w, --work-dir <PATH>         Working directory
  -m, --memory-budget <SIZE>    Memory budget (default: 4G)
      --report <PATH>           Write run-report.json
      --max-iterations <N>      Safety cap on ABox fixpoint rounds
"
        )
    }

    fn parse_strings(arguments: Vec<String>) -> std::result::Result<Self, CliError> {
        if arguments.len() <= 1 {
            return Ok(Self {
                verbose: 0,
                quiet: false,
                benchmark: false,
                command: Command::Help,
            });
        }

        let mut index = 1usize;
        let mut verbose = 0u8;
        let mut quiet = false;
        let mut benchmark = false;

        while index < arguments.len() {
            match arguments[index].as_str() {
                "-h" | "--help" => {
                    return Ok(Self {
                        verbose,
                        quiet,
                        benchmark,
                        command: Command::Help,
                    });
                }
                "-v" | "--verbose" => {
                    verbose = verbose.saturating_add(1);
                    index += 1;
                }
                "-q" | "--quiet" => {
                    quiet = true;
                    index += 1;
                }
                "--benchmark" => {
                    benchmark = true;
                    index += 1;
                }
                "reason" => {
                    let reason_args = parse_reason_args(&arguments[index + 1..])?;
                    return Ok(Self {
                        verbose,
                        quiet,
                        benchmark,
                        command: Command::Reason(reason_args),
                    });
                }
                unknown => {
                    return Err(CliError::new(format!(
                        "unrecognized argument or command: {unknown}\n\n{}",
                        Self::usage()
                    )));
                }
            }
        }

        Ok(Self {
            verbose,
            quiet,
            benchmark,
            command: Command::Help,
        })
    }
}

fn parse_reason_args(arguments: &[String]) -> std::result::Result<ReasonArgs, CliError> {
    let mut data = None;
    let mut output = None;
    let mut ontology = None;
    let mut extract_ontology = false;
    let mut emit = EmitMode::Inferred;
    let mut output_format = OutputFormat::NTriples;
    let mut work_dir = None;
    let mut memory_budget = MemorySize::parse("4G")?;
    let mut report = None;
    let mut max_iterations = None;

    let mut index = 0usize;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "-h" | "--help" => {
                return Err(CliError::new(Cli::usage()));
            }
            "-d" | "--data" => {
                let value = next_value(arguments, &mut index, "--data")?;
                data = Some(PathBuf::from(value));
            }
            "-o" | "--output" => {
                let value = next_value(arguments, &mut index, "--output")?;
                output = Some(PathBuf::from(value));
            }
            "-O" | "--ontology" => {
                let value = next_value(arguments, &mut index, "--ontology")?;
                ontology = Some(PathBuf::from(value));
            }
            "--extract-ontology" => {
                extract_ontology = true;
                index += 1;
            }
            "--emit" => {
                let value = next_value(arguments, &mut index, "--emit")?;
                emit = match value {
                    "inferred" => EmitMode::Inferred,
                    "closure" => EmitMode::Closure,
                    _ => {
                        return Err(CliError::new(format!("unsupported emit mode: {value}")));
                    }
                };
            }
            "--output-format" => {
                let value = next_value(arguments, &mut index, "--output-format")?;
                output_format = match value {
                    "ntriples" => OutputFormat::NTriples,
                    _ => {
                        return Err(CliError::new(format!("unsupported output format: {value}")));
                    }
                };
            }
            "-w" | "--work-dir" => {
                let value = next_value(arguments, &mut index, "--work-dir")?;
                work_dir = Some(PathBuf::from(value));
            }
            "-m" | "--memory-budget" => {
                let value = next_value(arguments, &mut index, "--memory-budget")?;
                memory_budget = MemorySize::parse(value)?;
            }
            "--report" => {
                let value = next_value(arguments, &mut index, "--report")?;
                report = Some(PathBuf::from(value));
            }
            "--max-iterations" => {
                let value = next_value(arguments, &mut index, "--max-iterations")?;
                max_iterations = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| CliError::new(format!("invalid iteration count: {value}")))?,
                );
            }
            unknown => {
                return Err(CliError::new(format!(
                    "unrecognized reason option: {unknown}\n\n{}",
                    Cli::usage()
                )));
            }
        }
    }

    let data = data.ok_or_else(|| {
        CliError::new(format!(
            "missing required option --data\n\n{}",
            Cli::usage()
        ))
    })?;
    let output = output.ok_or_else(|| {
        CliError::new(format!(
            "missing required option --output\n\n{}",
            Cli::usage()
        ))
    })?;

    Ok(ReasonArgs {
        data,
        output,
        ontology,
        extract_ontology,
        emit,
        output_format,
        work_dir,
        memory_budget,
        report,
        max_iterations,
    })
}

fn next_value<'a>(
    arguments: &'a [String],
    index: &mut usize,
    flag: &str,
) -> std::result::Result<&'a str, CliError> {
    let value_index = *index + 1;
    if value_index >= arguments.len() {
        return Err(CliError::new(format!("missing value for {flag}")));
    }
    *index += 2;
    Ok(arguments[value_index].as_str())
}

#[derive(Debug, Clone)]
pub struct CliError {
    message: String,
}

impl CliError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for CliError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}
