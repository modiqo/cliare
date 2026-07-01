use std::path::{Path, PathBuf};

use serde::Serialize;

use super::REPORT_SCHEMA_VERSION;
use super::corpus::{BenchmarkCorpus, BenchmarkTarget, ScoreBand};

#[derive(Debug, Serialize)]
pub(super) struct BenchmarkReport {
    pub(super) schema_version: &'static str,
    pub(super) corpus: String,
    pub(super) manifest_path: PathBuf,
    pub(super) artifact_dir: PathBuf,
    pub(super) duration_ms: u128,
    pub(super) target_concurrency: usize,
    pub(super) totals: BenchmarkTotals,
    pub(super) calibration: BenchmarkCalibration,
    pub(super) targets: Vec<BenchmarkTargetReport>,
}

impl BenchmarkReport {
    pub(super) fn new(
        corpus: &BenchmarkCorpus,
        manifest_path: PathBuf,
        artifact_dir: PathBuf,
        duration_ms: u128,
        target_concurrency: usize,
        targets: Vec<BenchmarkTargetReport>,
    ) -> Self {
        let totals = BenchmarkTotals::from_targets(&targets);
        let calibration = BenchmarkCalibration::from_targets(&targets);
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            corpus: corpus.name.clone(),
            manifest_path,
            artifact_dir,
            duration_ms,
            target_concurrency,
            totals,
            calibration,
            targets,
        }
    }
}

#[derive(Debug, Serialize)]
pub(super) struct BenchmarkTotals {
    pub(super) targets: usize,
    pub(super) measured: usize,
    pub(super) skipped: usize,
    pub(super) pending: usize,
    pub(super) failed: usize,
    pub(super) passed: bool,
    pub(super) complete: bool,
}

impl BenchmarkTotals {
    pub(super) fn from_targets(targets: &[BenchmarkTargetReport]) -> Self {
        let measured = targets
            .iter()
            .filter(|target| {
                matches!(
                    target.status,
                    BenchmarkTargetStatus::Passed | BenchmarkTargetStatus::Failed
                )
            })
            .count();
        let skipped = targets
            .iter()
            .filter(|target| target.status == BenchmarkTargetStatus::Skipped)
            .count();
        let failed = targets
            .iter()
            .filter(|target| target.status == BenchmarkTargetStatus::Failed)
            .count();
        let pending = targets
            .iter()
            .filter(|target| target.status == BenchmarkTargetStatus::Pending)
            .count();
        Self {
            targets: targets.len(),
            measured,
            skipped,
            pending,
            failed,
            passed: failed == 0 && pending == 0,
            complete: pending == 0,
        }
    }
}

#[derive(Debug, Serialize)]
pub(super) struct BenchmarkCalibration {
    pub(super) measured_targets: usize,
    pub(super) score_mean: Option<f64>,
    pub(super) score_min: Option<f64>,
    pub(super) score_max: Option<f64>,
    pub(super) expected_band_targets: usize,
    pub(super) expected_band_passed: usize,
    pub(super) expected_band_pass_rate: Option<f64>,
    pub(super) traversal_complete_targets: usize,
    pub(super) traversal_completion_rate: Option<f64>,
    pub(super) budget_exhausted_targets: usize,
    pub(super) budget_exhaustion_rate: Option<f64>,
    pub(super) probes_completed_total: usize,
    pub(super) findings_total: usize,
    pub(super) commands_precondition_blocked: usize,
    pub(super) precondition_blocked_probes: usize,
    pub(super) auth_required_probes: usize,
    pub(super) local_context_required_probes: usize,
    pub(super) fixture_required_probes: usize,
    pub(super) output_contracts_discovered: usize,
    pub(super) machine_readable_output_contracts: usize,
    pub(super) output_mode_parse_successes: usize,
    pub(super) output_mode_precondition_blocked: usize,
    pub(super) side_effect_files_total: usize,
    pub(super) side_effect_probe_count: usize,
    pub(super) credential_like_side_effects: usize,
}

impl BenchmarkCalibration {
    pub(super) fn from_targets(targets: &[BenchmarkTargetReport]) -> Self {
        let scores = targets
            .iter()
            .filter_map(|target| target.score)
            .collect::<Vec<_>>();
        let measured_targets = scores.len();
        let score_sum = scores.iter().sum::<f64>();
        let score_mean = if scores.is_empty() {
            None
        } else {
            Some(score_sum / scores.len() as f64)
        };
        let score_min = scores.iter().copied().reduce(f64::min);
        let score_max = scores.iter().copied().reduce(f64::max);

        let expected_band_targets = targets
            .iter()
            .filter(|target| target.score.is_some() && target.expected_score.is_some())
            .count();
        let expected_band_passed = targets
            .iter()
            .filter(|target| {
                target
                    .score
                    .zip(target.expected_score.as_ref())
                    .is_some_and(|(score, band)| (band.min..=band.max).contains(&score))
            })
            .count();
        let expected_band_pass_rate = ratio(expected_band_passed, expected_band_targets);

        let traversal_complete_targets = targets
            .iter()
            .filter(|target| target.traversal_complete == Some(true))
            .count();
        let traversal_completion_rate = ratio(traversal_complete_targets, measured_targets);
        let budget_exhausted_targets = targets
            .iter()
            .filter(|target| target.budget_exhausted == Some(true))
            .count();
        let budget_exhaustion_rate = ratio(budget_exhausted_targets, measured_targets);

        Self {
            measured_targets,
            score_mean: score_mean.map(round_metric),
            score_min: score_min.map(round_metric),
            score_max: score_max.map(round_metric),
            expected_band_targets,
            expected_band_passed,
            expected_band_pass_rate,
            traversal_complete_targets,
            traversal_completion_rate,
            budget_exhausted_targets,
            budget_exhaustion_rate,
            probes_completed_total: sum_optional_usize(targets, |target| target.probes_completed),
            findings_total: sum_optional_usize(targets, |target| target.findings),
            commands_precondition_blocked: sum_optional_usize(targets, |target| {
                target.commands_precondition_blocked
            }),
            precondition_blocked_probes: sum_optional_usize(targets, |target| {
                target.precondition_blocked_probes
            }),
            auth_required_probes: sum_optional_usize(targets, |target| target.auth_required_probes),
            local_context_required_probes: sum_optional_usize(targets, |target| {
                target.local_context_required_probes
            }),
            fixture_required_probes: sum_optional_usize(targets, |target| {
                target.fixture_required_probes
            }),
            output_contracts_discovered: sum_optional_usize(targets, |target| {
                target.output_contracts_discovered
            }),
            machine_readable_output_contracts: sum_optional_usize(targets, |target| {
                target.machine_readable_output_contracts
            }),
            output_mode_parse_successes: sum_optional_usize(targets, |target| {
                target.output_mode_parse_successes
            }),
            output_mode_precondition_blocked: sum_optional_usize(targets, |target| {
                target.output_mode_precondition_blocked
            }),
            side_effect_files_total: sum_optional_usize(targets, |target| {
                target.side_effect_files_total
            }),
            side_effect_probe_count: sum_optional_usize(targets, |target| {
                target.side_effect_probe_count
            }),
            credential_like_side_effects: sum_optional_usize(targets, |target| {
                target.credential_like_side_effects
            }),
        }
    }
}

fn ratio(numerator: usize, denominator: usize) -> Option<f64> {
    if denominator == 0 {
        None
    } else {
        Some(round_metric(numerator as f64 / denominator as f64))
    }
}

fn round_metric(value: f64) -> f64 {
    (value * 1_000.0).round() / 1_000.0
}

fn sum_optional_usize(
    targets: &[BenchmarkTargetReport],
    selector: impl Fn(&BenchmarkTargetReport) -> Option<usize>,
) -> usize {
    targets.iter().filter_map(selector).sum()
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct BenchmarkTargetReport {
    pub(super) id: String,
    pub(super) target: String,
    pub(super) resolved_target: String,
    pub(super) required: bool,
    pub(super) tags: Vec<String>,
    pub(super) status: BenchmarkTargetStatus,
    pub(super) score: Option<f64>,
    pub(super) expected_score: Option<ScoreBand>,
    pub(super) duration_ms: Option<u128>,
    pub(super) max_duration_ms: Option<u128>,
    pub(super) probes_completed: Option<usize>,
    pub(super) findings: Option<usize>,
    pub(super) traversal_complete: Option<bool>,
    pub(super) budget_exhausted: Option<bool>,
    pub(super) traversal_stop_reason: Option<String>,
    pub(super) observed_max_depth: Option<usize>,
    pub(super) max_depth: Option<usize>,
    pub(super) max_probes: Option<usize>,
    pub(super) concurrency_limit: Option<usize>,
    pub(super) commands_precondition_blocked: Option<usize>,
    pub(super) precondition_blocked_probes: Option<usize>,
    pub(super) auth_required_probes: Option<usize>,
    pub(super) local_context_required_probes: Option<usize>,
    pub(super) fixture_required_probes: Option<usize>,
    pub(super) output_contracts_discovered: Option<usize>,
    pub(super) machine_readable_output_contracts: Option<usize>,
    pub(super) output_mode_parse_successes: Option<usize>,
    pub(super) output_mode_precondition_blocked: Option<usize>,
    pub(super) side_effect_files_total: Option<usize>,
    pub(super) side_effect_probe_count: Option<usize>,
    pub(super) credential_like_side_effects: Option<usize>,
    pub(super) artifact_dir: PathBuf,
    pub(super) issues: Vec<String>,
}

impl BenchmarkTargetReport {
    pub(super) fn pending(
        target: &BenchmarkTarget,
        resolved_target: &Path,
        artifact_dir: PathBuf,
    ) -> Self {
        Self {
            id: target.id.clone(),
            target: target.target.display().to_string(),
            resolved_target: resolved_target.display().to_string(),
            required: target.required,
            tags: target.tags.clone(),
            status: BenchmarkTargetStatus::Pending,
            score: None,
            expected_score: target.expected_score.clone(),
            duration_ms: None,
            max_duration_ms: target.max_duration_ms,
            probes_completed: None,
            findings: None,
            traversal_complete: None,
            budget_exhausted: None,
            traversal_stop_reason: None,
            observed_max_depth: None,
            max_depth: None,
            max_probes: None,
            concurrency_limit: None,
            commands_precondition_blocked: None,
            precondition_blocked_probes: None,
            auth_required_probes: None,
            local_context_required_probes: None,
            fixture_required_probes: None,
            output_contracts_discovered: None,
            machine_readable_output_contracts: None,
            output_mode_parse_successes: None,
            output_mode_precondition_blocked: None,
            side_effect_files_total: None,
            side_effect_probe_count: None,
            credential_like_side_effects: None,
            artifact_dir,
            issues: Vec::new(),
        }
    }

    pub(super) fn skipped(
        target: &BenchmarkTarget,
        resolved_target: &Path,
        artifact_dir: PathBuf,
        reason: String,
    ) -> Self {
        Self {
            id: target.id.clone(),
            target: target.target.display().to_string(),
            resolved_target: resolved_target.display().to_string(),
            required: target.required,
            tags: target.tags.clone(),
            status: BenchmarkTargetStatus::Skipped,
            score: None,
            expected_score: target.expected_score.clone(),
            duration_ms: None,
            max_duration_ms: target.max_duration_ms,
            probes_completed: None,
            findings: None,
            traversal_complete: None,
            budget_exhausted: None,
            traversal_stop_reason: None,
            observed_max_depth: None,
            max_depth: None,
            max_probes: None,
            concurrency_limit: None,
            commands_precondition_blocked: None,
            precondition_blocked_probes: None,
            auth_required_probes: None,
            local_context_required_probes: None,
            fixture_required_probes: None,
            output_contracts_discovered: None,
            machine_readable_output_contracts: None,
            output_mode_parse_successes: None,
            output_mode_precondition_blocked: None,
            side_effect_files_total: None,
            side_effect_probe_count: None,
            credential_like_side_effects: None,
            artifact_dir,
            issues: vec![reason],
        }
    }

    pub(super) fn failed(
        target: &BenchmarkTarget,
        resolved_target: &Path,
        artifact_dir: PathBuf,
        duration_ms: u128,
        reason: String,
    ) -> Self {
        Self {
            id: target.id.clone(),
            target: target.target.display().to_string(),
            resolved_target: resolved_target.display().to_string(),
            required: target.required,
            tags: target.tags.clone(),
            status: BenchmarkTargetStatus::Failed,
            score: None,
            expected_score: target.expected_score.clone(),
            duration_ms: Some(duration_ms),
            max_duration_ms: target.max_duration_ms,
            probes_completed: None,
            findings: None,
            traversal_complete: None,
            budget_exhausted: None,
            traversal_stop_reason: None,
            observed_max_depth: None,
            max_depth: None,
            max_probes: None,
            concurrency_limit: None,
            commands_precondition_blocked: None,
            precondition_blocked_probes: None,
            auth_required_probes: None,
            local_context_required_probes: None,
            fixture_required_probes: None,
            output_contracts_discovered: None,
            machine_readable_output_contracts: None,
            output_mode_parse_successes: None,
            output_mode_precondition_blocked: None,
            side_effect_files_total: None,
            side_effect_probe_count: None,
            credential_like_side_effects: None,
            artifact_dir,
            issues: vec![reason],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum BenchmarkTargetStatus {
    Passed,
    Failed,
    Skipped,
    Pending,
}

impl BenchmarkTargetStatus {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Pending => "pending",
        }
    }
}
