use std::path::{Path, PathBuf};

use cliare_core::artifacts::{REPORT_MD, SCORECARD_JSON, write_atomic};
use cliare_core::error::{CliareError, Result};
use cliare_runtime::fingerprint::TargetFingerprint;
use cliare_shape::observation::ShapeObservation;

use super::calculator::scorecard;
use super::labels::{score_status_label, traversal_stop_reason_label};
use super::model::{ScoreArtifactSummary, ScoreRunContext, Scorecard};
use super::report;

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
        model: scorecard.score.model.clone(),
        status: score_status_label(&scorecard.score.status),
        findings: scorecard.findings.len(),
        commands_precondition_blocked: scorecard.coverage.commands_precondition_blocked,
        help_text_probes: scorecard.coverage.help_text_probes,
        help_text_probes_with_shape: scorecard.coverage.help_text_probes_with_shape,
        help_text_probes_without_shape: scorecard.coverage.help_text_probes_without_shape,
        help_text_probes_not_recognized: scorecard.coverage.help_text_probes_not_recognized,
        parser_extraction_rate: scorecard.coverage.parser_extraction_rate,
        output_contracts_discovered: scorecard.coverage.output_contracts_discovered,
        machine_readable_output_contracts: scorecard.coverage.machine_readable_output_contracts,
        output_mode_probes_completed: scorecard.coverage.output_mode_probes_completed,
        output_mode_parse_successes: scorecard.coverage.output_mode_parse_successes,
        output_mode_precondition_blocked: scorecard.coverage.output_mode_precondition_blocked,
        precondition_blocked_probes: scorecard.coverage.precondition_blocked_probes,
        auth_required_probes: scorecard.coverage.auth_required_probes,
        local_context_required_probes: scorecard.coverage.local_context_required_probes,
        fixture_required_probes: scorecard.coverage.fixture_required_probes,
        actionable_precondition_probes: scorecard.coverage.actionable_precondition_probes,
        precondition_recovery_rate: scorecard.coverage.precondition_recovery_rate,
        side_effect_files_created: scorecard.coverage.side_effect_files_created,
        side_effect_files_modified: scorecard.coverage.side_effect_files_modified,
        side_effect_files_deleted: scorecard.coverage.side_effect_files_deleted,
        side_effect_files_total: scorecard.coverage.side_effect_files_total,
        side_effect_probe_count: scorecard.coverage.side_effect_probe_count,
        credential_like_side_effects: scorecard.coverage.credential_like_side_effects,
        side_effect_scan_truncated: scorecard.coverage.side_effect_scan_truncated,
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
        snapshot_max_files: scorecard.coverage.snapshot_max_files,
        snapshot_max_directories: scorecard.coverage.snapshot_max_directories,
        snapshot_max_hash_bytes: scorecard.coverage.snapshot_max_hash_bytes,
        hostile_binary_containment: scorecard.coverage.hostile_binary_containment,
        runtime_context: scorecard.runtime_context.clone(),
    })
}

pub(super) async fn write_scorecard(out_dir: &Path, scorecard: &Scorecard) -> Result<PathBuf> {
    let path = out_dir.join(SCORECARD_JSON);
    let bytes = serde_json::to_vec_pretty(&scorecard).map_err(CliareError::SerializeScorecard)?;
    write_atomic(&path, &bytes)
        .await
        .map_err(|source| CliareError::WriteScorecard {
            path: path.clone(),
            source,
        })?;
    Ok(path)
}

pub(super) async fn write_report(out_dir: &Path, scorecard: &Scorecard) -> Result<PathBuf> {
    let path = out_dir.join(REPORT_MD);
    let report = report::render(scorecard);
    write_atomic(&path, report.as_bytes())
        .await
        .map_err(|source| CliareError::WriteReport {
            path: path.clone(),
            source,
        })?;
    Ok(path)
}
