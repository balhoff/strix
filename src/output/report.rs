use std::fmt::Write as _;

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct InputReport {
    pub triples: usize,
    pub tbox_axioms: usize,
    pub dictionary_terms: usize,
    pub output_triples: usize,
    pub memory_budget_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct RulesReport {
    pub supported: Vec<String>,
    pub unsupported_encountered: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ReasoningReport {
    pub strata: Vec<StratumReport>,
    pub total_inferred: usize,
    pub total_iterations: usize,
    pub fixpoint_reached: bool,
}

#[derive(Clone, Debug)]
pub struct StratumReport {
    pub name: String,
    pub iterations: usize,
    pub inferred: usize,
    pub time_ms: u128,
}

impl RunReport {
    pub fn to_json_pretty(&self) -> String {
        let mut output = String::new();
        output.push_str("{\n");
        line(&mut output, 1, "\"version\":", self.version);
        output.push_str(",\n");
        output.push_str(&format!("{}\"input\": {{\n", indent(1)));
        line(&mut output, 2, "\"triples\":", self.input.triples);
        output.push_str(",\n");
        line(&mut output, 2, "\"tbox_axioms\":", self.input.tbox_axioms);
        output.push_str(",\n");
        line(
            &mut output,
            2,
            "\"dictionary_terms\":",
            self.input.dictionary_terms,
        );
        output.push_str(",\n");
        line(
            &mut output,
            2,
            "\"output_triples\":",
            self.input.output_triples,
        );
        output.push_str(",\n");
        line(
            &mut output,
            2,
            "\"memory_budget_bytes\":",
            self.input.memory_budget_bytes,
        );
        output.push_str("\n");
        output.push_str(&format!("{}}},\n", indent(1)));

        output.push_str(&format!("{}\"rules\": {{\n", indent(1)));
        string_array(&mut output, 2, "\"supported\":", &self.rules.supported);
        output.push_str(",\n");
        string_array(
            &mut output,
            2,
            "\"unsupported_encountered\":",
            &self.rules.unsupported_encountered,
        );
        output.push_str("\n");
        output.push_str(&format!("{}}},\n", indent(1)));

        output.push_str(&format!("{}\"reasoning\": {{\n", indent(1)));
        output.push_str(&format!("{}\"strata\": [\n", indent(2)));
        for (index, stratum) in self.reasoning.strata.iter().enumerate() {
            output.push_str(&format!("{}{{\n", indent(3)));
            string_field(&mut output, 4, "\"name\":", &stratum.name);
            output.push_str(",\n");
            line(&mut output, 4, "\"iterations\":", stratum.iterations);
            output.push_str(",\n");
            line(&mut output, 4, "\"inferred\":", stratum.inferred);
            output.push_str(",\n");
            line(&mut output, 4, "\"time_ms\":", stratum.time_ms);
            output.push_str("\n");
            output.push_str(&format!(
                "{}}}{}\n",
                indent(3),
                if index + 1 == self.reasoning.strata.len() {
                    ""
                } else {
                    ","
                }
            ));
        }
        output.push_str(&format!("{}],\n", indent(2)));
        line(
            &mut output,
            2,
            "\"total_inferred\":",
            self.reasoning.total_inferred,
        );
        output.push_str(",\n");
        line(
            &mut output,
            2,
            "\"total_iterations\":",
            self.reasoning.total_iterations,
        );
        output.push_str(",\n");
        bool_field(
            &mut output,
            2,
            "\"fixpoint_reached\":",
            self.reasoning.fixpoint_reached,
        );
        output.push_str("\n");
        output.push_str(&format!("{}}},\n", indent(1)));

        output.push_str(&format!("{}\"peak_rss_bytes\": ", indent(1)));
        match self.peak_rss_bytes {
            Some(value) => {
                let _ = write!(output, "{value}");
            }
            None => output.push_str("null"),
        }
        output.push_str(",\n");
        line(&mut output, 1, "\"ingest_time_ms\":", self.ingest_time_ms);
        output.push_str(",\n");
        line(&mut output, 1, "\"export_time_ms\":", self.export_time_ms);
        output.push_str(",\n");
        line(&mut output, 1, "\"wall_time_ms\":", self.wall_time_ms);
        output.push_str("\n}\n");
        output
    }
}

fn line<T: std::fmt::Display>(output: &mut String, level: usize, label: &str, value: T) {
    let _ = write!(output, "{}{} {}", indent(level), label, value);
}

fn bool_field(output: &mut String, level: usize, label: &str, value: bool) {
    let _ = write!(
        output,
        "{}{} {}",
        indent(level),
        label,
        if value { "true" } else { "false" }
    );
}

fn string_field(output: &mut String, level: usize, label: &str, value: &str) {
    let _ = write!(
        output,
        "{}{} \"{}\"",
        indent(level),
        label,
        escape_json(value)
    );
}

fn string_array(output: &mut String, level: usize, label: &str, values: &[String]) {
    let _ = write!(output, "{}{} [", indent(level), label);
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        let _ = write!(output, "\"{}\"", escape_json(value));
    }
    output.push(']');
}

fn indent(level: usize) -> String {
    "  ".repeat(level)
}

fn escape_json(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            _ => output.push(character),
        }
    }
    output
}
