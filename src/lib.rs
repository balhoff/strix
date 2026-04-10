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
use std::time::Instant;

use clap::{Parser, error::ErrorKind};
use tracing_subscriber::EnvFilter;

use bench::StageTimer;
use cli::{Cli, Commands, InconsistencyMode, OutputFormat, ReasonArgs};
use compile::compile_schema;
use dict::{Dictionary, WellKnown};
use engine::inconsistency::{self, Inconsistency};
use engine::{MaterializeResult, materialize};
use error::Result;
use output::report::{
    InconsistencyReport, InputReport, ReasoningReport, RulesReport, RunReport, StratumReport,
};
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

    // Inject type assertions from ontology ABox axioms (OneOf subclass, ClassAssertion)
    for &(individual, class) in &schema.one_of_types {
        store.insert_asserted_type(individual, class)?;
    }
    // Inject property assertions from ontology ABox (ObjectPropertyAssertion, DataPropertyAssertion)
    for &(subject, predicate, object) in &schema.extra_property_assertions {
        store.insert_asserted_property(subject, predicate, object)?;
    }
    // Inject type(literal, datatype) only if the schema uses data range restrictions.
    // Without data range restrictions these are dead weight in the reasoning loop.
    if schema.has_data_range_restrictions {
        for &(literal, datatype) in &schema.literal_datatype_types {
            store.insert_asserted_type(literal, datatype)?;
        }
        for &(literal, datatype) in &literal_types {
            store.insert_asserted_type(literal, datatype)?;
        }
    }

    tracing::info!("Materializing inferences");
    let reasoning_timer = StageTimer::start();
    let MaterializeResult {
        stats: reasoning_stats,
        mut union_find,
        swrl_different_pairs,
        literal_conflicts,
    } = materialize(
        &mut store,
        &compiled_schema,
        args.max_iterations,
        engine_budget,
        well_known.owl_same_as,
        &dictionary,
    )?;
    let reasoning_time_ms = reasoning_timer.elapsed_ms();

    let mut all_different_pairs = compiled_schema.different_individual_pairs.clone();
    all_different_pairs.extend_from_slice(&swrl_different_pairs);

    // Build disjoint-property assertions index once, shared by both
    // DifferentIndividuals inference and DisjointProperties inconsistency check.
    let disjoint_prop_assertions = if !compiled_schema.disjoint_property_pairs.is_empty() {
        let mut relevant_props = std::collections::BTreeSet::new();
        for &(a, b) in &compiled_schema.disjoint_property_pairs {
            relevant_props.insert(a);
            relevant_props.insert(b);
        }
        Some(inconsistency::collect_property_assertions(
            &mut store,
            &relevant_props,
        )?)
    } else {
        None
    };

    if let Some(ref idx) = disjoint_prop_assertions {
        let disjoint_prop_different =
            engine::infer_different_from_disjoint_properties(idx, &compiled_schema);
        all_different_pairs.extend_from_slice(&disjoint_prop_different);
    }

    let mut inconsistencies = inconsistency::check_inconsistencies(
        &mut store,
        &compiled_schema,
        Some(&mut union_find),
        &all_different_pairs,
        disjoint_prop_assertions.as_ref(),
    )?;
    inconsistencies.extend(literal_conflicts);
    let inconsistency_reports: Vec<InconsistencyReport> = inconsistencies
        .iter()
        .map(|inc| format_inconsistency(inc, &dictionary, &compiled_schema.proxy_display))
        .collect();

    if !inconsistencies.is_empty() {
        tracing::warn!(
            count = inconsistencies.len(),
            "Detected logical inconsistencies"
        );
        for report in &inconsistency_reports {
            tracing::warn!(kind = %report.kind, "{}", report.detail);
        }
        if args.inconsistency_mode == InconsistencyMode::Halt {
            anyhow::bail!(
                "{} inconsistenc{} detected (use --inconsistency-mode report to continue)",
                inconsistencies.len(),
                if inconsistencies.len() == 1 {
                    "y"
                } else {
                    "ies"
                }
            );
        }
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

    if let Some(report_path) = &args.report {
        tracing::info!("Writing run report");
        let report = RunReport {
            version: 1,
            input: InputReport {
                triples: input_triples,
                tbox_axioms: schema.total_axioms(),
                dictionary_terms: dictionary.len(),
                output_triples: written_triples,
                memory_budget_bytes: total_budget as u64,
            },
            rules: RulesReport {
                supported: compiled_schema.rule_set.rule_ids(),
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
                equality_merges: reasoning_stats.equality_merges,
                equality_iterations: reasoning_stats.equality_iterations,
                inconsistencies: inconsistency_reports,
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

    tracing::debug!(inferred = reasoning_stats.total_inferred(), "Completed run");

    Ok(())
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
