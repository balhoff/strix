pub mod bench;
pub mod cli;
pub mod compile;
pub mod dict;
pub mod engine;
pub mod error;
pub mod output;
pub mod owl;
pub mod rdf;
pub mod store;

use std::ffi::OsString;
use std::fs;
use std::time::Instant;

use clap::{Parser, error::ErrorKind};

use bench::StageTimer;
use cli::{Cli, Commands, OutputFormat, ReasonArgs};
use compile::compile_schema;
use dict::{Dictionary, WellKnown};
use engine::materialize;
use error::{AppError, Result};
use output::report::{InputReport, ReasoningReport, RulesReport, RunReport, StratumReport};
use output::{write_ntriples, write_run_report};
use owl::{
    ExtractedSchema, RawSchema, ingest_data_triple, load_extracted_schema, load_ontology_path,
};
use rdf::visit_path;
use store::FactStore;

pub fn run<I, S>(args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp
                    | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
                    | ErrorKind::DisplayVersion
            ) =>
        {
            print!("{error}");
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    let Cli {
        verbose,
        quiet,
        benchmark,
        command,
    } = cli;
    match command {
        Commands::Reason(reason_args) => run_reason(verbose, quiet, benchmark, reason_args),
    }
}

fn run_reason(verbose: u8, quiet: bool, benchmark: bool, args: ReasonArgs) -> Result<()> {
    let logger = Logger::new(verbose, quiet);
    let wall_clock = Instant::now();

    match args.output_format {
        OutputFormat::NTriples => {}
    }

    if let Some(work_dir) = &args.work_dir {
        fs::create_dir_all(work_dir)?;
    }

    let mut dictionary = Dictionary::new();
    let _well_known = WellKnown::register(&mut dictionary);
    let mut schema = RawSchema::default();
    let mut extracted_schema = ExtractedSchema::default();
    let mut store = FactStore::default();
    let extract_schema = args.extract_ontology || args.ontology.is_none();
    let ignore_annotation_axioms = args.ignore_annotation_axioms;
    let mut input_triples = 0usize;

    logger.info("Ingesting data");
    let ingest_timer = StageTimer::start();
    for data_path in &args.data {
        visit_path(data_path, |triple| {
            input_triples += 1;
            ingest_data_triple(
                triple,
                extract_schema,
                &mut dictionary,
                &mut extracted_schema,
                &mut store,
            );
            Ok(())
        })?;
    }

    if let Some(ontology_path) = &args.ontology {
        logger.info("Loading ontology");
        load_ontology_path(ontology_path, &mut dictionary, &mut schema, ignore_annotation_axioms)?;
    }

    if extract_schema {
        logger.info("Normalizing extracted schema");
        load_extracted_schema(&extracted_schema, &mut dictionary, &mut schema, ignore_annotation_axioms)?;
    }
    let ingest_time_ms = ingest_timer.elapsed_ms();

    logger.info("Compiling schema");
    let compile_timer = StageTimer::start();
    let compiled_schema = compile_schema(&schema);
    let schema_compile_time_ms = compile_timer.elapsed_ms();

    logger.info("Materializing RDFS closure");
    let reasoning_timer = StageTimer::start();
    let reasoning_stats = materialize(&mut store, &compiled_schema, args.max_iterations)?;
    let reasoning_time_ms = reasoning_timer.elapsed_ms();

    logger.info("Writing output");
    let export_timer = StageTimer::start();
    let written_triples = write_ntriples(&args.output, args.emit, &dictionary, &store)?;
    let export_time_ms = export_timer.elapsed_ms();

    if let Some(report_path) = &args.report {
        logger.info("Writing run report");
        let report = RunReport {
            version: 1,
            input: InputReport {
                triples: input_triples,
                tbox_axioms: schema.total_axioms(),
                dictionary_terms: dictionary.len(),
                output_triples: written_triples,
                memory_budget_bytes: args.memory_budget.bytes(),
            },
            rules: RulesReport {
                supported: compiled_schema
                    .rule_set
                    .rules
                    .iter()
                    .map(|rule| rule.id.to_string())
                    .collect(),
                unsupported_encountered: schema.unsupported_constructs(),
            },
            reasoning: ReasoningReport {
                strata: vec![
                    StratumReport {
                        name: "schema-closure".to_string(),
                        iterations: compiled_schema.schema_iterations,
                        inferred: 0,
                        time_ms: schema_compile_time_ms,
                    },
                    StratumReport {
                        name: "rdfs-abox".to_string(),
                        iterations: reasoning_stats.iterations,
                        inferred: reasoning_stats.total_inferred(),
                        time_ms: reasoning_time_ms,
                    },
                ],
                total_inferred: reasoning_stats.total_inferred(),
                total_iterations: compiled_schema.schema_iterations + reasoning_stats.iterations,
                fixpoint_reached: reasoning_stats.fixpoint_reached,
            },
            peak_rss_bytes: if benchmark {
                bench::peak_rss_bytes()
            } else {
                None
            },
            wall_time_ms: wall_clock.elapsed().as_millis(),
            ingest_time_ms,
            export_time_ms,
        };
        write_run_report(report_path, &report)?;
    }

    logger.debug(&format!(
        "Completed run with {} inferred ABox triples",
        reasoning_stats.total_inferred()
    ));

    Ok(())
}

struct Logger {
    verbose: u8,
    quiet: bool,
}

impl Logger {
    fn new(verbose: u8, quiet: bool) -> Self {
        Self { verbose, quiet }
    }

    fn info(&self, message: &str) {
        if !self.quiet {
            eprintln!("{message}");
        }
    }

    fn debug(&self, message: &str) {
        if !self.quiet && self.verbose > 0 {
            eprintln!("{message}");
        }
    }
}

impl From<clap::Error> for AppError {
    fn from(error: clap::Error) -> Self {
        AppError::new(error.to_string())
    }
}
