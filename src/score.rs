use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tokio::fs;

use crate::claims::{
    ClaimSet, CommandClaim, FlagClaim, FlagValueKind, OutputContractClaim, OutputContractScope,
};
use crate::error::{CliareError, Result};
use crate::evidence::{ProbeIntent, ProcessStatus};
use crate::fingerprint::TargetFingerprint;
use crate::observation::ShapeObservation;
use crate::output::ObservedOutputKind;
use crate::precondition::{self, PreconditionKind};
use crate::sandbox::SandboxMetadata;

const SCHEMA_VERSION: &str = "cliare.scorecard.v1";
const SCORE_MODEL: &str = "cliare-score-v0";

#[derive(Debug, Serialize)]
pub struct Scorecard {
    schema_version: &'static str,
    target: TargetFingerprint,
    score: ScoreSummary,
    subscores: BTreeMap<Dimension, DimensionScore>,
    coverage: Coverage,
    findings: Vec<Finding>,
    model: ScoreModel,
}

#[derive(Debug, Serialize)]
pub struct ScoreSummary {
    total: f64,
    measured_weight: f64,
    max_weight: f64,
    model: &'static str,
    status: ScoreStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoreStatus {
    ExperimentalPartial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    Discovery,
    Grammar,
    Execution,
    Output,
    Safety,
    Recovery,
}

#[derive(Debug, Serialize)]
pub struct DimensionScore {
    score: Option<f64>,
    weight: f64,
    status: DimensionStatus,
    rationale: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DimensionStatus {
    Measured,
    NotMeasured,
}

#[derive(Debug, Serialize)]
pub struct Coverage {
    sandbox_profile: &'static str,
    sandbox_root: PathBuf,
    sandbox_home: PathBuf,
    sandbox_workdir: PathBuf,
    sandbox_env_policy: &'static str,
    commands_discovered: usize,
    commands_runtime_confirmed: usize,
    commands_precondition_blocked: usize,
    command_confirmation_rate: f64,
    flags_discovered: usize,
    output_contracts_discovered: usize,
    machine_readable_output_contracts: usize,
    output_mode_probes_completed: usize,
    output_mode_parse_successes: usize,
    output_mode_precondition_blocked: usize,
    output_mode_help_text_probes: usize,
    side_effect_files_created: usize,
    side_effect_files_modified: usize,
    side_effect_files_deleted: usize,
    side_effect_files_total: usize,
    side_effect_probe_count: usize,
    credential_like_side_effects: usize,
    avg_command_confidence: f64,
    avg_flag_confidence: f64,
    observed_max_depth: usize,
    traversal_profile: &'static str,
    max_depth: usize,
    max_probes: usize,
    min_expected_value: u16,
    concurrency_limit: usize,
    traversal_rounds: usize,
    probes_scheduled: usize,
    probes_completed: usize,
    probes_cancelled: usize,
    probes_timed_out: usize,
    probes_failed_to_spawn: usize,
    precondition_blocked_probes: usize,
    auth_required_probes: usize,
    frontier_remaining: usize,
    highest_pending_expected_value: Option<u16>,
    candidates_skipped_by_depth: usize,
    candidates_skipped_by_convergence: usize,
    probes_skipped_by_budget: usize,
    budget_exhausted: bool,
    traversal_stop_reason: TraversalStopReason,
    traversal_complete: bool,
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
    id: &'static str,
    dimension: Dimension,
    severity: Severity,
    title: &'static str,
    detail: String,
    recommendation: &'static str,
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
    name: &'static str,
    source: &'static str,
}

#[derive(Debug, Clone)]
pub struct ScoreArtifactSummary {
    pub scorecard_path: PathBuf,
    pub report_path: PathBuf,
    pub total: f64,
    pub measured_weight: f64,
    pub max_weight: f64,
    pub model: &'static str,
    pub status: &'static str,
    pub findings: usize,
    pub commands_precondition_blocked: usize,
    pub output_contracts_discovered: usize,
    pub machine_readable_output_contracts: usize,
    pub output_mode_probes_completed: usize,
    pub output_mode_parse_successes: usize,
    pub output_mode_precondition_blocked: usize,
    pub precondition_blocked_probes: usize,
    pub auth_required_probes: usize,
    pub side_effect_files_created: usize,
    pub side_effect_files_modified: usize,
    pub side_effect_files_deleted: usize,
    pub side_effect_files_total: usize,
    pub side_effect_probe_count: usize,
    pub credential_like_side_effects: usize,
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
}

#[derive(Debug, Clone, Default)]
pub struct SandboxScoreContext {
    pub profile: &'static str,
    pub root: PathBuf,
    pub home: PathBuf,
    pub workdir: PathBuf,
    pub env_policy: &'static str,
}

impl From<&SandboxMetadata> for SandboxScoreContext {
    fn from(metadata: &SandboxMetadata) -> Self {
        Self {
            profile: metadata.profile.label(),
            root: metadata.root.clone(),
            home: metadata.home.clone(),
            workdir: metadata.workdir.clone(),
            env_policy: env_policy_label(metadata.env_policy),
        }
    }
}

pub async fn write_score_artifacts(
    out_dir: &Path,
    target: TargetFingerprint,
    observations: &[ShapeObservation],
    run_context: ScoreRunContext,
) -> Result<ScoreArtifactSummary> {
    let scorecard = scorecard(target, observations, run_context);
    let scorecard_path = write_scorecard(out_dir, &scorecard).await?;
    let report_path = write_report(out_dir, &scorecard).await?;

    Ok(ScoreArtifactSummary {
        scorecard_path,
        report_path,
        total: scorecard.score.total,
        measured_weight: scorecard.score.measured_weight,
        max_weight: scorecard.score.max_weight,
        model: scorecard.score.model,
        status: score_status_label(&scorecard.score.status),
        findings: scorecard.findings.len(),
        commands_precondition_blocked: scorecard.coverage.commands_precondition_blocked,
        output_contracts_discovered: scorecard.coverage.output_contracts_discovered,
        machine_readable_output_contracts: scorecard.coverage.machine_readable_output_contracts,
        output_mode_probes_completed: scorecard.coverage.output_mode_probes_completed,
        output_mode_parse_successes: scorecard.coverage.output_mode_parse_successes,
        output_mode_precondition_blocked: scorecard.coverage.output_mode_precondition_blocked,
        precondition_blocked_probes: scorecard.coverage.precondition_blocked_probes,
        auth_required_probes: scorecard.coverage.auth_required_probes,
        side_effect_files_created: scorecard.coverage.side_effect_files_created,
        side_effect_files_modified: scorecard.coverage.side_effect_files_modified,
        side_effect_files_deleted: scorecard.coverage.side_effect_files_deleted,
        side_effect_files_total: scorecard.coverage.side_effect_files_total,
        side_effect_probe_count: scorecard.coverage.side_effect_probe_count,
        credential_like_side_effects: scorecard.coverage.credential_like_side_effects,
        observed_max_depth: scorecard.coverage.observed_max_depth,
        traversal_profile: scorecard.coverage.traversal_profile,
        max_depth: scorecard.coverage.max_depth,
        max_probes: scorecard.coverage.max_probes,
        min_expected_value: scorecard.coverage.min_expected_value,
        concurrency_limit: scorecard.coverage.concurrency_limit,
        traversal_rounds: scorecard.coverage.traversal_rounds,
        probes_scheduled: scorecard.coverage.probes_scheduled,
        probes_cancelled: scorecard.coverage.probes_cancelled,
        frontier_remaining: scorecard.coverage.frontier_remaining,
        highest_pending_expected_value: scorecard.coverage.highest_pending_expected_value,
        candidates_skipped_by_depth: scorecard.coverage.candidates_skipped_by_depth,
        candidates_skipped_by_convergence: scorecard.coverage.candidates_skipped_by_convergence,
        probes_skipped_by_budget: scorecard.coverage.probes_skipped_by_budget,
        budget_exhausted: scorecard.coverage.budget_exhausted,
        traversal_stop_reason: traversal_stop_reason_label(
            scorecard.coverage.traversal_stop_reason,
        ),
        traversal_complete: scorecard.coverage.traversal_complete,
        sandbox_profile: scorecard.coverage.sandbox_profile,
        sandbox_root: scorecard.coverage.sandbox_root.clone(),
        sandbox_home: scorecard.coverage.sandbox_home.clone(),
        sandbox_workdir: scorecard.coverage.sandbox_workdir.clone(),
        sandbox_env_policy: scorecard.coverage.sandbox_env_policy,
    })
}

async fn write_scorecard(out_dir: &Path, scorecard: &Scorecard) -> Result<PathBuf> {
    let path = out_dir.join("scorecard.json");
    let bytes = serde_json::to_vec_pretty(&scorecard).map_err(CliareError::SerializeScorecard)?;
    fs::write(&path, bytes)
        .await
        .map_err(|source| CliareError::WriteScorecard {
            path: path.clone(),
            source,
        })?;
    Ok(path)
}

async fn write_report(out_dir: &Path, scorecard: &Scorecard) -> Result<PathBuf> {
    let path = out_dir.join("report.md");
    fs::write(&path, render_report(scorecard))
        .await
        .map_err(|source| CliareError::WriteReport {
            path: path.clone(),
            source,
        })?;
    Ok(path)
}

pub fn scorecard(
    target: TargetFingerprint,
    observations: &[ShapeObservation],
    run_context: ScoreRunContext,
) -> Scorecard {
    let binary_name = target_binary_name(&target);
    let claims = ClaimSet::from_observations(&binary_name, observations);
    let metrics = Metrics::from_claims_and_observations(&claims, observations, run_context);

    let subscores = subscores(&metrics);
    let score = total_score(&subscores);
    let findings = findings(&metrics);

    Scorecard {
        schema_version: SCHEMA_VERSION,
        target,
        score,
        subscores,
        coverage: metrics.coverage,
        findings,
        model: ScoreModel {
            name: SCORE_MODEL,
            source: "experimental evidence-only score over measured dimensions",
        },
    }
}

fn subscores(metrics: &Metrics) -> BTreeMap<Dimension, DimensionScore> {
    let mut subscores = BTreeMap::new();

    subscores.insert(
        Dimension::Discovery,
        DimensionScore {
            score: Some(round_score(
                70.0 * metrics.command_recognition_rate()
                    + 30.0 * metrics.coverage.avg_command_confidence,
            )),
            weight: 0.35,
            status: DimensionStatus::Measured,
            rationale: "confirmed command coverage plus average command confidence".to_owned(),
        },
    );
    subscores.insert(
        Dimension::Grammar,
        DimensionScore {
            score: Some(round_score(grammar_score(metrics))),
            weight: 0.20,
            status: DimensionStatus::Measured,
            rationale: "flag discovery, flag confidence, and confirmed-command grammar gaps"
                .to_owned(),
        },
    );
    subscores.insert(
        Dimension::Execution,
        DimensionScore {
            score: Some(round_score(execution_score(metrics))),
            weight: 0.20,
            status: DimensionStatus::Measured,
            rationale: "probe completion without timeouts or spawn failures".to_owned(),
        },
    );
    subscores.insert(
        Dimension::Recovery,
        DimensionScore {
            score: Some(round_score(recovery_score(metrics))),
            weight: 0.15,
            status: DimensionStatus::Measured,
            rationale: "invalid-command, invalid-child, and invalid-flag probes reject cleanly"
                .to_owned(),
        },
    );
    subscores.insert(
        Dimension::Output,
        DimensionScore {
            score: Some(round_score(output_score(metrics))),
            weight: 0.05,
            status: DimensionStatus::Measured,
            rationale: "advertised machine-readable output modes and safe parse probes".to_owned(),
        },
    );
    subscores.insert(
        Dimension::Safety,
        DimensionScore {
            score: Some(round_score(safety_score(metrics))),
            weight: 0.05,
            status: DimensionStatus::Measured,
            rationale: "persistent sandbox filesystem side effects from safe probes".to_owned(),
        },
    );

    subscores
}

fn total_score(subscores: &BTreeMap<Dimension, DimensionScore>) -> ScoreSummary {
    let mut weighted = 0.0;
    let mut measured_weight = 0.0;
    let mut max_weight = 0.0;

    for subscore in subscores.values() {
        max_weight += subscore.weight;
        if let Some(score) = subscore.score {
            weighted += score * subscore.weight;
            measured_weight += subscore.weight;
        }
    }

    let total = if measured_weight > 0.0 {
        weighted / measured_weight
    } else {
        0.0
    };

    ScoreSummary {
        total: round_score(total),
        measured_weight: round_weight(measured_weight),
        max_weight: round_weight(max_weight),
        model: SCORE_MODEL,
        status: ScoreStatus::ExperimentalPartial,
    }
}

fn grammar_score(metrics: &Metrics) -> f64 {
    if metrics.coverage.commands_runtime_confirmed == 0 {
        return 0.0;
    }

    let flag_presence = if metrics.coverage.flags_discovered > 0 {
        1.0
    } else {
        0.0
    };
    let grammar_gap_rate = metrics.grammar_gap_rate();

    30.0 * flag_presence
        + 25.0 * metrics.coverage.avg_flag_confidence
        + 20.0 * metrics.flag_grammar_rate()
        + 25.0 * (1.0 - grammar_gap_rate)
}

fn execution_score(metrics: &Metrics) -> f64 {
    if metrics.coverage.probes_completed == 0 {
        return 0.0;
    }

    let bad = metrics.coverage.probes_timed_out + metrics.coverage.probes_failed_to_spawn;
    100.0 * (1.0 - ratio(bad, metrics.coverage.probes_completed))
}

fn recovery_score(metrics: &Metrics) -> f64 {
    if metrics.invalid_probe_count == 0 {
        return 0.0;
    }

    100.0
        * ratio(
            metrics.invalid_probe_rejections,
            metrics.invalid_probe_count,
        )
}

fn findings(metrics: &Metrics) -> Vec<Finding> {
    let mut findings = Vec::new();

    if metrics.coverage.command_confirmation_rate < 0.50 && metrics.coverage.commands_discovered > 0
    {
        findings.push(Finding {
            id: "finding.discovery.low_runtime_confirmation",
            dimension: Dimension::Discovery,
            severity: Severity::High,
            title: "Most discovered command candidates are not runtime-confirmed",
            detail: format!(
                "{} of {} command candidates were runtime-confirmed; {} were blocked by runtime preconditions.",
                metrics.coverage.commands_runtime_confirmed,
                metrics.coverage.commands_discovered,
                metrics.coverage.commands_precondition_blocked
            ),
            recommendation: "Increase probe budget, improve help consistency, or expose a clearer command catalog.",
        });
    }

    if metrics.grammar_gap_rate() > 0.50 && metrics.coverage.commands_runtime_confirmed > 0 {
        findings.push(Finding {
            id: "finding.grammar.unconfirmed_arity",
            dimension: Dimension::Grammar,
            severity: Severity::Medium,
            title: "Confirmed commands still have unknown grammar details",
            detail: format!(
                "{} grammar gaps remain across {} runtime-confirmed commands.",
                metrics.grammar_gap_count, metrics.coverage.commands_runtime_confirmed
            ),
            recommendation: "Add explicit usage syntax, flag arity, and value-domain diagnostics.",
        });
    }

    if metrics.coverage.probes_timed_out > 0 {
        findings.push(Finding {
            id: "finding.execution.timeouts",
            dimension: Dimension::Execution,
            severity: Severity::High,
            title: "Some probes timed out",
            detail: format!("{} probes timed out.", metrics.coverage.probes_timed_out),
            recommendation: "Ensure help and diagnostic paths are fast and noninteractive under CI=1.",
        });
    }

    if metrics.invalid_probe_count > 0 && recovery_score(metrics) < 80.0 {
        findings.push(Finding {
            id: "finding.recovery.invalid_probe_acceptance",
            dimension: Dimension::Recovery,
            severity: Severity::Medium,
            title: "Invalid probes did not consistently reject",
            detail: format!(
                "{} of {} invalid probes rejected with nonzero exit status.",
                metrics.invalid_probe_rejections, metrics.invalid_probe_count
            ),
            recommendation: "Reject unknown commands and flags with clear diagnostics and nonzero exit codes.",
        });
    }

    if metrics.coverage.machine_readable_output_contracts == 0 {
        findings.push(Finding {
            id: "finding.output.no_machine_readable_mode",
            dimension: Dimension::Output,
            severity: Severity::Medium,
            title: "No machine-readable output mode was discovered",
            detail: "No JSON or YAML output contract was found in runtime help evidence."
                .to_owned(),
            recommendation: "Advertise a stable JSON or YAML output mode in command help.",
        });
    } else if metrics.output_mode_parse_failures() > 0 {
        findings.push(Finding {
            id: "finding.output.unparseable_mode",
            dimension: Dimension::Output,
            severity: Severity::High,
            title: "Some advertised output modes did not parse",
            detail: format!(
                "{} of {} non-blocked output-mode probes parsed successfully.",
                metrics.coverage.output_mode_parse_successes,
                metrics
                    .output_mode_probe_count
                    .saturating_sub(metrics.output_mode_precondition_blocked)
                    .saturating_sub(metrics.output_mode_help_text_probes)
                    .saturating_sub(metrics.output_mode_global_scope_failures)
            ),
            recommendation: "Ensure documented machine-readable modes produce valid output for safe help or diagnostic probes.",
        });
    }

    if metrics.coverage.precondition_blocked_probes > 0 {
        findings.push(Finding {
            id: "finding.precondition.runtime_blocked",
            dimension: Dimension::Discovery,
            severity: Severity::Medium,
            title: "Some probes were blocked by runtime preconditions",
            detail: format!(
                "{} probes were blocked by runtime preconditions across {} command candidates.",
                metrics.coverage.precondition_blocked_probes,
                metrics.coverage.commands_precondition_blocked
            ),
            recommendation: "Document required runtime preconditions separately from command existence, and keep help paths available where practical.",
        });
    }

    if metrics.coverage.side_effect_files_total > 0 {
        findings.push(Finding {
            id: "finding.safety.safe_probe_side_effects",
            dimension: Dimension::Safety,
            severity: Severity::High,
            title: "Safe probes left persistent filesystem side effects",
            detail: format!(
                "{} file changes were observed across {} probes.",
                metrics.coverage.side_effect_files_total,
                metrics.coverage.side_effect_probe_count
            ),
            recommendation: "Keep help, version, and diagnostic paths read-only, or clearly document unavoidable cache/config writes.",
        });
    }

    if metrics.coverage.credential_like_side_effects > 0 {
        findings.push(Finding {
            id: "finding.safety.credential_like_side_effects",
            dimension: Dimension::Safety,
            severity: Severity::High,
            title: "Side-effect paths looked credential-related",
            detail: format!(
                "{} side-effect paths contained credential-like terms.",
                metrics.coverage.credential_like_side_effects
            ),
            recommendation: "Do not create token, secret, credential, or key material during discovery probes.",
        });
    }

    findings
}

fn render_report(scorecard: &Scorecard) -> String {
    let mut report = String::new();

    report.push_str("# CLIARE Report\n\n");
    report.push_str("This report is generated from runtime evidence. Score v0 is experimental and partial: unmeasured dimensions are shown but excluded from the partial total.\n\n");
    report.push_str("## Summary\n\n");
    report.push_str(&format!(
        "- Target: `{}`\n",
        scorecard.target.requested.display()
    ));
    report.push_str(&format!(
        "- Resolved binary: `{}`\n",
        scorecard.target.resolved.display()
    ));
    report.push_str(&format!(
        "- Binary SHA-256: `{}`\n",
        scorecard.target.binary_sha256
    ));
    report.push_str(&format!(
        "- Score: `{:.1}` / 100 (`{}`)\n",
        scorecard.score.total,
        score_status_label(&scorecard.score.status)
    ));
    report.push_str(&format!(
        "- Measured weight: `{:.1}` of `{:.1}`\n",
        scorecard.score.measured_weight, scorecard.score.max_weight
    ));
    report.push_str(&format!("- Model: `{}`\n\n", scorecard.model.name));

    report.push_str("## Runtime Isolation\n\n");
    report.push_str(&format!(
        "- Sandbox profile: `{}`\n",
        scorecard.coverage.sandbox_profile
    ));
    report.push_str(&format!(
        "- Environment policy: `{}`\n",
        scorecard.coverage.sandbox_env_policy
    ));
    report.push_str(&format!(
        "- Sandbox root: `{}`\n",
        scorecard.coverage.sandbox_root.display()
    ));
    report.push_str(&format!(
        "- Sandbox home: `{}`\n",
        scorecard.coverage.sandbox_home.display()
    ));
    report.push_str(&format!(
        "- Sandbox workdir: `{}`\n\n",
        scorecard.coverage.sandbox_workdir.display()
    ));

    report.push_str("## Subscores\n\n");
    report.push_str("| Dimension | Score | Weight | Status | Rationale |\n");
    report.push_str("| --- | ---: | ---: | --- | --- |\n");
    for (dimension, subscore) in &scorecard.subscores {
        report.push_str(&format!(
            "| {} | {} | {:.2} | {} | {} |\n",
            dimension_label(*dimension),
            score_label(subscore.score),
            subscore.weight,
            dimension_status_label(&subscore.status),
            escape_table_cell(&subscore.rationale)
        ));
    }
    report.push('\n');

    report.push_str("## Coverage\n\n");
    report.push_str(&format!(
        "- Commands discovered: `{}`\n",
        scorecard.coverage.commands_discovered
    ));
    report.push_str(&format!(
        "- Commands runtime-confirmed: `{}`\n",
        scorecard.coverage.commands_runtime_confirmed
    ));
    report.push_str(&format!(
        "- Commands precondition-blocked: `{}`\n",
        scorecard.coverage.commands_precondition_blocked
    ));
    report.push_str(&format!(
        "- Command confirmation rate: `{:.1}%`\n",
        scorecard.coverage.command_confirmation_rate * 100.0
    ));
    report.push_str(&format!(
        "- Flags discovered: `{}`\n",
        scorecard.coverage.flags_discovered
    ));
    report.push_str(&format!(
        "- Output contracts discovered: `{}`\n",
        scorecard.coverage.output_contracts_discovered
    ));
    report.push_str(&format!(
        "- Machine-readable output contracts: `{}`\n",
        scorecard.coverage.machine_readable_output_contracts
    ));
    report.push_str(&format!(
        "- Output-mode probes completed: `{}`\n",
        scorecard.coverage.output_mode_probes_completed
    ));
    report.push_str(&format!(
        "- Output-mode parse successes: `{}`\n",
        scorecard.coverage.output_mode_parse_successes
    ));
    report.push_str(&format!(
        "- Output-mode precondition-blocked: `{}`\n",
        scorecard.coverage.output_mode_precondition_blocked
    ));
    report.push_str(&format!(
        "- Side-effect file changes: `{}`\n",
        scorecard.coverage.side_effect_files_total
    ));
    report.push_str(&format!(
        "- Side-effect probes: `{}`\n",
        scorecard.coverage.side_effect_probe_count
    ));
    report.push_str(&format!(
        "- Side-effect files created: `{}`\n",
        scorecard.coverage.side_effect_files_created
    ));
    report.push_str(&format!(
        "- Side-effect files modified: `{}`\n",
        scorecard.coverage.side_effect_files_modified
    ));
    report.push_str(&format!(
        "- Side-effect files deleted: `{}`\n",
        scorecard.coverage.side_effect_files_deleted
    ));
    report.push_str(&format!(
        "- Credential-like side-effect paths: `{}`\n",
        scorecard.coverage.credential_like_side_effects
    ));
    report.push_str(&format!(
        "- Average command confidence: `{:.3}`\n",
        scorecard.coverage.avg_command_confidence
    ));
    report.push_str(&format!(
        "- Average flag confidence: `{:.3}`\n",
        scorecard.coverage.avg_flag_confidence
    ));
    report.push_str(&format!(
        "- Observed max command depth: `{}`\n",
        scorecard.coverage.observed_max_depth
    ));
    report.push_str(&format!(
        "- Traversal profile: `{}`\n",
        scorecard.coverage.traversal_profile
    ));
    report.push_str(&format!(
        "- Depth budget: `{}`\n",
        scorecard.coverage.max_depth
    ));
    report.push_str(&format!(
        "- Probe budget: `{}`\n",
        scorecard.coverage.max_probes
    ));
    report.push_str(&format!(
        "- Minimum expected probe value: `{}`\n",
        scorecard.coverage.min_expected_value
    ));
    report.push_str(&format!(
        "- Concurrency limit: `{}`\n",
        scorecard.coverage.concurrency_limit
    ));
    report.push_str(&format!(
        "- Scheduler rounds: `{}`\n",
        scorecard.coverage.traversal_rounds
    ));
    report.push_str(&format!(
        "- Probes scheduled: `{}`\n",
        scorecard.coverage.probes_scheduled
    ));
    report.push_str(&format!(
        "- Probes completed: `{}`\n",
        scorecard.coverage.probes_completed
    ));
    report.push_str(&format!(
        "- Probes cancelled: `{}`\n",
        scorecard.coverage.probes_cancelled
    ));
    report.push_str(&format!(
        "- Probe timeouts: `{}`\n",
        scorecard.coverage.probes_timed_out
    ));
    report.push_str(&format!(
        "- Probe spawn failures: `{}`\n\n",
        scorecard.coverage.probes_failed_to_spawn
    ));
    report.push_str(&format!(
        "- Precondition-blocked probes: `{}`\n",
        scorecard.coverage.precondition_blocked_probes
    ));
    report.push_str(&format!(
        "- Auth-required probes: `{}`\n\n",
        scorecard.coverage.auth_required_probes
    ));
    report.push_str(&format!(
        "- Frontier remaining: `{}`\n",
        scorecard.coverage.frontier_remaining
    ));
    report.push_str(&format!(
        "- Highest pending expected value: `{}`\n",
        scorecard
            .coverage
            .highest_pending_expected_value
            .map_or_else(|| "none".to_owned(), |value| value.to_string())
    ));
    report.push_str(&format!(
        "- Candidates skipped by depth: `{}`\n",
        scorecard.coverage.candidates_skipped_by_depth
    ));
    report.push_str(&format!(
        "- Candidates skipped by convergence: `{}`\n",
        scorecard.coverage.candidates_skipped_by_convergence
    ));
    report.push_str(&format!(
        "- Probes skipped by budget: `{}`\n",
        scorecard.coverage.probes_skipped_by_budget
    ));
    report.push_str(&format!(
        "- Budget exhausted: `{}`\n\n",
        scorecard.coverage.budget_exhausted
    ));
    report.push_str(&format!(
        "- Traversal stop reason: `{}`\n",
        traversal_stop_reason_label(scorecard.coverage.traversal_stop_reason)
    ));
    report.push_str(&format!(
        "- Traversal complete: `{}`\n\n",
        scorecard.coverage.traversal_complete
    ));

    report.push_str("## Findings\n\n");
    if scorecard.findings.is_empty() {
        report.push_str("No findings for measured dimensions.\n");
    } else {
        for finding in &scorecard.findings {
            report.push_str(&format!(
                "### {}: {}\n\n",
                severity_label(&finding.severity),
                finding.title
            ));
            report.push_str(&format!("- ID: `{}`\n", finding.id));
            report.push_str(&format!(
                "- Dimension: `{}`\n",
                dimension_label(finding.dimension)
            ));
            report.push_str(&format!("- Detail: {}\n", finding.detail));
            report.push_str(&format!("- Recommendation: {}\n\n", finding.recommendation));
        }
    }

    report
}

fn score_label(score: Option<f64>) -> String {
    score.map_or_else(|| "not measured".to_owned(), |score| format!("{score:.1}"))
}

fn score_status_label(status: &ScoreStatus) -> &'static str {
    match status {
        ScoreStatus::ExperimentalPartial => "experimental partial",
    }
}

fn dimension_label(dimension: Dimension) -> &'static str {
    match dimension {
        Dimension::Discovery => "discovery",
        Dimension::Grammar => "grammar",
        Dimension::Execution => "execution",
        Dimension::Output => "output",
        Dimension::Safety => "safety",
        Dimension::Recovery => "recovery",
    }
}

fn dimension_status_label(status: &DimensionStatus) -> &'static str {
    match status {
        DimensionStatus::Measured => "measured",
        DimensionStatus::NotMeasured => "not measured",
    }
}

fn severity_label(severity: &Severity) -> &'static str {
    match severity {
        Severity::Low => "Low",
        Severity::Medium => "Medium",
        Severity::High => "High",
    }
}

fn traversal_stop_reason_label(reason: TraversalStopReason) -> &'static str {
    match reason {
        TraversalStopReason::FrontierExhausted => "frontier_exhausted",
        TraversalStopReason::Converged => "converged",
        TraversalStopReason::DepthBudgetExhausted => "depth_budget_exhausted",
        TraversalStopReason::ProbeBudgetExhausted => "probe_budget_exhausted",
    }
}

fn env_policy_label(policy: crate::sandbox::EnvPolicy) -> &'static str {
    match policy {
        crate::sandbox::EnvPolicy::ClearedWithAllowlist => "cleared_with_allowlist",
    }
}

fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

#[derive(Debug)]
struct Metrics {
    coverage: Coverage,
    grammar_gap_count: usize,
    flags_with_known_grammar: usize,
    machine_readable_output_contracts: usize,
    output_mode_scored_contracts: usize,
    output_mode_probe_count: usize,
    output_mode_parse_successes: usize,
    output_mode_precondition_blocked: usize,
    output_mode_help_text_probes: usize,
    output_mode_global_scope_failures: usize,
    side_effect_files_total: usize,
    side_effect_probe_count: usize,
    credential_like_side_effects: usize,
    invalid_probe_count: usize,
    invalid_probe_rejections: usize,
}

impl Metrics {
    fn from_claims_and_observations(
        claims: &ClaimSet,
        observations: &[ShapeObservation],
        run_context: ScoreRunContext,
    ) -> Self {
        let commands = claims.commands().collect::<Vec<_>>();
        let flags = claims.flags().collect::<Vec<_>>();
        let outputs = claims.output_contracts().collect::<Vec<_>>();
        let commands_discovered = commands.len();
        let commands_runtime_confirmed = commands
            .iter()
            .filter(|command| command.runtime_confirmed())
            .count();
        let commands_precondition_blocked = commands
            .iter()
            .filter(|command| command.precondition_blocked())
            .count();
        let avg_command_confidence = average(commands.iter().map(|command| command.confidence()));
        let avg_flag_confidence = average(flags.iter().map(|flag| flag.confidence()));
        let grammar_gap_count = commands
            .iter()
            .filter(|command| command.runtime_confirmed())
            .map(|command| grammar_gaps_for(command))
            .sum();
        let flags_with_known_grammar = flags.iter().filter(|flag| flag_grammar_known(flag)).count();
        let output_contracts_discovered = outputs.len();
        let machine_readable_output_contracts = outputs
            .iter()
            .filter(|contract| machine_readable_output_contract(contract))
            .count();
        let output_mode_scored_contracts = outputs
            .iter()
            .filter(|contract| {
                machine_readable_output_contract(contract)
                    && !contract.precondition_blocked()
                    && contract.observed_kind() != Some(ObservedOutputKind::HelpText)
                    && (!contract.scope().is_global_only() || contract.parse_success())
            })
            .count();
        let output_mode_probe_count = outputs.iter().filter(|contract| contract.probed()).count();
        let output_mode_parse_successes = outputs
            .iter()
            .filter(|contract| contract.parse_success())
            .count();
        let output_mode_precondition_blocked = outputs
            .iter()
            .filter(|contract| contract.precondition_blocked())
            .count();
        let output_mode_help_text_probes = outputs
            .iter()
            .filter(|contract| contract.observed_kind() == Some(ObservedOutputKind::HelpText))
            .count();
        let output_mode_global_scope_failures = outputs
            .iter()
            .filter(|contract| {
                contract.probed()
                    && !contract.parse_success()
                    && !contract.precondition_blocked()
                    && contract.observed_kind() != Some(ObservedOutputKind::HelpText)
                    && contract.scope() == OutputContractScope::GlobalFlag
            })
            .count();

        let process_metrics = ProcessMetrics::from_observations(observations);
        let probes_skipped_by_budget =
            if run_context.max_probes > 0 && observations.len() >= run_context.max_probes {
                run_context.frontier_remaining
            } else {
                0
            };
        let traversal_stop_reason = traversal_stop_reason(
            commands_discovered,
            probes_skipped_by_budget,
            run_context.candidates_skipped_by_depth,
            run_context.candidates_skipped_by_convergence,
        );

        Self {
            coverage: Coverage {
                sandbox_profile: run_context.sandbox.profile,
                sandbox_root: run_context.sandbox.root,
                sandbox_home: run_context.sandbox.home,
                sandbox_workdir: run_context.sandbox.workdir,
                sandbox_env_policy: run_context.sandbox.env_policy,
                commands_discovered,
                commands_runtime_confirmed,
                commands_precondition_blocked,
                command_confirmation_rate: ratio(commands_runtime_confirmed, commands_discovered),
                flags_discovered: flags.len(),
                output_contracts_discovered,
                machine_readable_output_contracts,
                output_mode_probes_completed: output_mode_probe_count,
                output_mode_parse_successes,
                output_mode_precondition_blocked,
                output_mode_help_text_probes,
                side_effect_files_created: process_metrics.side_effect_files_created,
                side_effect_files_modified: process_metrics.side_effect_files_modified,
                side_effect_files_deleted: process_metrics.side_effect_files_deleted,
                side_effect_files_total: process_metrics.side_effect_files_total,
                side_effect_probe_count: process_metrics.side_effect_probe_count,
                credential_like_side_effects: process_metrics.credential_like_side_effects,
                avg_command_confidence,
                avg_flag_confidence,
                observed_max_depth: observed_max_depth(&commands),
                traversal_profile: run_context.traversal_profile,
                max_depth: run_context.max_depth,
                max_probes: run_context.max_probes,
                min_expected_value: run_context.min_expected_value,
                concurrency_limit: run_context.concurrency_limit,
                traversal_rounds: run_context.traversal_rounds,
                probes_scheduled: run_context.probes_scheduled,
                probes_completed: observations.len(),
                probes_cancelled: run_context.probes_cancelled,
                probes_timed_out: process_metrics.timed_out,
                probes_failed_to_spawn: process_metrics.failed_to_spawn,
                precondition_blocked_probes: process_metrics.precondition_blocked,
                auth_required_probes: process_metrics.auth_required,
                frontier_remaining: run_context.frontier_remaining,
                highest_pending_expected_value: run_context.highest_pending_expected_value,
                candidates_skipped_by_depth: run_context.candidates_skipped_by_depth,
                candidates_skipped_by_convergence: run_context.candidates_skipped_by_convergence,
                probes_skipped_by_budget,
                budget_exhausted: probes_skipped_by_budget > 0,
                traversal_stop_reason,
                traversal_complete: matches!(
                    traversal_stop_reason,
                    TraversalStopReason::Converged | TraversalStopReason::FrontierExhausted
                ),
            },
            grammar_gap_count,
            flags_with_known_grammar,
            machine_readable_output_contracts,
            output_mode_scored_contracts,
            output_mode_probe_count,
            output_mode_parse_successes,
            output_mode_precondition_blocked,
            output_mode_help_text_probes,
            output_mode_global_scope_failures,
            side_effect_files_total: process_metrics.side_effect_files_total,
            side_effect_probe_count: process_metrics.side_effect_probe_count,
            credential_like_side_effects: process_metrics.credential_like_side_effects,
            invalid_probe_count: process_metrics.invalid_probe_count,
            invalid_probe_rejections: process_metrics.invalid_probe_rejections,
        }
    }

    fn grammar_gap_rate(&self) -> f64 {
        let possible = self.coverage.commands_runtime_confirmed.saturating_mul(2);
        ratio(self.grammar_gap_count, possible)
    }

    fn flag_grammar_rate(&self) -> f64 {
        ratio(
            self.flags_with_known_grammar,
            self.coverage.flags_discovered,
        )
    }

    fn command_recognition_rate(&self) -> f64 {
        ratio(
            self.coverage.commands_runtime_confirmed + self.coverage.commands_precondition_blocked,
            self.coverage.commands_discovered,
        )
    }

    fn output_mode_parse_failures(&self) -> usize {
        self.output_mode_probe_count
            .saturating_sub(self.output_mode_parse_successes)
            .saturating_sub(self.output_mode_precondition_blocked)
            .saturating_sub(self.output_mode_help_text_probes)
            .saturating_sub(self.output_mode_global_scope_failures)
    }
}

fn output_score(metrics: &Metrics) -> f64 {
    if metrics.machine_readable_output_contracts == 0 {
        return 0.0;
    }

    let advertised_score = 40.0;
    let non_blocked_probe_count = metrics
        .output_mode_probe_count
        .saturating_sub(metrics.output_mode_precondition_blocked)
        .saturating_sub(metrics.output_mode_help_text_probes)
        .saturating_sub(metrics.output_mode_global_scope_failures);
    let denominator = metrics
        .output_mode_scored_contracts
        .max(non_blocked_probe_count);
    advertised_score + 60.0 * ratio(metrics.output_mode_parse_successes, denominator)
}

fn safety_score(metrics: &Metrics) -> f64 {
    if metrics.coverage.probes_completed == 0 {
        return 0.0;
    }

    let changed_probe_penalty = 45.0
        * ratio(
            metrics.side_effect_probe_count,
            metrics.coverage.probes_completed,
        );
    let file_penalty = (metrics.side_effect_files_total as f64 * 8.0).min(35.0);
    let credential_penalty = (metrics.credential_like_side_effects as f64 * 20.0).min(40.0);

    (100.0 - changed_probe_penalty - file_penalty - credential_penalty).max(0.0)
}

fn machine_readable_output_contract(contract: &OutputContractClaim) -> bool {
    matches!(
        contract.mode(),
        crate::output::OutputMode::Json | crate::output::OutputMode::Yaml
    )
}

fn traversal_stop_reason(
    commands_discovered: usize,
    probes_skipped_by_budget: usize,
    candidates_skipped_by_depth: usize,
    candidates_skipped_by_convergence: usize,
) -> TraversalStopReason {
    if probes_skipped_by_budget > 0 {
        TraversalStopReason::ProbeBudgetExhausted
    } else if candidates_skipped_by_depth > 0 {
        TraversalStopReason::DepthBudgetExhausted
    } else if commands_discovered > 0 || candidates_skipped_by_convergence > 0 {
        TraversalStopReason::Converged
    } else {
        TraversalStopReason::FrontierExhausted
    }
}

fn observed_max_depth(commands: &[&CommandClaim]) -> usize {
    commands
        .iter()
        .map(|command| command.path().len())
        .max()
        .unwrap_or(0)
}

#[derive(Debug, Default)]
struct ProcessMetrics {
    timed_out: usize,
    failed_to_spawn: usize,
    side_effect_files_created: usize,
    side_effect_files_modified: usize,
    side_effect_files_deleted: usize,
    side_effect_files_total: usize,
    side_effect_probe_count: usize,
    credential_like_side_effects: usize,
    precondition_blocked: usize,
    auth_required: usize,
    invalid_probe_count: usize,
    invalid_probe_rejections: usize,
}

impl ProcessMetrics {
    fn from_observations(observations: &[ShapeObservation]) -> Self {
        let mut metrics = Self::default();

        for observation in observations {
            match &observation.process.status {
                ProcessStatus::TimedOut => metrics.timed_out += 1,
                ProcessStatus::SpawnFailed { .. } => metrics.failed_to_spawn += 1,
                ProcessStatus::Exited { .. } => {}
            }

            let side_effects = &observation.process.side_effects;
            metrics.side_effect_files_created += side_effects.created;
            metrics.side_effect_files_modified += side_effects.modified;
            metrics.side_effect_files_deleted += side_effects.deleted;
            metrics.side_effect_files_total += side_effects.total;
            if side_effects.total > 0 {
                metrics.side_effect_probe_count += 1;
            }
            metrics.credential_like_side_effects += side_effects
                .changes
                .iter()
                .filter(|change| credential_like_path(&change.path))
                .count();

            let precondition = precondition::classify_process(
                &observation.process.status,
                observation.process.stdout.text.as_deref(),
                observation.process.stderr.text.as_deref(),
            );
            if let Some(precondition) = precondition {
                metrics.precondition_blocked += 1;
                if precondition == PreconditionKind::AuthRequired {
                    metrics.auth_required += 1;
                }
            }

            if matches!(
                observation.intent,
                ProbeIntent::InvalidCommand | ProbeIntent::InvalidChild | ProbeIntent::InvalidFlag
            ) && precondition.is_none()
            {
                metrics.invalid_probe_count += 1;
                if exited_nonzero(&observation.process.status) {
                    metrics.invalid_probe_rejections += 1;
                }
            }
        }

        metrics
    }
}

fn credential_like_path(path: &Path) -> bool {
    let text = path.display().to_string().to_ascii_lowercase();
    ["token", "secret", "credential", "credentials", "key"]
        .iter()
        .any(|needle| text.contains(needle))
}

fn grammar_gaps_for(command: &CommandClaim) -> usize {
    let mut gaps = 2_usize;
    if command.invalid_flag_rejected() {
        gaps = gaps.saturating_sub(1);
    }
    if command.usage_observed()
        || !command.has_child_candidates()
        || command.invalid_child_rejected()
    {
        gaps = gaps.saturating_sub(1);
    }
    gaps
}

fn flag_grammar_known(flag: &FlagClaim) -> bool {
    matches!(flag.value_kind(), FlagValueKind::Boolean) || flag.value_name().is_some()
}

fn exited_nonzero(status: &ProcessStatus) -> bool {
    matches!(status, ProcessStatus::Exited { code: Some(code) } if *code != 0)
}

fn average(values: impl Iterator<Item = f64>) -> f64 {
    let mut sum = 0.0;
    let mut count = 0_usize;

    for value in values {
        sum += value;
        count += 1;
    }

    if count == 0 { 0.0 } else { sum / count as f64 }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn round_score(score: f64) -> f64 {
    (score * 10.0).round() / 10.0
}

fn round_weight(weight: f64) -> f64 {
    (weight * 100.0).round() / 100.0
}

fn target_binary_name(target: &TargetFingerprint) -> String {
    target
        .resolved
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{Dimension, SandboxScoreContext, ScoreRunContext, render_report, scorecard};
    use crate::evidence::{ProbeIntent, ProcessCompleted, ProcessStatus};
    use crate::fingerprint::TargetFingerprint;
    use crate::observation::ShapeObservation;
    use crate::process::OutputCapture;

    #[test]
    fn runtime_confirmation_improves_discovery_score() {
        let target = target();
        let weak = vec![observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Commands:\n  measure  Run probes\n",
            Some(0),
        )];
        let strong = vec![
            observation(
                "e_000003",
                ProbeIntent::Help,
                vec![],
                "Commands:\n  measure  Run probes\n",
                Some(0),
            ),
            observation(
                "e_000005",
                ProbeIntent::Help,
                vec!["measure".to_owned()],
                "Usage: cliare measure <TARGET>\n\nOptions:\n  --out <DIR>  Output directory\n",
                Some(0),
            ),
        ];

        let weak_score = scorecard(target.clone(), &weak, ScoreRunContext::default());
        let strong_score = scorecard(target, &strong, ScoreRunContext::default());

        assert!(
            dimension_score(&strong_score, Dimension::Discovery)
                > dimension_score(&weak_score, Dimension::Discovery)
        );
    }

    #[test]
    fn invalid_flag_rejection_improves_recovery_score() {
        let target = target();
        let observations = vec![
            observation(
                "e_000003",
                ProbeIntent::Help,
                vec!["measure".to_owned()],
                "Usage: cliare measure <TARGET>\n\nOptions:\n  --out <DIR>  Output directory\n",
                Some(0),
            ),
            observation(
                "e_000005",
                ProbeIntent::InvalidFlag,
                vec!["measure".to_owned()],
                "error: unexpected argument",
                Some(2),
            ),
        ];

        let scorecard = scorecard(target, &observations, ScoreRunContext::default());

        assert_eq!(dimension_score(&scorecard, Dimension::Recovery), 100.0);
    }

    #[test]
    fn auth_blocked_probes_are_reported_as_preconditions_not_recovery_success() {
        let observations = vec![
            observation(
                "e_000003",
                ProbeIntent::Help,
                vec![],
                "Commands:\n  model  Track AI model identity\n",
                Some(0),
            ),
            observation(
                "e_000005",
                ProbeIntent::Help,
                vec!["model".to_owned()],
                "error: rote requires login\n\nrun rote login",
                Some(77),
            ),
            observation(
                "e_000007",
                ProbeIntent::InvalidFlag,
                vec!["model".to_owned()],
                "error: rote requires login\n\nrun rote login",
                Some(77),
            ),
        ];

        let scorecard = scorecard(target(), &observations, ScoreRunContext::default());

        assert_eq!(scorecard.coverage.commands_runtime_confirmed, 0);
        assert_eq!(scorecard.coverage.commands_precondition_blocked, 1);
        assert_eq!(scorecard.coverage.precondition_blocked_probes, 2);
        assert_eq!(scorecard.coverage.auth_required_probes, 2);
        assert_eq!(dimension_score(&scorecard, Dimension::Recovery), 0.0);
        assert!(scorecard.findings.iter().any(|finding| {
            finding.id == "finding.precondition.runtime_blocked"
                && finding.title == "Some probes were blocked by runtime preconditions"
        }));

        let report = render_report(&scorecard);
        assert!(report.contains("- Commands precondition-blocked: `1`"));
        assert!(report.contains("- Precondition-blocked probes: `2`"));
        assert!(report.contains("- Auth-required probes: `2`"));
    }

    #[test]
    fn report_renders_scorecard_summary_and_unmeasured_dimensions() {
        let scorecard = scorecard(
            target(),
            &[observation(
                "e_000003",
                ProbeIntent::Help,
                vec![],
                "Commands:\n  measure  Run probes\n",
                Some(0),
            )],
            ScoreRunContext {
                traversal_profile: "standard",
                max_depth: 5,
                max_probes: 256,
                min_expected_value: 150,
                concurrency_limit: 4,
                traversal_rounds: 1,
                probes_scheduled: 1,
                probes_cancelled: 0,
                frontier_remaining: 0,
                highest_pending_expected_value: None,
                candidates_skipped_by_depth: 0,
                candidates_skipped_by_convergence: 0,
                sandbox: test_sandbox(),
            },
        );

        let report = render_report(&scorecard);

        assert!(report.contains("# CLIARE Report"));
        assert!(report.contains("| output | 0.0 | 0.05 | measured |"));
        assert!(report.contains("experimental partial"));
        assert!(report.contains("- Output contracts discovered: `0`"));
        assert!(report.contains("- Traversal profile: `standard`"));
        assert!(report.contains("- Depth budget: `5`"));
        assert!(report.contains("- Minimum expected probe value: `150`"));
        assert!(report.contains("- Concurrency limit: `4`"));
        assert!(report.contains("- Scheduler rounds: `1`"));
        assert!(report.contains("- Probes scheduled: `1`"));
        assert!(report.contains("- Probes cancelled: `0`"));
        assert!(report.contains("- Sandbox profile: `isolated`"));
        assert!(report.contains("- Environment policy: `cleared_with_allowlist`"));
        assert!(report.contains("- Budget exhausted: `false`"));
        assert!(report.contains("- Traversal stop reason: `converged`"));
        assert!(report.contains("- Traversal complete: `true`"));
    }

    #[test]
    fn scorecard_reports_budget_pressure_without_lowering_score() {
        let observations = vec![
            observation(
                "e_000003",
                ProbeIntent::Help,
                vec![],
                "Commands:\n  alpha  First level\n",
                Some(0),
            ),
            observation(
                "e_000004",
                ProbeIntent::Help,
                vec!["alpha".to_owned()],
                "Commands:\n  beta  Second level\n",
                Some(0),
            ),
        ];

        let scorecard = scorecard(
            target(),
            &observations,
            ScoreRunContext {
                traversal_profile: "quick",
                max_depth: 1,
                max_probes: 2,
                min_expected_value: 300,
                concurrency_limit: 2,
                traversal_rounds: 1,
                probes_scheduled: 2,
                probes_cancelled: 0,
                frontier_remaining: 3,
                highest_pending_expected_value: Some(400),
                candidates_skipped_by_depth: 1,
                candidates_skipped_by_convergence: 2,
                ..ScoreRunContext::default()
            },
        );

        assert_eq!(scorecard.coverage.observed_max_depth, 2);
        assert_eq!(scorecard.coverage.traversal_profile, "quick");
        assert_eq!(scorecard.coverage.max_depth, 1);
        assert_eq!(scorecard.coverage.max_probes, 2);
        assert_eq!(scorecard.coverage.min_expected_value, 300);
        assert_eq!(scorecard.coverage.concurrency_limit, 2);
        assert_eq!(scorecard.coverage.traversal_rounds, 1);
        assert_eq!(scorecard.coverage.probes_scheduled, 2);
        assert_eq!(scorecard.coverage.probes_cancelled, 0);
        assert_eq!(scorecard.coverage.frontier_remaining, 3);
        assert_eq!(scorecard.coverage.highest_pending_expected_value, Some(400));
        assert_eq!(scorecard.coverage.candidates_skipped_by_depth, 1);
        assert_eq!(scorecard.coverage.candidates_skipped_by_convergence, 2);
        assert_eq!(scorecard.coverage.probes_skipped_by_budget, 3);
        assert!(scorecard.coverage.budget_exhausted);
        assert_eq!(
            scorecard.coverage.traversal_stop_reason,
            super::TraversalStopReason::ProbeBudgetExhausted
        );
        assert!(!scorecard.coverage.traversal_complete);
    }

    #[test]
    fn scorecard_classifies_depth_budget_stop_before_convergence() {
        let scorecard = scorecard(
            target(),
            &[observation(
                "e_000003",
                ProbeIntent::Help,
                vec![],
                "Commands:\n  alpha  First level\n",
                Some(0),
            )],
            ScoreRunContext {
                traversal_profile: "quick",
                max_depth: 1,
                max_probes: 64,
                min_expected_value: 300,
                frontier_remaining: 0,
                highest_pending_expected_value: None,
                candidates_skipped_by_depth: 2,
                candidates_skipped_by_convergence: 0,
                ..ScoreRunContext::default()
            },
        );

        assert_eq!(
            scorecard.coverage.traversal_stop_reason,
            super::TraversalStopReason::DepthBudgetExhausted
        );
        assert!(!scorecard.coverage.traversal_complete);
    }

    #[test]
    fn scorecard_classifies_empty_frontier_without_claims() {
        let scorecard = scorecard(
            target(),
            &[],
            ScoreRunContext {
                traversal_profile: "standard",
                max_depth: 5,
                max_probes: 256,
                min_expected_value: 150,
                frontier_remaining: 0,
                highest_pending_expected_value: None,
                candidates_skipped_by_depth: 0,
                candidates_skipped_by_convergence: 0,
                ..ScoreRunContext::default()
            },
        );

        assert_eq!(
            scorecard.coverage.traversal_stop_reason,
            super::TraversalStopReason::FrontierExhausted
        );
        assert!(scorecard.coverage.traversal_complete);
    }

    fn dimension_score(scorecard: &super::Scorecard, dimension: Dimension) -> f64 {
        scorecard.subscores[&dimension]
            .score
            .expect("dimension is measured")
    }

    fn target() -> TargetFingerprint {
        TargetFingerprint {
            requested: "cliare".into(),
            resolved: "/tmp/cliare".into(),
            binary_sha256: "abc".to_owned(),
            size_bytes: 1,
        }
    }

    fn test_sandbox() -> SandboxScoreContext {
        SandboxScoreContext {
            profile: "isolated",
            root: "/tmp/cliare/sandbox".into(),
            home: "/tmp/cliare/sandbox/home".into(),
            workdir: "/tmp/cliare/sandbox/cwd".into(),
            env_policy: "cleared_with_allowlist",
        }
    }

    fn observation(
        evidence_id: &str,
        intent: ProbeIntent,
        path: Vec<String>,
        stdout: &str,
        exit_code: Option<i32>,
    ) -> ShapeObservation {
        ShapeObservation {
            evidence_id: evidence_id.to_owned(),
            intent,
            path,
            process: ProcessCompleted {
                probe_id: "p_000001".to_owned(),
                argv: vec!["cliare".to_owned(), "--help".to_owned()],
                status: ProcessStatus::Exited { code: exit_code },
                duration_ms: 1,
                stdout: output(stdout),
                stderr: output(""),
                side_effects: crate::sandbox::SideEffectSummary::default(),
            },
        }
    }

    fn output(text: &str) -> OutputCapture {
        OutputCapture {
            sha256: "unused".to_owned(),
            bytes: text.len(),
            retained_bytes: text.len(),
            truncated: false,
            text: Some(text.to_owned()),
        }
    }
}
