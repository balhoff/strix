use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct RunReport {
    pub version: u32,
    pub input: InputReport,
    pub rules: RulesReport,
    pub reasoning: ReasoningReport,
    pub peak_rss_bytes: Option<u64>,
    pub wall_time_ms: u128,
    pub ingest_time_ms: u128,
    pub export_time_ms: u128,
}

#[derive(Clone, Debug, Serialize)]
pub struct InputReport {
    pub triples: usize,
    pub tbox_axioms: usize,
    pub dictionary_terms: usize,
    pub output_triples: usize,
    pub memory_budget_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct RulesReport {
    pub supported: Vec<String>,
    pub unsupported_encountered: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReasoningReport {
    pub strata: Vec<StratumReport>,
    pub total_inferred: usize,
    pub total_iterations: usize,
    pub fixpoint_reached: bool,
    pub equality_merges: usize,
    pub equality_iterations: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct StratumReport {
    pub name: String,
    pub iterations: usize,
    pub inferred: usize,
    pub time_ms: u128,
}
