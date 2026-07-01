use std::path::{Path, PathBuf};

use crate::error::{CliareError, Result};

use super::format::{
    budget_label, escape_markdown, expected_score_label, optional_depth, optional_duration,
    optional_percent, optional_score, optional_usize, output_label, precondition_label,
    score_range,
};
use super::io::write_atomic;
use super::report_model::BenchmarkReport;

pub(super) async fn write_json_report(out_dir: &Path, report: &BenchmarkReport) -> Result<PathBuf> {
    let path = out_dir.join("benchmark.json");
    let bytes = serde_json::to_vec_pretty(report).map_err(CliareError::SerializeBenchmarkReport)?;
    write_atomic(&path, bytes, |path, source| {
        CliareError::WriteBenchmarkReport { path, source }
    })
    .await?;
    Ok(path)
}

pub(super) async fn write_markdown_report(
    out_dir: &Path,
    report: &BenchmarkReport,
) -> Result<PathBuf> {
    let path = out_dir.join("benchmark.md");
    let mut text = String::new();
    text.push_str("# CLIARE Benchmark Report\n\n");
    text.push_str(&format!(
        "- Corpus: `{}`\n",
        escape_markdown(&report.corpus)
    ));
    text.push_str(&format!(
        "- Result: `{}`\n",
        if report.totals.passed { "pass" } else { "fail" }
    ));
    text.push_str(&format!("- Duration: `{}` ms\n", report.duration_ms));
    text.push_str(&format!(
        "- Target concurrency: `{}`\n",
        report.target_concurrency
    ));
    text.push_str(&format!("- Complete: `{}`\n", report.totals.complete));
    text.push_str(&format!(
        "- Targets: `{}` measured, `{}` skipped, `{}` pending, `{}` failed\n",
        report.totals.measured, report.totals.skipped, report.totals.pending, report.totals.failed
    ));
    text.push_str(&format!(
        "- Expected band pass rate: `{}`\n",
        optional_percent(report.calibration.expected_band_pass_rate)
    ));
    text.push_str(&format!(
        "- Traversal completion rate: `{}`\n",
        optional_percent(report.calibration.traversal_completion_rate)
    ));
    text.push_str(&format!(
        "- Budget exhaustion rate: `{}`\n\n",
        optional_percent(report.calibration.budget_exhaustion_rate)
    ));
    text.push_str("## Corpus Metrics\n\n");
    text.push_str("| Metric | Value |\n");
    text.push_str("|---|---:|\n");
    text.push_str(&format!(
        "| Measured score mean | {} |\n",
        optional_score(report.calibration.score_mean)
    ));
    text.push_str(&format!(
        "| Measured score range | {} |\n",
        score_range(report.calibration.score_min, report.calibration.score_max)
    ));
    text.push_str(&format!(
        "| Expected band targets | {} |\n",
        report.calibration.expected_band_targets
    ));
    text.push_str(&format!(
        "| Expected band passed | {} |\n",
        report.calibration.expected_band_passed
    ));
    text.push_str(&format!(
        "| Probes completed | {} |\n",
        report.calibration.probes_completed_total
    ));
    text.push_str(&format!(
        "| Findings | {} |\n",
        report.calibration.findings_total
    ));
    text.push_str(&format!(
        "| Commands precondition-blocked | {} |\n",
        report.calibration.commands_precondition_blocked
    ));
    text.push_str(&format!(
        "| Precondition-blocked probes | {} |\n",
        report.calibration.precondition_blocked_probes
    ));
    text.push_str(&format!(
        "| Auth-required probes | {} |\n",
        report.calibration.auth_required_probes
    ));
    text.push_str(&format!(
        "| Local-context-required probes | {} |\n",
        report.calibration.local_context_required_probes
    ));
    text.push_str(&format!(
        "| Fixture-required probes | {} |\n",
        report.calibration.fixture_required_probes
    ));
    text.push_str(&format!(
        "| Output contracts discovered | {} |\n",
        report.calibration.output_contracts_discovered
    ));
    text.push_str(&format!(
        "| Machine-readable output contracts | {} |\n",
        report.calibration.machine_readable_output_contracts
    ));
    text.push_str(&format!(
        "| Output parse successes | {} |\n",
        report.calibration.output_mode_parse_successes
    ));
    text.push_str(&format!(
        "| Output precondition-blocked | {} |\n",
        report.calibration.output_mode_precondition_blocked
    ));
    text.push_str(&format!(
        "| Side-effect file changes | {} |\n",
        report.calibration.side_effect_files_total
    ));
    text.push_str(&format!(
        "| Probes with side effects | {} |\n",
        report.calibration.side_effect_probe_count
    ));
    text.push_str(&format!(
        "| Credential-like side effects | {} |\n\n",
        report.calibration.credential_like_side_effects
    ));

    text.push_str("## Targets\n\n");
    text.push_str("| Target | Status | Score | Expected | Duration | Probes | Depth | Budget | Preconditions | Output | Side effects | Issues |\n");
    text.push_str("|---|---|---:|---:|---:|---:|---:|---|---:|---:|---:|---|\n");
    for target in &report.targets {
        text.push_str(&format!(
            "| `{}` | `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            escape_markdown(&target.id),
            target.status.label(),
            optional_score(target.score),
            expected_score_label(target.expected_score.as_ref()),
            optional_duration(target.duration_ms),
            optional_usize(target.probes_completed),
            optional_depth(target.observed_max_depth, target.max_depth),
            budget_label(target),
            precondition_label(target),
            output_label(target),
            optional_usize(target.side_effect_files_total),
            escape_markdown(&target.issues.join("; "))
        ));
    }

    write_atomic(&path, text.into_bytes(), |path, source| {
        CliareError::WriteBenchmarkMarkdown { path, source }
    })
    .await?;
    Ok(path)
}
