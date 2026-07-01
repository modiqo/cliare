use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(super) struct ShapeArtifact {
    pub(super) schema_version: String,
    #[serde(default)]
    pub(super) target: Value,
    #[serde(default)]
    pub(super) commands: Vec<ShapeCommand>,
    #[serde(default)]
    pub(super) flags: Vec<ShapeFlag>,
    #[serde(default)]
    pub(super) output_contracts: Vec<ShapeOutputContract>,
    #[serde(default)]
    pub(super) model: Value,
}

#[derive(Debug, Deserialize)]
pub(super) struct ShapeCommand {
    #[serde(default)]
    pub(super) path: Vec<String>,
    #[serde(default)]
    pub(super) aliases: Vec<String>,
    #[serde(default)]
    pub(super) positionals: Vec<ShapePositional>,
    #[serde(default)]
    pub(super) preconditions: Vec<String>,
    #[serde(default)]
    pub(super) evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ShapeFlag {
    #[serde(default)]
    pub(super) command_path: Vec<String>,
    pub(super) name: String,
    #[serde(default)]
    pub(super) short: Option<String>,
    #[serde(default = "default_flag_value_kind")]
    pub(super) value_kind: String,
    #[serde(default)]
    pub(super) value_name: Option<String>,
    #[serde(default)]
    pub(super) required: bool,
    #[serde(default)]
    pub(super) repeatable: bool,
    #[serde(default)]
    pub(super) evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ShapePositional {
    pub(super) name: String,
    #[serde(default)]
    pub(super) required: bool,
    #[serde(default)]
    pub(super) variadic: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct ShapeOutputContract {
    #[serde(default)]
    pub(super) command_path: Vec<String>,
    pub(super) mode: String,
    pub(super) flag_name: String,
    #[serde(default)]
    pub(super) argv_fragment: Vec<String>,
    #[serde(default)]
    pub(super) parse_success: bool,
    #[serde(default)]
    pub(super) preconditions: Vec<String>,
    #[serde(default)]
    pub(super) evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ShapeTruth {
    pub(super) schema_version: String,
    #[serde(default)]
    pub(super) target_id: Option<String>,
    #[serde(default)]
    pub(super) commands: Vec<TruthCommand>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TruthCommand {
    #[serde(default)]
    pub(super) path: Vec<String>,
    #[serde(default)]
    pub(super) aliases: Vec<String>,
    #[serde(default)]
    pub(super) positionals: Vec<TruthPositional>,
    #[serde(default)]
    pub(super) flags: Vec<TruthFlag>,
    #[serde(default)]
    pub(super) output_contracts: Vec<TruthOutputContract>,
    #[serde(default)]
    pub(super) preconditions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TruthFlag {
    pub(super) name: String,
    #[serde(default)]
    pub(super) short: Option<String>,
    #[serde(default = "default_flag_value_kind")]
    pub(super) value_kind: String,
    #[serde(default)]
    pub(super) value_name: Option<String>,
    #[serde(default)]
    pub(super) required: bool,
    #[serde(default)]
    pub(super) repeatable: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct TruthPositional {
    pub(super) name: String,
    #[serde(default)]
    pub(super) required: bool,
    #[serde(default)]
    pub(super) variadic: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct TruthOutputContract {
    pub(super) mode: String,
    pub(super) flag_name: String,
    #[serde(default)]
    pub(super) argv_fragment: Vec<String>,
    #[serde(default = "truth_parseable")]
    pub(super) parseable: bool,
}

#[derive(Debug, Serialize)]
pub struct ShapeQualityReport {
    pub schema_version: &'static str,
    pub shape_path: String,
    pub truth_path: String,
    pub shape_schema_version: String,
    pub truth_schema_version: String,
    pub target_id: Option<String>,
    pub overall: ShapeQualityOverall,
    pub metrics: ShapeQualityMetrics,
    pub provenance: ProvenanceReport,
}

#[derive(Debug, Serialize)]
pub struct ShapeQualityOverall {
    pub score: Option<f64>,
    pub mean_f1: Option<f64>,
    pub metrics_scored: usize,
}

#[derive(Debug, Serialize)]
pub struct ShapeQualityMetrics {
    pub commands: MetricReport,
    pub aliases: MetricReport,
    pub flags: MetricReport,
    pub flag_grammar: MetricReport,
    pub positionals: MetricReport,
    pub output_contracts: MetricReport,
    pub preconditions: MetricReport,
}

#[derive(Debug, Serialize)]
pub struct MetricReport {
    pub expected: usize,
    pub observed: usize,
    pub matched: usize,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1: Option<f64>,
    pub missing: Vec<String>,
    pub unexpected: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProvenanceReport {
    pub target_present: bool,
    pub model_present: bool,
    pub command_evidence: EvidenceCompleteness,
    pub flag_evidence: EvidenceCompleteness,
    pub output_contract_evidence: EvidenceCompleteness,
}

#[derive(Debug, Serialize)]
pub struct EvidenceCompleteness {
    pub total: usize,
    pub with_evidence: usize,
    pub rate: Option<f64>,
}

fn default_flag_value_kind() -> String {
    "boolean".to_owned()
}

fn truth_parseable() -> bool {
    true
}
