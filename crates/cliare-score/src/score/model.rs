use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;

use cliare_context::RuntimeContext;
use cliare_runtime::fingerprint::TargetFingerprint;
use cliare_runtime::sandbox::{SandboxMetadata, SnapshotLimits};

use super::Dimension;
use super::labels::env_policy_label;

#[derive(Debug, Serialize)]
pub struct Scorecard {
    pub(super) schema_version: &'static str,
    pub(super) target: TargetFingerprint,
    pub(super) runtime_context: RuntimeContext,
    pub(super) score: ScoreSummary,
    pub(super) subscores: BTreeMap<Dimension, DimensionScore>,
    pub(super) coverage: Coverage,
    pub(super) findings: Vec<Finding>,
    pub(super) model: ScoreModel,
}

#[derive(Debug, Serialize)]
pub struct ScoreSummary {
    pub(super) total: f64,
    pub(super) measured_weight: f64,
    pub(super) max_weight: f64,
    pub(super) model: String,
    pub(super) status: ScoreStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoreStatus {
    ExperimentalPartial,
}

#[derive(Debug, Serialize)]
pub struct DimensionScore {
    pub(super) score: Option<f64>,
    pub(super) weight: f64,
    pub(super) status: DimensionStatus,
    pub(super) rationale: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DimensionStatus {
    Measured,
    NotMeasured,
}

#[derive(Debug, Serialize)]
pub struct Coverage {
    pub(super) sandbox_profile: &'static str,
    pub(super) sandbox_root: PathBuf,
    pub(super) sandbox_home: PathBuf,
    pub(super) sandbox_workdir: PathBuf,
    pub(super) sandbox_env_policy: &'static str,
    pub(super) snapshot_max_files: usize,
    pub(super) snapshot_max_directories: usize,
    pub(super) snapshot_max_hash_bytes: u64,
    pub(super) hostile_binary_containment: bool,
    pub(super) commands_discovered: usize,
    pub(super) commands_runtime_confirmed: usize,
    pub(super) commands_precondition_blocked: usize,
    pub(super) command_confirmation_rate: f64,
    pub(super) help_text_probes: usize,
    pub(super) help_text_probes_with_shape: usize,
    pub(super) help_text_probes_without_shape: usize,
    pub(super) help_text_probes_not_recognized: usize,
    pub(super) parser_extraction_rate: f64,
    pub(super) flags_discovered: usize,
    pub(super) output_contracts_discovered: usize,
    pub(super) machine_readable_output_contracts: usize,
    pub(super) output_mode_probes_completed: usize,
    pub(super) output_mode_parse_successes: usize,
    pub(super) output_mode_precondition_blocked: usize,
    pub(super) output_mode_help_text_probes: usize,
    pub(super) side_effect_files_created: usize,
    pub(super) side_effect_files_modified: usize,
    pub(super) side_effect_files_deleted: usize,
    pub(super) side_effect_files_total: usize,
    pub(super) side_effect_probe_count: usize,
    pub(super) credential_like_side_effects: usize,
    pub(super) side_effect_scan_truncated: bool,
    pub(super) avg_command_confidence: f64,
    pub(super) avg_flag_confidence: f64,
    pub(super) observed_max_depth: usize,
    pub(super) traversal_profile: &'static str,
    pub(super) max_depth: usize,
    pub(super) max_probes: usize,
    pub(super) min_expected_value: u16,
    pub(super) concurrency_limit: usize,
    pub(super) traversal_rounds: usize,
    pub(super) probes_scheduled: usize,
    pub(super) probes_completed: usize,
    pub(super) probes_cancelled: usize,
    pub(super) probes_timed_out: usize,
    pub(super) probes_failed_to_spawn: usize,
    pub(super) precondition_blocked_probes: usize,
    pub(super) auth_required_probes: usize,
    pub(super) local_context_required_probes: usize,
    pub(super) fixture_required_probes: usize,
    pub(super) actionable_precondition_probes: usize,
    pub(super) precondition_recovery_rate: f64,
    pub(super) frontier_remaining: usize,
    pub(super) highest_pending_expected_value: Option<u16>,
    pub(super) candidates_skipped_by_depth: usize,
    pub(super) candidates_skipped_by_convergence: usize,
    pub(super) probes_skipped_by_budget: usize,
    pub(super) budget_exhausted: bool,
    pub(super) traversal_stop_reason: TraversalStopReason,
    pub(super) traversal_complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraversalStopReason {
    FrontierExhausted,
    Converged,
    DepthBudgetExhausted,
    ProbeBudgetExhausted,
}

#[derive(Debug, Serialize)]
pub struct Finding {
    pub(super) id: &'static str,
    pub(super) dimension: Dimension,
    pub(super) severity: Severity,
    pub(super) title: &'static str,
    pub(super) detail: String,
    pub(super) recommendation: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Serialize)]
pub struct ScoreModel {
    pub(super) name: String,
    pub(super) sha256: String,
    pub(super) source: String,
    pub(super) status: String,
    pub(super) normalization: String,
}

#[derive(Debug, Clone)]
pub struct ScoreArtifactSummary {
    pub scorecard_path: PathBuf,
    pub report_path: PathBuf,
    pub total: f64,
    pub measured_weight: f64,
    pub max_weight: f64,
    pub model: String,
    pub status: &'static str,
    pub findings: usize,
    pub commands_precondition_blocked: usize,
    pub help_text_probes: usize,
    pub help_text_probes_with_shape: usize,
    pub help_text_probes_without_shape: usize,
    pub help_text_probes_not_recognized: usize,
    pub parser_extraction_rate: f64,
    pub output_contracts_discovered: usize,
    pub machine_readable_output_contracts: usize,
    pub output_mode_probes_completed: usize,
    pub output_mode_parse_successes: usize,
    pub output_mode_precondition_blocked: usize,
    pub precondition_blocked_probes: usize,
    pub auth_required_probes: usize,
    pub local_context_required_probes: usize,
    pub fixture_required_probes: usize,
    pub actionable_precondition_probes: usize,
    pub precondition_recovery_rate: f64,
    pub side_effect_files_created: usize,
    pub side_effect_files_modified: usize,
    pub side_effect_files_deleted: usize,
    pub side_effect_files_total: usize,
    pub side_effect_probe_count: usize,
    pub credential_like_side_effects: usize,
    pub side_effect_scan_truncated: bool,
    pub observed_max_depth: usize,
    pub traversal_profile: &'static str,
    pub max_depth: usize,
    pub max_probes: usize,
    pub min_expected_value: u16,
    pub concurrency_limit: usize,
    pub traversal_rounds: usize,
    pub probes_scheduled: usize,
    pub probes_cancelled: usize,
    pub frontier_remaining: usize,
    pub highest_pending_expected_value: Option<u16>,
    pub candidates_skipped_by_depth: usize,
    pub candidates_skipped_by_convergence: usize,
    pub probes_skipped_by_budget: usize,
    pub budget_exhausted: bool,
    pub traversal_stop_reason: &'static str,
    pub traversal_complete: bool,
    pub sandbox_profile: &'static str,
    pub sandbox_root: PathBuf,
    pub sandbox_home: PathBuf,
    pub sandbox_workdir: PathBuf,
    pub sandbox_env_policy: &'static str,
    pub snapshot_max_files: usize,
    pub snapshot_max_directories: usize,
    pub snapshot_max_hash_bytes: u64,
    pub hostile_binary_containment: bool,
    pub runtime_context: RuntimeContext,
}

#[derive(Debug, Clone, Default)]
pub struct ScoreRunContext {
    pub traversal_profile: &'static str,
    pub max_depth: usize,
    pub max_probes: usize,
    pub min_expected_value: u16,
    pub concurrency_limit: usize,
    pub traversal_rounds: usize,
    pub probes_scheduled: usize,
    pub probes_cancelled: usize,
    pub frontier_remaining: usize,
    pub highest_pending_expected_value: Option<u16>,
    pub candidates_skipped_by_depth: usize,
    pub candidates_skipped_by_convergence: usize,
    pub sandbox: SandboxScoreContext,
    pub runtime_context: RuntimeContext,
}

#[derive(Debug, Clone, Default)]
pub struct SandboxScoreContext {
    pub profile: &'static str,
    pub root: PathBuf,
    pub home: PathBuf,
    pub workdir: PathBuf,
    pub env_policy: &'static str,
    pub snapshot_limits: SnapshotLimits,
    pub hostile_binary_containment: bool,
}

impl From<&SandboxMetadata> for SandboxScoreContext {
    fn from(metadata: &SandboxMetadata) -> Self {
        Self {
            profile: metadata.profile.label(),
            root: metadata.root.clone(),
            home: metadata.home.clone(),
            workdir: metadata.workdir.clone(),
            env_policy: env_policy_label(metadata.env_policy),
            snapshot_limits: metadata.snapshot_limits,
            hostile_binary_containment: metadata.hostile_binary_containment,
        }
    }
}
