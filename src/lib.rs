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

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::{Parser, ValueEnum, error::ErrorKind};
use tracing_subscriber::EnvFilter;

use bench::StageTimer;
use cli::{Cli, Commands, InconsistencyMode, OutputFormat, ReasonArgs};
use compile::{CompiledSchema, compile_schema};
use dict::{Dictionary, WellKnown};
use engine::inconsistency::{self, Inconsistency};
use engine::{MaterializeResult, ReasoningStats, materialize};
use error::Result;
use output::report::{
    BatchInputReport, BatchRunReport, DatasetRunReport, DatasetStatus, InconsistencyReport,
    InputReport, ReasoningReport, RulesReport, RunReport, StratumReport,
};
use output::{write_batch_run_report, write_ntriples, write_run_report};
use owl::{
    ExtractedSchema, RawSchema, ingest_data_triple, load_extracted_schema, load_ontology_path,
};
use rdf::{RdfInput, discover_inputs, visit_input, visit_path};
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
        benchmark: _,
        command,
    } = cli;
    match command {
        Commands::Reason(reason_args) => run_reason(verbose, quiet, reason_args),
    }
}

fn run_reason(verbose: u8, quiet: bool, args: ReasonArgs) -> Result<()> {
    let filter = match (quiet, verbose) {
        (true, _) => EnvFilter::new("error"),
        (_, 0) => EnvFilter::new("info"),
        (_, 1) => EnvFilter::new("debug"),
        (_, _) => EnvFilter::new("trace"),
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .try_init();

    let wall_clock = Instant::now();

    match args.output_format {
        OutputFormat::NTriples => {}
    }

    if !args.input_merge {
        return run_reason_per_file(args, wall_clock);
    }

    let temp_dir = if args.work_dir.is_none() {
        Some(tempfile::TempDir::new()?)
    } else {
        None
    };
    let work_dir = match &args.work_dir {
        Some(dir) => {
            fs::create_dir_all(dir)?;
            dir.clone()
        }
        None => temp_dir.as_ref().unwrap().path().to_path_buf(),
    };

    let mut dictionary = Dictionary::new();
    let well_known = WellKnown::register(&mut dictionary);
    let mut schema = RawSchema::default();
    let mut extracted_schema = ExtractedSchema::default();
    let total_budget = args.memory_budget.bytes() as usize;
    let store_budget = total_budget / 2;
    let engine_budget = total_budget / 2;
    let mut store = FactStore::new(&work_dir, store_budget)?;
    let extract_schema = args.extract_ontology || args.ontology.is_none();
    let ignore_annotation_axioms = args.ignore_annotation_axioms;
    let mut input_triples = 0usize;
    let mut literal_types: Vec<(dict::TermId, dict::TermId)> = Vec::new();

    tracing::info!("Ingesting data");
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
                &mut literal_types,
            )
        })?;
    }

    if let Some(ontology_path) = &args.ontology {
        tracing::info!("Loading ontology");
        load_ontology_path(
            ontology_path,
            &mut dictionary,
            &mut schema,
            ignore_annotation_axioms,
        )?;
    }

    if extract_schema {
        tracing::info!("Normalizing extracted schema");
        load_extracted_schema(
            &extracted_schema,
            &mut dictionary,
            &mut schema,
            ignore_annotation_axioms,
        )?;
    }
    let ingest_time_ms = ingest_timer.elapsed_ms();

    tracing::info!("Compiling schema");
    let compile_timer = StageTimer::start();
    let compiled_schema = compile_schema(&schema, well_known.owl_thing, well_known.rdfs_literal);
    let schema_compile_time_ms = compile_timer.elapsed_ms();

    inject_schema_assertions(&schema, &mut store, &literal_types)?;

    tracing::info!("Materializing inferences");
    let outcome = materialize_and_check(
        &mut store,
        &compiled_schema,
        args.max_iterations,
        engine_budget,
        well_known.owl_same_as,
        &dictionary,
    )?;

    if args.inconsistency_mode == InconsistencyMode::Halt
        && !outcome.inconsistency_reports.is_empty()
    {
        return Err(halt_error(outcome.inconsistency_reports.len()));
    }

    tracing::info!("Writing output");
    let export_timer = StageTimer::start();
    let written_triples = write_ntriples(
        &args.output,
        args.emit,
        &dictionary,
        &mut store,
        &compiled_schema.proxy_terms,
    )?;
    let export_time_ms = export_timer.elapsed_ms();

    let reasoning_time_ms = outcome.reasoning_time_ms;
    let reasoning_report = build_reasoning_report(
        &compiled_schema,
        schema_compile_time_ms,
        outcome.stats,
        reasoning_time_ms,
        outcome.inconsistency_reports,
        true,
    );

    let total_inferred = reasoning_report.total_inferred;
    tracing::debug!(inferred = total_inferred, "Completed run");

    if let Some(report_path) = &args.report {
        tracing::info!("Writing run report");
        let report = RunReport {
            version: 2,
            input: InputReport {
                triples: input_triples,
                dictionary_terms: dictionary.len(),
                output_triples: written_triples,
                memory_budget_bytes: total_budget as u64,
            },
            ontology: schema.ontology_report(),
            compiled: compiled_schema.compiled_schema_report(),
            rules: RulesReport {
                active: compiled_schema.active_rules(),
                unsupported_encountered: schema.unsupported_constructs(),
            },
            reasoning: reasoning_report,
            peak_rss_bytes: bench::peak_rss_bytes(),
            wall_time_ms: wall_clock.elapsed().as_millis(),
            ingest_time_ms,
            export_time_ms,
        };
        write_run_report(report_path, &report)?;
    }

    Ok(())
}

fn run_reason_per_file(args: ReasonArgs, wall_clock: Instant) -> Result<()> {
    if args.extract_ontology {
        // clap can express the "requires --ontology" constraint declaratively
        // (required_if_eq), but not a conflict that is conditional on a flag's
        // *value*, so this guard stays here.
        anyhow::bail!("--input-merge false cannot be combined with --extract-ontology");
    }
    let ontology_path = args
        .ontology
        .as_ref()
        .expect("clap requires --ontology when --input-merge false");

    let temp_dir = if args.work_dir.is_none() {
        Some(tempfile::TempDir::new()?)
    } else {
        None
    };
    let work_dir = match &args.work_dir {
        Some(dir) => {
            fs::create_dir_all(dir)?;
            dir.clone()
        }
        None => temp_dir.as_ref().unwrap().path().to_path_buf(),
    };

    // Resolve the input/output layout up front so configuration errors fail fast,
    // before the (potentially expensive) ontology load and schema compile.
    let inputs = discover_data_inputs(&args.data)?;
    let output_target = resolve_output_target(&args.output, inputs.len())?;

    let total_budget = args.memory_budget.bytes() as usize;
    let store_budget = total_budget / 2;
    let engine_budget = total_budget / 2;

    let mut dictionary = Dictionary::new();
    let well_known = WellKnown::register(&mut dictionary);
    let mut schema = RawSchema::default();

    tracing::info!("Loading ontology");
    load_ontology_path(
        ontology_path,
        &mut dictionary,
        &mut schema,
        args.ignore_annotation_axioms,
    )?;

    tracing::info!("Compiling schema");
    let compile_timer = StageTimer::start();
    let compiled_schema = compile_schema(&schema, well_known.owl_thing, well_known.rdfs_literal);
    let schema_compile_time_ms = compile_timer.elapsed_ms();

    let ctx = PerFileContext {
        compiled_schema: &compiled_schema,
        schema: &schema,
        owl_same_as: well_known.owl_same_as,
        store_budget,
        engine_budget,
        max_iterations: args.max_iterations,
        emit: args.emit,
    };

    let mut datasets = Vec::with_capacity(inputs.len());
    let mut total_input_triples = 0usize;
    let mut total_output_triples = 0usize;
    let mut total_ingest_time_ms = 0u128;
    let mut total_export_time_ms = 0u128;

    for (index, input) in inputs.iter().enumerate() {
        tracing::info!(input = %input.path.display(), "Processing input file");

        let output_path = match &output_target {
            OutputTarget::File(path) => path.clone(),
            OutputTarget::Directory(dir) => dir.join(per_file_output_name(input, index, args.emit)),
        };
        let dataset_work_dir = work_dir.join(format!("input-{:06}", index + 1));

        // Isolate per-input failures: a parse/IO/reasoning error in one input is
        // recorded against that dataset and the batch continues with the rest.
        let dataset = match process_one_input(
            &ctx,
            &mut dictionary,
            input,
            &output_path,
            &dataset_work_dir,
        ) {
            Ok(dataset) => dataset,
            Err(error) => {
                tracing::error!(
                    input = %input.path.display(),
                    error = %error,
                    "Failed to process input file; continuing with remaining inputs"
                );
                // write_ntriples truncates the output file before streaming, so a
                // failure may have left a partial file here; a stale file from a
                // previous run may also still sit at this path. Remove it so the
                // filesystem matches the recorded `error` status (no output for
                // this input). Best-effort: a missing file is fine to ignore.
                let _ = fs::remove_file(&output_path);
                DatasetRunReport {
                    input_path: input.path.display().to_string(),
                    output_path: output_path.display().to_string(),
                    status: DatasetStatus::Error,
                    error: Some(error.to_string()),
                    input_triples: 0,
                    output_triples: 0,
                    reasoning: None,
                    ingest_time_ms: 0,
                    reasoning_time_ms: 0,
                    export_time_ms: 0,
                }
            }
        };

        // The store created inside process_one_input has been dropped (clearing
        // its segment files); remove the now-empty per-input working directory.
        let _ = fs::remove_dir_all(&dataset_work_dir);

        total_input_triples += dataset.input_triples;
        total_output_triples += dataset.output_triples;
        total_ingest_time_ms += dataset.ingest_time_ms;
        total_export_time_ms += dataset.export_time_ms;
        datasets.push(dataset);
    }

    let failed = datasets
        .iter()
        .filter(|dataset| dataset.status == DatasetStatus::Error)
        .count();
    let inconsistent = datasets
        .iter()
        .filter(|dataset| dataset.status == DatasetStatus::Inconsistent)
        .count();

    if let Some(report_path) = &args.report {
        tracing::info!("Writing batch run report");
        let report = BatchRunReport {
            version: 3,
            input_merge: false,
            inconsistency_mode: args
                .inconsistency_mode
                .to_possible_value()
                .map(|value| value.get_name().to_string())
                .unwrap_or_default(),
            input: BatchInputReport {
                files: inputs.len(),
                triples: total_input_triples,
                dictionary_terms: dictionary.len(),
                output_triples: total_output_triples,
                memory_budget_bytes: total_budget as u64,
            },
            ontology: schema.ontology_report(),
            compiled: compiled_schema.compiled_schema_report(),
            rules: RulesReport {
                active: compiled_schema.active_rules(),
                unsupported_encountered: schema.unsupported_constructs(),
            },
            datasets,
            peak_rss_bytes: bench::peak_rss_bytes(),
            wall_time_ms: wall_clock.elapsed().as_millis(),
            schema_compile_time_ms,
            ingest_time_ms: total_ingest_time_ms,
            export_time_ms: total_export_time_ms,
        };
        write_batch_run_report(report_path, &report)?;
    }

    // The run report (when requested) is written before signalling failure, so
    // per-file diagnostics survive even when the overall run exits non-zero.
    if failed > 0 {
        anyhow::bail!(
            "{failed} of {} input file(s) failed to process (see run report for details)",
            inputs.len()
        );
    }
    if args.inconsistency_mode == InconsistencyMode::Halt && inconsistent > 0 {
        anyhow::bail!(
            "{inconsistent} of {} input file(s) had logical inconsistencies \
             (use --inconsistency-mode report to continue)",
            inputs.len()
        );
    }

    Ok(())
}

/// Invariant context shared across every per-file reasoning pass: the schema is
/// loaded and compiled once, then applied independently to each input.
struct PerFileContext<'a> {
    compiled_schema: &'a CompiledSchema,
    schema: &'a RawSchema,
    owl_same_as: dict::TermId,
    store_budget: usize,
    engine_budget: usize,
    max_iterations: Option<usize>,
    emit: cli::EmitMode,
}

/// Reason over a single input file in isolation: fresh store, ingest, materialize,
/// export. The store is local, so it is dropped (and its segment files cleared) on
/// return. Returns a dataset report on success; any error is propagated for the
/// caller to record without aborting the batch.
fn process_one_input(
    ctx: &PerFileContext,
    dictionary: &mut Dictionary,
    input: &RdfInput,
    output_path: &Path,
    work_dir: &Path,
) -> Result<DatasetRunReport> {
    let mut store = FactStore::new(work_dir, ctx.store_budget)?;
    let mut extracted_schema = ExtractedSchema::default();
    let mut literal_types: Vec<(dict::TermId, dict::TermId)> = Vec::new();
    let mut input_triples = 0usize;

    tracing::info!(input = %input.path.display(), "Ingesting data file");
    let ingest_timer = StageTimer::start();
    visit_input(input, |triple| {
        input_triples += 1;
        ingest_data_triple(
            triple,
            false,
            dictionary,
            &mut extracted_schema,
            &mut store,
            &mut literal_types,
        )
    })?;
    inject_schema_assertions(ctx.schema, &mut store, &literal_types)?;
    let ingest_time_ms = ingest_timer.elapsed_ms();

    tracing::info!(input = %input.path.display(), "Materializing inferences");
    let outcome = materialize_and_check(
        &mut store,
        ctx.compiled_schema,
        ctx.max_iterations,
        ctx.engine_budget,
        ctx.owl_same_as,
        dictionary,
    )?;

    tracing::info!(output = %output_path.display(), "Writing output");
    let export_timer = StageTimer::start();
    let written_triples = write_ntriples(
        output_path,
        ctx.emit,
        dictionary,
        &mut store,
        &ctx.compiled_schema.proxy_terms,
    )?;
    let export_time_ms = export_timer.elapsed_ms();

    let status = if outcome.inconsistency_reports.is_empty() {
        DatasetStatus::Ok
    } else {
        DatasetStatus::Inconsistent
    };
    let reasoning_time_ms = outcome.reasoning_time_ms;
    let reasoning_report = build_reasoning_report(
        ctx.compiled_schema,
        0,
        outcome.stats,
        reasoning_time_ms,
        outcome.inconsistency_reports,
        false,
    );

    Ok(DatasetRunReport {
        input_path: input.path.display().to_string(),
        output_path: output_path.display().to_string(),
        status,
        error: None,
        input_triples,
        output_triples: written_triples,
        reasoning: Some(reasoning_report),
        ingest_time_ms,
        reasoning_time_ms,
        export_time_ms,
    })
}

struct ReasoningOutcome {
    stats: ReasoningStats,
    inconsistency_reports: Vec<InconsistencyReport>,
    reasoning_time_ms: u128,
}

fn materialize_and_check(
    store: &mut FactStore,
    compiled_schema: &CompiledSchema,
    max_iterations: Option<usize>,
    engine_budget: usize,
    owl_same_as: dict::TermId,
    dictionary: &Dictionary,
) -> Result<ReasoningOutcome> {
    let reasoning_timer = StageTimer::start();
    let MaterializeResult {
        stats,
        mut union_find,
        swrl_different_pairs,
        literal_conflicts,
    } = materialize(
        store,
        compiled_schema,
        max_iterations,
        engine_budget,
        owl_same_as,
        dictionary,
    )?;
    let reasoning_time_ms = reasoning_timer.elapsed_ms();

    let mut all_different_pairs = compiled_schema.different_individual_pairs.clone();
    all_different_pairs.extend_from_slice(&swrl_different_pairs);
    all_different_pairs.sort_unstable();
    all_different_pairs.dedup();

    let disjoint_prop_assertions = if !compiled_schema.disjoint_property_pairs.is_empty() {
        let mut relevant_props = std::collections::BTreeSet::new();
        for &(a, b) in &compiled_schema.disjoint_property_pairs {
            relevant_props.insert(a);
            relevant_props.insert(b);
        }
        Some(inconsistency::collect_property_assertions(
            store,
            &relevant_props,
        )?)
    } else {
        None
    };

    if let Some(ref idx) = disjoint_prop_assertions {
        let disjoint_prop_different =
            engine::infer_different_from_disjoint_properties(idx, compiled_schema);
        all_different_pairs.extend_from_slice(&disjoint_prop_different);
    }

    let mut inconsistencies = inconsistency::check_inconsistencies(
        store,
        compiled_schema,
        Some(&mut union_find),
        &all_different_pairs,
        disjoint_prop_assertions.as_ref(),
    )?;
    inconsistencies.extend(literal_conflicts);
    let inconsistency_reports: Vec<InconsistencyReport> = inconsistencies
        .iter()
        .map(|inc| format_inconsistency(inc, dictionary, &compiled_schema.proxy_display))
        .collect();

    if !inconsistencies.is_empty() {
        tracing::warn!(
            count = inconsistencies.len(),
            "Detected logical inconsistencies"
        );
        for report in &inconsistency_reports {
            tracing::warn!(kind = %report.kind, "{}", report.detail);
        }
    }

    Ok(ReasoningOutcome {
        stats,
        inconsistency_reports,
        reasoning_time_ms,
    })
}

/// Error returned when inconsistencies are detected under `--inconsistency-mode halt`.
fn halt_error(count: usize) -> anyhow::Error {
    anyhow::anyhow!(
        "{count} inconsistenc{} detected (use --inconsistency-mode report to continue)",
        if count == 1 { "y" } else { "ies" }
    )
}

fn inject_schema_assertions(
    schema: &RawSchema,
    store: &mut FactStore,
    literal_types: &[(dict::TermId, dict::TermId)],
) -> Result<()> {
    for &(individual, class) in &schema.one_of_types {
        store.insert_asserted_type(individual, class)?;
    }
    for &(subject, predicate, object) in &schema.extra_property_assertions {
        store.insert_asserted_property(subject, predicate, object)?;
    }
    if schema.has_data_range_restrictions {
        for &(literal, datatype) in &schema.literal_datatype_types {
            store.insert_asserted_type(literal, datatype)?;
        }
        for &(literal, datatype) in literal_types {
            store.insert_asserted_type(literal, datatype)?;
        }
    }
    Ok(())
}

fn build_reasoning_report(
    compiled_schema: &CompiledSchema,
    schema_compile_time_ms: u128,
    reasoning_stats: ReasoningStats,
    reasoning_time_ms: u128,
    inconsistency_reports: Vec<InconsistencyReport>,
    include_schema_stratum: bool,
) -> ReasoningReport {
    let total_inferred = reasoning_stats.total_inferred();
    let schema_iterations = if include_schema_stratum {
        compiled_schema.schema_iterations
    } else {
        0
    };
    let mut strata = Vec::new();
    if include_schema_stratum {
        strata.push(StratumReport {
            name: "schema-closure".to_string(),
            iterations: compiled_schema.schema_iterations,
            inferred: 0,
            time_ms: schema_compile_time_ms,
        });
    }
    strata.push(StratumReport {
        name: "abox-materialization".to_string(),
        iterations: reasoning_stats.iterations,
        inferred: total_inferred,
        time_ms: reasoning_time_ms,
    });

    ReasoningReport {
        strata,
        iterations: reasoning_stats.iteration_details,
        total_inferred,
        total_iterations: schema_iterations + reasoning_stats.iterations,
        fixpoint_reached: reasoning_stats.fixpoint_reached,
        equality_merges: reasoning_stats.equality_merges,
        equality_iterations: reasoning_stats.equality_iterations,
        rule_firings: reasoning_stats.rule_firings,
        inconsistencies: inconsistency_reports,
    }
}

fn discover_data_inputs(paths: &[PathBuf]) -> Result<Vec<RdfInput>> {
    let mut inputs = Vec::new();
    for path in paths {
        inputs.extend(discover_inputs(path)?);
    }
    Ok(inputs)
}

/// Where per-file outputs are written. A single input with a non-directory
/// `--output` is written straight to that path (matching merged-mode UX);
/// otherwise each input gets a deterministically-named file under a directory.
enum OutputTarget {
    File(PathBuf),
    Directory(PathBuf),
}

fn resolve_output_target(output_root: &Path, total_inputs: usize) -> Result<OutputTarget> {
    if total_inputs == 1 && !output_root.is_dir() {
        return Ok(OutputTarget::File(output_root.to_path_buf()));
    }
    if output_root.exists() && !output_root.is_dir() {
        anyhow::bail!(
            "--input-merge false with multiple inputs requires --output to be a directory"
        );
    }
    fs::create_dir_all(output_root)?;
    Ok(OutputTarget::Directory(output_root.to_path_buf()))
}

fn per_file_output_name(input: &RdfInput, index: usize, emit: cli::EmitMode) -> String {
    let base_name = input
        .path
        .file_name()
        .map(|name| name.to_string_lossy())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "input".into());
    let safe_name = sanitize_file_name(&base_name);
    let emit_label = match emit {
        cli::EmitMode::Inferred => "inferred",
        cli::EmitMode::Closure => "closure",
    };
    format!("{:06}-{safe_name}.{emit_label}.nt", index + 1)
}

fn sanitize_file_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "input".to_string()
    } else {
        sanitized
    }
}

fn format_term(
    id: dict::TermId,
    dictionary: &Dictionary,
    proxy_display: &BTreeMap<dict::TermId, String>,
) -> String {
    if let Some(display) = proxy_display.get(&id) {
        return display.clone();
    }
    match dictionary.decode(id) {
        Some(term) => term.to_ntriples(),
        None => format!("_{id}"),
    }
}

fn format_inconsistency(
    inc: &Inconsistency,
    dictionary: &Dictionary,
    proxy_display: &BTreeMap<dict::TermId, String>,
) -> InconsistencyReport {
    match inc {
        Inconsistency::DisjointClasses {
            individual,
            class_a,
            class_b,
        } => InconsistencyReport {
            kind: "DisjointClasses".to_string(),
            detail: format!(
                "{} has types {} and {}, which are disjoint",
                format_term(*individual, dictionary, proxy_display),
                format_term(*class_a, dictionary, proxy_display),
                format_term(*class_b, dictionary, proxy_display),
            ),
        },
        Inconsistency::ComplementOf {
            individual,
            class,
            complement,
        } => InconsistencyReport {
            kind: "ComplementOf".to_string(),
            detail: format!(
                "{} has types {} and {}, which are complements",
                format_term(*individual, dictionary, proxy_display),
                format_term(*class, dictionary, proxy_display),
                format_term(*complement, dictionary, proxy_display),
            ),
        },
        Inconsistency::DisjointProperties {
            subject,
            object,
            prop_a,
            prop_b,
        } => InconsistencyReport {
            kind: "DisjointProperties".to_string(),
            detail: format!(
                "({}, {}) appears in both {} and {}, which are disjoint",
                format_term(*subject, dictionary, proxy_display),
                format_term(*object, dictionary, proxy_display),
                format_term(*prop_a, dictionary, proxy_display),
                format_term(*prop_b, dictionary, proxy_display),
            ),
        },
        Inconsistency::MaxCardinalityZero {
            individual,
            class,
            property,
            object,
        } => InconsistencyReport {
            kind: "MaxCardinalityZero".to_string(),
            detail: format!(
                "{} (type {}) has {} link to {}, violating max cardinality 0",
                format_term(*individual, dictionary, proxy_display),
                format_term(*class, dictionary, proxy_display),
                format_term(*property, dictionary, proxy_display),
                format_term(*object, dictionary, proxy_display),
            ),
        },
        Inconsistency::IrreflexiveProperty {
            individual,
            property,
        } => InconsistencyReport {
            kind: "IrreflexiveProperty".to_string(),
            detail: format!(
                "{} has self-link via {}, which is irreflexive",
                format_term(*individual, dictionary, proxy_display),
                format_term(*property, dictionary, proxy_display),
            ),
        },
        Inconsistency::AsymmetricProperty {
            subject,
            object,
            property,
        } => InconsistencyReport {
            kind: "AsymmetricProperty".to_string(),
            detail: format!(
                "{} and {} are linked in both directions via {}, which is asymmetric",
                format_term(*subject, dictionary, proxy_display),
                format_term(*object, dictionary, proxy_display),
                format_term(*property, dictionary, proxy_display),
            ),
        },
        Inconsistency::DifferentIndividuals {
            individual_a,
            individual_b,
        } => InconsistencyReport {
            kind: "DifferentIndividuals".to_string(),
            detail: format!(
                "{} and {} are declared different but were merged by equality reasoning",
                format_term(*individual_a, dictionary, proxy_display),
                format_term(*individual_b, dictionary, proxy_display),
            ),
        },
        Inconsistency::NegativePropertyAssertion {
            subject,
            property,
            object,
        } => InconsistencyReport {
            kind: "NegativePropertyAssertion".to_string(),
            detail: format!(
                "({}, {}, {}) is asserted but negated by a negative property assertion",
                format_term(*subject, dictionary, proxy_display),
                format_term(*property, dictionary, proxy_display),
                format_term(*object, dictionary, proxy_display),
            ),
        },
        Inconsistency::LiteralConflict {
            individual,
            property,
            literal_a,
            literal_b,
        } => InconsistencyReport {
            kind: "LiteralConflict".to_string(),
            detail: format!(
                "{} has values {} and {} for {}, which requires at most one value",
                format_term(*individual, dictionary, proxy_display),
                format_term(*literal_a, dictionary, proxy_display),
                format_term(*literal_b, dictionary, proxy_display),
                format_term(*property, dictionary, proxy_display),
            ),
        },
    }
}
