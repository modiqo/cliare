use std::path::{Path, PathBuf};
use std::time::Instant;

use tokio::task::JoinSet;

use crate::artifact_guide;
use crate::error::{CliareError, Result};
use crate::measure::{self, MeasurementSummary};

use super::corpus::{BenchmarkCorpus, BenchmarkDefaults, BenchmarkTarget};
use super::paths::{resolve_manifest_target, sanitize_target_id};
use super::report_model::{BenchmarkReport, BenchmarkTargetReport, BenchmarkTargetStatus};
use super::writer::{write_json_report, write_markdown_report};

pub(super) async fn run_benchmark_targets(
    corpus: &BenchmarkCorpus,
    manifest_path: &Path,
    manifest_dir: &Path,
    out_dir: &Path,
    started: Instant,
    refresh: bool,
    target_concurrency: usize,
) -> Result<Vec<BenchmarkTargetReport>> {
    let mut next_index = 0_usize;
    let mut reports = std::iter::repeat_with(|| None)
        .take(corpus.targets.len())
        .collect::<Vec<Option<BenchmarkTargetReport>>>();
    let mut tasks = JoinSet::new();

    write_progress_report(
        corpus,
        manifest_path,
        manifest_dir,
        out_dir,
        started,
        target_concurrency,
        &reports,
    )
    .await?;

    while next_index < corpus.targets.len() || !tasks.is_empty() {
        while next_index < corpus.targets.len() && tasks.len() < target_concurrency {
            let target = corpus.targets[next_index].clone();
            let defaults = corpus.defaults.clone();
            let artifact_dir = out_dir.join(sanitize_target_id(&target.id));
            let resolved_target = resolve_manifest_target(manifest_dir, &target.target);
            let index = next_index;
            tasks.spawn(async move {
                (
                    index,
                    run_benchmark_target(target, defaults, resolved_target, artifact_dir, refresh)
                        .await,
                )
            });
            next_index += 1;
        }

        let Some(joined) = tasks.join_next().await else {
            break;
        };
        let (index, report) = joined.map_err(CliareError::Join)?;
        reports[index] = Some(report);
        write_progress_report(
            corpus,
            manifest_path,
            manifest_dir,
            out_dir,
            started,
            target_concurrency,
            &reports,
        )
        .await?;
    }

    Ok(materialize_target_reports(
        &corpus.targets,
        manifest_dir,
        out_dir,
        &reports,
    ))
}

async fn write_progress_report(
    corpus: &BenchmarkCorpus,
    manifest_path: &Path,
    manifest_dir: &Path,
    out_dir: &Path,
    started: Instant,
    target_concurrency: usize,
    reports: &[Option<BenchmarkTargetReport>],
) -> Result<()> {
    let report = BenchmarkReport::new(
        corpus,
        manifest_path.to_path_buf(),
        out_dir.to_path_buf(),
        started.elapsed().as_millis(),
        target_concurrency,
        materialize_target_reports(&corpus.targets, manifest_dir, out_dir, reports),
    );
    write_json_report(out_dir, &report).await?;
    write_markdown_report(out_dir, &report).await?;
    artifact_guide::write_benchmark_guides(out_dir).await?;
    Ok(())
}

fn materialize_target_reports(
    targets: &[BenchmarkTarget],
    manifest_dir: &Path,
    out_dir: &Path,
    reports: &[Option<BenchmarkTargetReport>],
) -> Vec<BenchmarkTargetReport> {
    targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            reports
                .get(index)
                .and_then(Clone::clone)
                .unwrap_or_else(|| {
                    let artifact_dir = out_dir.join(sanitize_target_id(&target.id));
                    let resolved_target = resolve_manifest_target(manifest_dir, &target.target);
                    BenchmarkTargetReport::pending(target, &resolved_target, artifact_dir)
                })
        })
        .collect()
}

async fn run_benchmark_target(
    target: BenchmarkTarget,
    defaults: BenchmarkDefaults,
    resolved_target: PathBuf,
    artifact_dir: PathBuf,
    refresh: bool,
) -> BenchmarkTargetReport {
    let target_started = Instant::now();
    let measure_args = target.measure_args(
        resolved_target.clone(),
        artifact_dir.clone(),
        &defaults,
        refresh,
    );

    match measure::measure(measure_args).await {
        Ok(summary) => target_report_from_summary(
            &target,
            &resolved_target,
            artifact_dir,
            summary,
            target_started.elapsed().as_millis(),
        ),
        Err(error) if !target.required && missing_target(&error) => BenchmarkTargetReport::skipped(
            &target,
            &resolved_target,
            artifact_dir,
            format!(
                "target was not found: {resolved_target_display}",
                resolved_target_display = resolved_target.display()
            ),
        ),
        Err(error) => BenchmarkTargetReport::failed(
            &target,
            &resolved_target,
            artifact_dir,
            target_started.elapsed().as_millis(),
            error.to_string(),
        ),
    }
}

fn target_report_from_summary(
    target: &BenchmarkTarget,
    resolved_target: &Path,
    artifact_dir: PathBuf,
    summary: MeasurementSummary,
    duration_ms: u128,
) -> BenchmarkTargetReport {
    let mut issues = Vec::new();
    if let Some(band) = &target.expected_score
        && !(band.min..=band.max).contains(&summary.score_total)
    {
        issues.push(format!(
            "score {:.0} is outside expected band {:.0}..={:.0}",
            summary.score_total, band.min, band.max
        ));
    }
    if let Some(max_duration_ms) = target.max_duration_ms
        && duration_ms > max_duration_ms
    {
        issues.push(format!(
            "duration {duration_ms}ms exceeded maximum {max_duration_ms}ms"
        ));
    }

    BenchmarkTargetReport {
        id: target.id.clone(),
        target: target.target.display().to_string(),
        resolved_target: resolved_target.display().to_string(),
        required: target.required,
        tags: target.tags.clone(),
        status: if issues.is_empty() {
            BenchmarkTargetStatus::Passed
        } else {
            BenchmarkTargetStatus::Failed
        },
        score: Some(summary.score_total),
        expected_score: target.expected_score.clone(),
        duration_ms: Some(duration_ms),
        max_duration_ms: target.max_duration_ms,
        probes_completed: Some(summary.probes_completed),
        findings: Some(summary.findings),
        traversal_complete: Some(summary.traversal_complete),
        budget_exhausted: Some(summary.budget_exhausted),
        traversal_stop_reason: Some(summary.traversal_stop_reason.clone()),
        observed_max_depth: Some(summary.observed_max_depth),
        max_depth: Some(summary.max_depth),
        max_probes: Some(summary.max_probes),
        concurrency_limit: Some(summary.concurrency_limit),
        commands_precondition_blocked: Some(summary.commands_precondition_blocked),
        precondition_blocked_probes: Some(summary.precondition_blocked_probes),
        auth_required_probes: Some(summary.auth_required_probes),
        local_context_required_probes: Some(summary.local_context_required_probes),
        fixture_required_probes: Some(summary.fixture_required_probes),
        output_contracts_discovered: Some(summary.output_contracts_discovered),
        machine_readable_output_contracts: Some(summary.machine_readable_output_contracts),
        output_mode_parse_successes: Some(summary.output_mode_parse_successes),
        output_mode_precondition_blocked: Some(summary.output_mode_precondition_blocked),
        side_effect_files_total: Some(summary.side_effect_files_total),
        side_effect_probe_count: Some(summary.side_effect_probe_count),
        credential_like_side_effects: Some(summary.credential_like_side_effects),
        artifact_dir,
        issues,
    }
}

fn missing_target(error: &CliareError) -> bool {
    matches!(error, CliareError::TargetNotFound(_))
}
