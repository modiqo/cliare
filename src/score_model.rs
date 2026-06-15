use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const BUNDLED_SCORE_MODEL: &str = include_str!("../score-models/cliare-score-v0.json");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScoreModelSpec {
    pub schema_version: String,
    pub id: String,
    pub status: ModelStatus,
    pub source: String,
    pub normalization: Normalization,
    pub precision: ScorePrecision,
    pub dimension_weights: DimensionWeights,
    pub scoring: ScoringParameters,
    pub thresholds: FindingThresholds,
    pub claim_priors: ClaimPriors,
    pub evidence_weights: EvidenceWeights,
    pub calibration: CalibrationPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    ExperimentalPartial,
    Calibrating,
    Certified,
    Deprecated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Normalization {
    DeclaredWeight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoreDimension {
    Discovery,
    Grammar,
    Execution,
    Output,
    Safety,
    Recovery,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScorePrecision {
    pub score_decimals: u8,
    pub weight_decimals: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DimensionWeights {
    pub discovery: f64,
    pub grammar: f64,
    pub execution: f64,
    pub recovery: f64,
    pub output: f64,
    pub safety: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScoringParameters {
    pub discovery: DiscoveryScoring,
    pub grammar: GrammarScoring,
    pub output: OutputScoring,
    pub recovery: RecoveryScoring,
    pub safety: SafetyScoring,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DiscoveryScoring {
    pub recognition_weight: f64,
    pub confidence_weight: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GrammarScoring {
    pub flag_presence_score: f64,
    pub flag_confidence_score: f64,
    pub flag_grammar_score: f64,
    pub command_gap_score: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OutputScoring {
    pub advertised_score: f64,
    pub parse_score: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecoveryScoring {
    pub invalid_probe_weight: f64,
    pub precondition_recovery_weight: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SafetyScoring {
    pub changed_probe_penalty: f64,
    pub file_penalty_per_file: f64,
    pub file_penalty_cap: f64,
    pub credential_penalty_per_path: f64,
    pub credential_penalty_cap: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FindingThresholds {
    pub low_runtime_confirmation: f64,
    pub grammar_gap_rate: f64,
    pub recovery_score_minimum: f64,
    pub extraction_limited_min_help_probes: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClaimPriors {
    pub command_exists: f64,
    pub flag_exists: f64,
    pub output_contract_exists: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvidenceWeights {
    pub layout_candidate: f64,
    pub usage_syntax: f64,
    pub runtime_help_match: f64,
    pub runtime_precondition_block: f64,
    pub runtime_rejection: f64,
    pub non_help_output_from_help_probe: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CalibrationPlan {
    pub stage: String,
    pub required_splits: Vec<String>,
    pub metrics: Vec<String>,
    pub freeze_rule: String,
}

#[derive(Debug)]
struct BundledModel {
    spec: ScoreModelSpec,
    sha256: String,
}

impl ScoreModelSpec {
    pub fn bundled() -> &'static Self {
        &bundled_model().spec
    }

    pub fn bundled_sha256() -> &'static str {
        &bundled_model().sha256
    }

    pub fn weight(&self, dimension: ScoreDimension) -> f64 {
        self.dimension_weights.weight(dimension)
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema_version != "cliare.score-model.v1" {
            return Err(format!(
                "unsupported score model schema: {}",
                self.schema_version
            ));
        }
        if self.id.trim().is_empty() {
            return Err("score model id must not be empty".to_owned());
        }
        if self.precision.score_decimals > 1 {
            return Err(
                "experimental score models must not display sub-tenth precision".to_owned(),
            );
        }
        validate_probability(
            "claim_priors.command_exists",
            self.claim_priors.command_exists,
        )?;
        validate_probability("claim_priors.flag_exists", self.claim_priors.flag_exists)?;
        validate_probability(
            "claim_priors.output_contract_exists",
            self.claim_priors.output_contract_exists,
        )?;
        validate_unit_interval(
            "thresholds.low_runtime_confirmation",
            self.thresholds.low_runtime_confirmation,
        )?;
        validate_unit_interval(
            "thresholds.grammar_gap_rate",
            self.thresholds.grammar_gap_rate,
        )?;
        validate_score(
            "thresholds.recovery_score_minimum",
            self.thresholds.recovery_score_minimum,
        )?;
        if self.thresholds.extraction_limited_min_help_probes == 0 {
            return Err("extraction-limited threshold must require at least one probe".to_owned());
        }
        self.dimension_weights.validate()?;
        self.scoring.validate()?;
        if self.calibration.required_splits.len() < 3
            || !self
                .calibration
                .required_splits
                .iter()
                .any(|split| split == "train")
            || !self
                .calibration
                .required_splits
                .iter()
                .any(|split| split == "validation")
            || !self
                .calibration
                .required_splits
                .iter()
                .any(|split| split == "holdout")
        {
            return Err(
                "calibration plan must declare train, validation, and holdout splits".to_owned(),
            );
        }
        Ok(())
    }
}

impl DimensionWeights {
    pub fn weight(&self, dimension: ScoreDimension) -> f64 {
        match dimension {
            ScoreDimension::Discovery => self.discovery,
            ScoreDimension::Grammar => self.grammar,
            ScoreDimension::Execution => self.execution,
            ScoreDimension::Recovery => self.recovery,
            ScoreDimension::Output => self.output,
            ScoreDimension::Safety => self.safety,
        }
    }

    fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("discovery", self.discovery),
            ("grammar", self.grammar),
            ("execution", self.execution),
            ("recovery", self.recovery),
            ("output", self.output),
            ("safety", self.safety),
        ] {
            validate_unit_interval(&format!("dimension_weights.{name}"), value)?;
        }

        let sum = self.discovery
            + self.grammar
            + self.execution
            + self.recovery
            + self.output
            + self.safety;
        if (sum - 1.0).abs() > 0.000_001 {
            return Err(format!("dimension weights must sum to 1.0, got {sum}"));
        }
        Ok(())
    }
}

impl ScoringParameters {
    fn validate(&self) -> Result<(), String> {
        validate_score_mix(
            "scoring.discovery",
            self.discovery.recognition_weight,
            self.discovery.confidence_weight,
            100.0,
        )?;
        validate_score_components(
            "scoring.grammar",
            &[
                self.grammar.flag_presence_score,
                self.grammar.flag_confidence_score,
                self.grammar.flag_grammar_score,
                self.grammar.command_gap_score,
            ],
            100.0,
        )?;
        validate_score_mix(
            "scoring.output",
            self.output.advertised_score,
            self.output.parse_score,
            100.0,
        )?;
        validate_score_mix(
            "scoring.recovery",
            self.recovery.invalid_probe_weight,
            self.recovery.precondition_recovery_weight,
            1.0,
        )?;
        validate_nonnegative(
            "scoring.safety.changed_probe_penalty",
            self.safety.changed_probe_penalty,
        )?;
        validate_nonnegative(
            "scoring.safety.file_penalty_per_file",
            self.safety.file_penalty_per_file,
        )?;
        validate_nonnegative(
            "scoring.safety.file_penalty_cap",
            self.safety.file_penalty_cap,
        )?;
        validate_nonnegative(
            "scoring.safety.credential_penalty_per_path",
            self.safety.credential_penalty_per_path,
        )?;
        validate_nonnegative(
            "scoring.safety.credential_penalty_cap",
            self.safety.credential_penalty_cap,
        )?;
        Ok(())
    }
}

fn bundled_model() -> &'static BundledModel {
    static MODEL: OnceLock<BundledModel> = OnceLock::new();
    MODEL.get_or_init(|| {
        let spec: ScoreModelSpec = serde_json::from_str(BUNDLED_SCORE_MODEL)
            .expect("bundled score model must parse as JSON");
        spec.validate()
            .expect("bundled score model must satisfy invariants");
        BundledModel {
            spec,
            sha256: sha256_hex(BUNDLED_SCORE_MODEL.as_bytes()),
        }
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn validate_probability(field: &str, value: f64) -> Result<(), String> {
    if value > 0.0 && value < 1.0 {
        Ok(())
    } else {
        Err(format!("{field} must be greater than 0 and less than 1"))
    }
}

fn validate_unit_interval(field: &str, value: f64) -> Result<(), String> {
    if (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(format!("{field} must be between 0 and 1"))
    }
}

fn validate_score(field: &str, value: f64) -> Result<(), String> {
    if (0.0..=100.0).contains(&value) {
        Ok(())
    } else {
        Err(format!("{field} must be between 0 and 100"))
    }
}

fn validate_nonnegative(field: &str, value: f64) -> Result<(), String> {
    if value >= 0.0 {
        Ok(())
    } else {
        Err(format!("{field} must be nonnegative"))
    }
}

fn validate_score_mix(field: &str, left: f64, right: f64, expected: f64) -> Result<(), String> {
    validate_nonnegative(field, left)?;
    validate_nonnegative(field, right)?;
    let sum = left + right;
    if (sum - expected).abs() > 0.000_001 {
        return Err(format!("{field} weights must sum to {expected}, got {sum}"));
    }
    Ok(())
}

fn validate_score_components(field: &str, values: &[f64], expected: f64) -> Result<(), String> {
    for value in values {
        validate_nonnegative(field, *value)?;
    }
    let sum = values.iter().sum::<f64>();
    if (sum - expected).abs() > 0.000_001 {
        return Err(format!("{field} weights must sum to {expected}, got {sum}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ScoreDimension, ScoreModelSpec};

    #[test]
    fn bundled_score_model_is_valid_and_hashed() {
        let model = ScoreModelSpec::bundled();

        assert_eq!(model.id, "cliare-score-v0");
        assert_eq!(model.precision.score_decimals, 0);
        assert_eq!(model.weight(ScoreDimension::Discovery), 0.35);
        assert_eq!(ScoreModelSpec::bundled_sha256().len(), 64);
    }

    #[test]
    fn bundled_model_declares_train_validation_and_holdout() {
        let model = ScoreModelSpec::bundled();

        assert!(
            model
                .calibration
                .required_splits
                .iter()
                .any(|split| split == "train")
        );
        assert!(
            model
                .calibration
                .required_splits
                .iter()
                .any(|split| split == "validation")
        );
        assert!(
            model
                .calibration
                .required_splits
                .iter()
                .any(|split| split == "holdout")
        );
        assert!(
            model
                .calibration
                .metrics
                .iter()
                .any(|metric| metric == "expected_calibration_error")
        );
    }
}
