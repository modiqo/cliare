use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::artifact_guide::ArtifactGuideSummary;
use crate::ci::CiArtifactSummary;
use crate::context::RuntimeContext;
use crate::fingerprint::TargetFingerprint;
use crate::report::PersonaArtifactSummary;

#[derive(Debug, Clone)]
pub struct MeasurementSummary {
    pub target: TargetFingerprint,
    pub job_id: Option<String>,
    pub job_log_path: Option<PathBuf>,
    pub evidence_path: PathBuf,
    pub shape_path: PathBuf,
    pub command_index_json_path: PathBuf,
    pub command_index_markdown_path: PathBuf,
    pub scorecard_path: PathBuf,
    pub report_path: PathBuf,
    pub ci_summary_path: PathBuf,
    pub sarif_path: PathBuf,
    pub junit_path: PathBuf,
    pub issues_markdown_path: PathBuf,
    pub issues_json_path: PathBuf,
    pub persona_report_count: usize,
    pub readme_path: PathBuf,
    pub agent_skill_path: PathBuf,
    pub facts: MeasurementFacts,
    pub cache_hit: bool,
    pub runtime_context: RuntimeContext,
    pub suite_root_path: PathBuf,
    pub runtime_context_path: Option<PathBuf>,
    pub context_suite_path: Option<PathBuf>,
    pub context_compare_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct MeasurementFacts {
    pub probes_completed: usize,
    pub sandbox_profile: String,
    pub sandbox_root: PathBuf,
    pub sandbox_home: PathBuf,
    pub sandbox_workdir: PathBuf,
    pub sandbox_env_policy: String,
    #[serde(default)]
    pub snapshot_max_files: usize,
    #[serde(default)]
    pub snapshot_max_directories: usize,
    #[serde(default)]
    pub snapshot_max_hash_bytes: u64,
    #[serde(default)]
    pub hostile_binary_containment: bool,
    pub score_total: f64,
    pub score_measured_weight: f64,
    pub score_max_weight: f64,
    pub score_model: String,
    pub score_status: String,
    pub findings: usize,
    #[serde(default)]
    pub commands_precondition_blocked: usize,
    #[serde(default)]
    pub help_text_probes: usize,
    #[serde(default)]
    pub help_text_probes_with_shape: usize,
    #[serde(default)]
    pub help_text_probes_without_shape: usize,
    #[serde(default)]
    pub help_text_probes_not_recognized: usize,
    #[serde(default)]
    pub parser_extraction_rate: f64,
    pub output_contracts_discovered: usize,
    pub machine_readable_output_contracts: usize,
    pub output_mode_probes_completed: usize,
    pub output_mode_parse_successes: usize,
    #[serde(default)]
    pub output_mode_precondition_blocked: usize,
    #[serde(default)]
    pub precondition_blocked_probes: usize,
    #[serde(default)]
    pub auth_required_probes: usize,
    #[serde(default)]
    pub local_context_required_probes: usize,
    #[serde(default)]
    pub fixture_required_probes: usize,
    #[serde(default)]
    pub actionable_precondition_probes: usize,
    #[serde(default)]
    pub precondition_recovery_rate: f64,
    pub side_effect_files_created: usize,
    pub side_effect_files_modified: usize,
    pub side_effect_files_deleted: usize,
    pub side_effect_files_total: usize,
    pub side_effect_probe_count: usize,
    pub credential_like_side_effects: usize,
    #[serde(default)]
    pub side_effect_scan_truncated: bool,
    pub observed_max_depth: usize,
    pub traversal_profile: String,
    pub max_depth: usize,
    pub max_probes: usize,
    pub min_expected_value: u16,
    #[serde(default)]
    pub concurrency_limit: usize,
    #[serde(default)]
    pub traversal_rounds: usize,
    #[serde(default)]
    pub probes_scheduled: usize,
    #[serde(default)]
    pub probes_cancelled: usize,
    pub frontier_remaining: usize,
    pub highest_pending_expected_value: Option<u16>,
    pub candidates_skipped_by_depth: usize,
    pub candidates_skipped_by_convergence: usize,
    pub probes_skipped_by_budget: usize,
    pub budget_exhausted: bool,
    pub traversal_stop_reason: String,
    pub traversal_complete: bool,
}

impl Deref for MeasurementSummary {
    type Target = MeasurementFacts;

    fn deref(&self) -> &Self::Target {
        &self.facts
    }
}

impl DerefMut for MeasurementSummary {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.facts
    }
}

impl MeasurementSummary {
    pub fn set_ci_artifacts(&mut self, artifacts: CiArtifactSummary) {
        self.ci_summary_path = artifacts.summary_path;
        self.sarif_path = artifacts.sarif_path;
        self.junit_path = artifacts.junit_path;
    }

    pub fn set_artifact_guides(&mut self, guides: ArtifactGuideSummary) {
        self.readme_path = guides.readme_path;
        self.agent_skill_path = guides.agent_skill_path;
    }

    pub fn set_persona_artifacts(&mut self, artifacts: PersonaArtifactSummary) {
        let persona_report_count = artifacts.persona_count();
        self.issues_markdown_path = artifacts.issues_markdown_path;
        self.issues_json_path = artifacts.issues_json_path;
        self.persona_report_count = persona_report_count;
    }

    pub fn terminal_summary(&self) -> String {
        let mut lines = vec![
            "CLIARE measure complete".to_owned(),
            format!("target: {}", self.target.requested.display()),
            format!("resolved: {}", self.target.resolved.display()),
            format!(
                "score: {:.0}/100 ({}, measured {:.2}/{:.2}, model {})",
                self.score_total,
                self.score_status,
                self.score_measured_weight,
                self.score_max_weight,
                self.score_model
            ),
            format!("cache: {}", if self.cache_hit { "hit" } else { "miss" }),
        ];
        if let Some(job_id) = &self.job_id {
            lines.push(format!("job_id: {job_id}"));
        }
        if let Some(path) = &self.job_log_path {
            lines.push(format!("progress log: {}", path.display()));
        }
        lines.extend([
            format!("probes: {}", self.probes_completed),
            format!("findings: {}", self.findings),
            "preconditions:".to_owned(),
            format!("  commands blocked: {}", self.commands_precondition_blocked),
            format!("  probes blocked: {}", self.precondition_blocked_probes),
            format!("  auth required: {}", self.auth_required_probes),
            format!(
                "  local context required: {}",
                self.local_context_required_probes
            ),
            format!("  fixture required: {}", self.fixture_required_probes),
            format!(
                "  actionable recovery: {} ({:.1}%)",
                self.actionable_precondition_probes,
                self.precondition_recovery_rate * 100.0
            ),
            "extraction:".to_owned(),
            format!("  help-text probes: {}", self.help_text_probes),
            format!(
                "  with extracted shape: {}",
                self.help_text_probes_with_shape
            ),
            format!(
                "  without extracted shape: {}",
                self.help_text_probes_without_shape
            ),
            format!(
                "  not recognized as help-like: {}",
                self.help_text_probes_not_recognized
            ),
            format!(
                "  parser extraction rate: {:.1}%",
                self.parser_extraction_rate * 100.0
            ),
            "output contracts:".to_owned(),
            format!("  discovered: {}", self.output_contracts_discovered),
            format!(
                "  machine-readable: {}",
                self.machine_readable_output_contracts
            ),
            format!("  probes completed: {}", self.output_mode_probes_completed),
            format!("  parse successes: {}", self.output_mode_parse_successes),
            format!("  blocked: {}", self.output_mode_precondition_blocked),
            "side effects:".to_owned(),
            format!("  file changes: {}", self.side_effect_files_total),
            format!("  probes with changes: {}", self.side_effect_probe_count),
            format!("  created: {}", self.side_effect_files_created),
            format!("  modified: {}", self.side_effect_files_modified),
            format!("  deleted: {}", self.side_effect_files_deleted),
            format!(
                "  credential-like paths: {}",
                self.credential_like_side_effects
            ),
            format!("  scanner truncated: {}", self.side_effect_scan_truncated),
            format!("  scanner max files: {}", self.snapshot_max_files),
            format!(
                "  scanner max directories: {}",
                self.snapshot_max_directories
            ),
            format!("  scanner max hash bytes: {}", self.snapshot_max_hash_bytes),
            "runtime isolation:".to_owned(),
            format!("  sandbox profile: {}", self.sandbox_profile),
            format!("  env policy: {}", self.sandbox_env_policy),
            format!(
                "  hostile binary containment: {}",
                self.hostile_binary_containment
            ),
            format!("  sandbox root: {}", self.sandbox_root.display()),
            format!("  sandbox home: {}", self.sandbox_home.display()),
            format!("  sandbox workdir: {}", self.sandbox_workdir.display()),
            "runtime context:".to_owned(),
            format!("  profile: {}", self.runtime_context.profile.label()),
            format!("  name: {}", self.runtime_context.name),
            format!("  auth: {}", self.runtime_context.auth_state.label()),
            format!(
                "  local context: {}",
                self.runtime_context.local_context_state.label()
            ),
            format!("  fixture: {}", self.runtime_context.fixture_state.label()),
            format!("  network: {}", self.runtime_context.network_state.label()),
            format!(
                "  runtime dependency: {}",
                self.runtime_context.runtime_dependency_state.label()
            ),
            format!("  cwd policy: {}", self.runtime_context.cwd_policy.label()),
            format!("  suite root: {}", self.suite_root_path.display()),
            "coverage pressure:".to_owned(),
            format!("  profile: {}", self.traversal_profile),
            format!(
                "  depth: observed {} / budget {}",
                self.observed_max_depth, self.max_depth
            ),
            format!(
                "  probes: completed {} / budget {}",
                self.probes_completed, self.max_probes
            ),
            format!("  min expected value: {}", self.min_expected_value),
            format!("  concurrency limit: {}", self.concurrency_limit),
            format!("  scheduler rounds: {}", self.traversal_rounds),
            format!("  probes scheduled: {}", self.probes_scheduled),
            format!("  probes cancelled: {}", self.probes_cancelled),
            format!("  frontier remaining: {}", self.frontier_remaining),
            format!(
                "  highest pending expected value: {}",
                self.highest_pending_expected_value
                    .map_or_else(|| "none".to_owned(), |value| value.to_string())
            ),
            format!("  skipped by depth: {}", self.candidates_skipped_by_depth),
            format!(
                "  skipped by convergence: {}",
                self.candidates_skipped_by_convergence
            ),
            format!(
                "  skipped by probe budget: {}",
                self.probes_skipped_by_budget
            ),
            format!("  budget exhausted: {}", self.budget_exhausted),
            format!("  stop reason: {}", self.traversal_stop_reason),
            format!("  traversal complete: {}", self.traversal_complete),
            "artifacts:".to_owned(),
            format!("  evidence: {}", self.evidence_path.display()),
            format!("  shape: {}", self.shape_path.display()),
            format!(
                "  command index: {}",
                self.command_index_json_path.display()
            ),
            format!(
                "  command index report: {}",
                self.command_index_markdown_path.display()
            ),
            format!("  scorecard: {}", self.scorecard_path.display()),
            format!("  report: {}", self.report_path.display()),
            format!("  ci summary: {}", self.ci_summary_path.display()),
            format!("  sarif: {}", self.sarif_path.display()),
            format!("  junit: {}", self.junit_path.display()),
            format!("  issues: {}", self.issues_json_path.display()),
            format!("  issue report: {}", self.issues_markdown_path.display()),
            format!(
                "  persona reports: {} markdown/json pairs",
                self.persona_report_count
            ),
            format!("  readme: {}", self.readme_path.display()),
            format!("  agent guide: {}", self.agent_skill_path.display()),
        ]);
        if let Some(path) = &self.runtime_context_path {
            lines.push(format!("  runtime context: {}", path.display()));
        }
        if let Some(path) = &self.context_suite_path {
            lines.push(format!("  context suite: {}", path.display()));
        }
        if let Some(path) = &self.context_compare_path {
            lines.push(format!("  context comparison: {}", path.display()));
        }

        format!("{}\n", lines.join("\n"))
    }
}
