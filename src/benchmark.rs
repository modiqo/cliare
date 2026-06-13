use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::task::JoinSet;

use crate::cli::{BenchmarkArgs, MeasureArgs, TraversalProfile};
use crate::error::{CliareError, Result};
use crate::measure::{self, MeasurementSummary};

const CORPUS_SCHEMA_VERSION: &str = "cliare.benchmark-corpus.v1";
const REPORT_SCHEMA_VERSION: &str = "cliare.benchmark-report.v1";

#[derive(Debug, Clone)]
pub struct BenchmarkSummary {
    pub manifest_path: PathBuf,
    pub report_path: PathBuf,
    pub markdown_path: PathBuf,
    pub targets_total: usize,
    pub measured: usize,
    pub skipped: usize,
    pub failed: usize,
    pub passed: bool,
    pub target_concurrency: usize,
    pub expected_band_pass_rate: Option<f64>,
    pub traversal_completion_rate: Option<f64>,
    pub budget_exhaustion_rate: Option<f64>,
    pub duration_ms: u128,
}

impl BenchmarkSummary {
    pub fn terminal_summary(&self) -> String {
        let result = if self.passed { "pass" } else { "fail" };
        let lines = [
            "CLIARE benchmark complete".to_owned(),
            format!("result: {result}"),
            format!("manifest: {}", self.manifest_path.display()),
            format!("targets: {}", self.targets_total),
            format!("measured: {}", self.measured),
            format!("skipped: {}", self.skipped),
            format!("failed: {}", self.failed),
            format!("target_concurrency: {}", self.target_concurrency),
            format!(
                "expected_band_pass_rate: {}",
                optional_percent(self.expected_band_pass_rate)
            ),
            format!(
                "traversal_completion_rate: {}",
                optional_percent(self.traversal_completion_rate)
            ),
            format!(
                "budget_exhaustion_rate: {}",
                optional_percent(self.budget_exhaustion_rate)
            ),
            format!("duration_ms: {}", self.duration_ms),
            "artifacts:".to_owned(),
            format!("  report: {}", self.report_path.display()),
            format!("  markdown: {}", self.markdown_path.display()),
        ];

        format!("{}\n", lines.join("\n"))
    }
}

pub async fn benchmark(args: BenchmarkArgs) -> Result<BenchmarkSummary> {
    let started = Instant::now();
    let corpus = read_corpus(&args.manifest).await?;
    validate_corpus(&corpus)?;
    if let Some(0) = args.target_concurrency {
        return Err(CliareError::InvalidBenchmarkPositiveInteger {
            field: "target_concurrency",
            value: 0,
        });
    }
    fs::create_dir_all(&args.out)
        .await
        .map_err(|source| CliareError::CreateBenchmarkDir {
            path: args.out.clone(),
            source,
        })?;
    let _lock = BenchmarkOutputLock::acquire(&args.out).await?;
    let target_concurrency = args
        .target_concurrency
        .or(corpus.defaults.target_concurrency)
        .unwrap_or(1);

    let manifest_dir = args
        .manifest
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let target_reports = run_benchmark_targets(
        &corpus,
        &args.manifest,
        manifest_dir,
        &args.out,
        started,
        args.refresh,
        target_concurrency,
    )
    .await?;

    let report = BenchmarkReport::new(
        &corpus,
        args.manifest.clone(),
        args.out.clone(),
        started.elapsed().as_millis(),
        target_concurrency,
        target_reports,
    );
    let report_path = write_json_report(&args.out, &report).await?;
    let markdown_path = write_markdown_report(&args.out, &report).await?;

    Ok(BenchmarkSummary {
        manifest_path: args.manifest,
        report_path,
        markdown_path,
        targets_total: report.totals.targets,
        measured: report.totals.measured,
        skipped: report.totals.skipped,
        failed: report.totals.failed,
        passed: report.totals.passed,
        target_concurrency: report.target_concurrency,
        expected_band_pass_rate: report.calibration.expected_band_pass_rate,
        traversal_completion_rate: report.calibration.traversal_completion_rate,
        budget_exhaustion_rate: report.calibration.budget_exhaustion_rate,
        duration_ms: report.duration_ms,
    })
}

async fn run_benchmark_targets(
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

async fn read_corpus(path: &Path) -> Result<BenchmarkCorpus> {
    let bytes = fs::read(path)
        .await
        .map_err(|source| CliareError::ReadBenchmarkManifest {
            path: path.to_path_buf(),
            source,
        })?;
    serde_json::from_slice(&bytes).map_err(|source| CliareError::ParseBenchmarkManifest {
        path: path.to_path_buf(),
        source,
    })
}

fn validate_corpus(corpus: &BenchmarkCorpus) -> Result<()> {
    if corpus.schema_version != CORPUS_SCHEMA_VERSION {
        return Err(CliareError::UnsupportedBenchmarkSchema {
            schema_version: corpus.schema_version.clone(),
        });
    }
    validate_positive(
        corpus.defaults.target_concurrency,
        "defaults.target_concurrency",
    )?;
    validate_positive(corpus.defaults.concurrency, "defaults.concurrency")?;
    for target in &corpus.targets {
        validate_positive(target.concurrency, "targets.concurrency")?;
        if let Some(band) = &target.expected_score
            && !(band.min.is_finite()
                && band.max.is_finite()
                && (0.0..=100.0).contains(&band.min)
                && (0.0..=100.0).contains(&band.max)
                && band.min <= band.max)
        {
            return Err(CliareError::InvalidBenchmarkScoreBand {
                target_id: target.id.clone(),
                min: band.min,
                max: band.max,
            });
        }
    }
    Ok(())
}

fn validate_positive(value: Option<usize>, field: &'static str) -> Result<()> {
    if let Some(0) = value {
        return Err(CliareError::InvalidBenchmarkPositiveInteger { field, value: 0 });
    }
    Ok(())
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
            "score {:.1} is outside expected band {:.1}..={:.1}",
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
        traversal_stop_reason: Some(summary.traversal_stop_reason),
        observed_max_depth: Some(summary.observed_max_depth),
        max_depth: Some(summary.max_depth),
        max_probes: Some(summary.max_probes),
        concurrency_limit: Some(summary.concurrency_limit),
        commands_precondition_blocked: Some(summary.commands_precondition_blocked),
        precondition_blocked_probes: Some(summary.precondition_blocked_probes),
        auth_required_probes: Some(summary.auth_required_probes),
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

fn resolve_manifest_target(manifest_dir: &Path, target: &Path) -> PathBuf {
    if is_path_like(target) && target.is_relative() {
        manifest_dir.join(target)
    } else {
        target.to_path_buf()
    }
}

fn is_path_like(path: &Path) -> bool {
    path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
        || path.components().count() > 1
}

fn sanitize_target_id(id: &str) -> String {
    id.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

async fn write_json_report(out_dir: &Path, report: &BenchmarkReport) -> Result<PathBuf> {
    let path = out_dir.join("benchmark.json");
    let bytes = serde_json::to_vec_pretty(report).map_err(CliareError::SerializeBenchmarkReport)?;
    write_atomic(&path, bytes, |path, source| {
        CliareError::WriteBenchmarkReport { path, source }
    })
    .await?;
    Ok(path)
}

async fn write_markdown_report(out_dir: &Path, report: &BenchmarkReport) -> Result<PathBuf> {
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

async fn write_atomic(
    path: &Path,
    bytes: Vec<u8>,
    error: impl Fn(PathBuf, std::io::Error) -> CliareError,
) -> Result<()> {
    let temp_path = atomic_temp_path(path);
    fs::write(&temp_path, bytes)
        .await
        .map_err(|source| error(temp_path.clone(), source))?;
    fs::rename(&temp_path, path)
        .await
        .map_err(|source| error(path.to_path_buf(), source))?;
    Ok(())
}

fn atomic_temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("benchmark");
    path.with_file_name(format!("{file_name}.tmp.{}", std::process::id()))
}

struct BenchmarkOutputLock {
    path: PathBuf,
}

impl BenchmarkOutputLock {
    async fn acquire(out_dir: &Path) -> Result<Self> {
        let path = out_dir.join(".benchmark.lock");
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
        {
            Ok(file) => file,
            Err(source) if source.kind() == ErrorKind::AlreadyExists => {
                return Err(CliareError::BenchmarkOutputLocked { path });
            }
            Err(source) => {
                return Err(CliareError::AcquireBenchmarkLock { path, source });
            }
        };
        let contents = format!("pid={}\n", std::process::id());
        file.write_all(contents.as_bytes())
            .await
            .map_err(|source| CliareError::AcquireBenchmarkLock {
                path: path.clone(),
                source,
            })?;
        Ok(Self { path })
    }
}

impl Drop for BenchmarkOutputLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn optional_score(score: Option<f64>) -> String {
    score.map_or_else(|| "n/a".to_owned(), |score| format!("{score:.1}"))
}

fn optional_percent(value: Option<f64>) -> String {
    value.map_or_else(
        || "n/a".to_owned(),
        |value| format!("{:.1}%", value * 100.0),
    )
}

fn optional_duration(duration_ms: Option<u128>) -> String {
    duration_ms.map_or_else(|| "n/a".to_owned(), |duration| format!("{duration} ms"))
}

fn optional_usize(value: Option<usize>) -> String {
    value.map_or_else(|| "n/a".to_owned(), |value| value.to_string())
}

fn optional_depth(observed: Option<usize>, max: Option<usize>) -> String {
    match (observed, max) {
        (Some(observed), Some(max)) => format!("{observed}/{max}"),
        _ => "n/a".to_owned(),
    }
}

fn score_range(min: Option<f64>, max: Option<f64>) -> String {
    match (min, max) {
        (Some(min), Some(max)) => format!("{min:.1}..={max:.1}"),
        _ => "n/a".to_owned(),
    }
}

fn budget_label(target: &BenchmarkTargetReport) -> String {
    match (
        target.budget_exhausted,
        target.traversal_stop_reason.as_deref(),
    ) {
        (Some(true), Some(reason)) => format!("exhausted:{reason}"),
        (Some(false), Some(reason)) => reason.to_owned(),
        (Some(value), None) => value.to_string(),
        _ => "n/a".to_owned(),
    }
}

fn output_label(target: &BenchmarkTargetReport) -> String {
    match (
        target.machine_readable_output_contracts,
        target.output_contracts_discovered,
        target.output_mode_parse_successes,
    ) {
        (Some(machine), Some(discovered), Some(parse_successes)) => {
            format!("{machine}/{discovered}; parse {parse_successes}")
        }
        _ => "n/a".to_owned(),
    }
}

fn precondition_label(target: &BenchmarkTargetReport) -> String {
    match (
        target.commands_precondition_blocked,
        target.precondition_blocked_probes,
        target.auth_required_probes,
    ) {
        (Some(commands), Some(probes), Some(auth)) => format!("{commands}/{probes}/{auth}"),
        _ => "n/a".to_owned(),
    }
}

fn expected_score_label(score: Option<&ScoreBand>) -> String {
    score.map_or_else(
        || "n/a".to_owned(),
        |score| format!("{:.1}..={:.1}", score.min, score.max),
    )
}

fn escape_markdown(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

#[derive(Debug, Clone, Deserialize)]
struct BenchmarkCorpus {
    schema_version: String,
    name: String,
    #[serde(default)]
    defaults: BenchmarkDefaults,
    targets: Vec<BenchmarkTarget>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct BenchmarkDefaults {
    target_concurrency: Option<usize>,
    profile: Option<TraversalProfile>,
    max_depth: Option<usize>,
    max_probes: Option<usize>,
    min_expected_value: Option<u16>,
    concurrency: Option<usize>,
    timeout_ms: Option<u64>,
    output_limit_bytes: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct BenchmarkTarget {
    id: String,
    target: PathBuf,
    #[serde(default = "default_required")]
    required: bool,
    #[serde(default)]
    tags: Vec<String>,
    profile: Option<TraversalProfile>,
    max_depth: Option<usize>,
    max_probes: Option<usize>,
    min_expected_value: Option<u16>,
    concurrency: Option<usize>,
    timeout_ms: Option<u64>,
    output_limit_bytes: Option<usize>,
    expected_score: Option<ScoreBand>,
    max_duration_ms: Option<u128>,
}

impl BenchmarkTarget {
    fn measure_args(
        &self,
        target: PathBuf,
        out: PathBuf,
        defaults: &BenchmarkDefaults,
        refresh: bool,
    ) -> MeasureArgs {
        let profile = self
            .profile
            .or(defaults.profile)
            .unwrap_or(TraversalProfile::Quick);
        MeasureArgs {
            target,
            out,
            timeout_ms: self.timeout_ms.or(defaults.timeout_ms).unwrap_or(5_000),
            output_limit_bytes: self
                .output_limit_bytes
                .or(defaults.output_limit_bytes)
                .unwrap_or(1_048_576),
            profile,
            max_depth: self.max_depth.or(defaults.max_depth),
            max_probes: self.max_probes.or(defaults.max_probes),
            min_expected_value: self.min_expected_value.or(defaults.min_expected_value),
            concurrency: self.concurrency.or(defaults.concurrency),
            refresh,
        }
    }
}

fn default_required() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ScoreBand {
    min: f64,
    max: f64,
}

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    schema_version: &'static str,
    corpus: String,
    manifest_path: PathBuf,
    artifact_dir: PathBuf,
    duration_ms: u128,
    target_concurrency: usize,
    totals: BenchmarkTotals,
    calibration: BenchmarkCalibration,
    targets: Vec<BenchmarkTargetReport>,
}

impl BenchmarkReport {
    fn new(
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
struct BenchmarkTotals {
    targets: usize,
    measured: usize,
    skipped: usize,
    pending: usize,
    failed: usize,
    passed: bool,
    complete: bool,
}

impl BenchmarkTotals {
    fn from_targets(targets: &[BenchmarkTargetReport]) -> Self {
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
struct BenchmarkCalibration {
    measured_targets: usize,
    score_mean: Option<f64>,
    score_min: Option<f64>,
    score_max: Option<f64>,
    expected_band_targets: usize,
    expected_band_passed: usize,
    expected_band_pass_rate: Option<f64>,
    traversal_complete_targets: usize,
    traversal_completion_rate: Option<f64>,
    budget_exhausted_targets: usize,
    budget_exhaustion_rate: Option<f64>,
    probes_completed_total: usize,
    findings_total: usize,
    commands_precondition_blocked: usize,
    precondition_blocked_probes: usize,
    auth_required_probes: usize,
    output_contracts_discovered: usize,
    machine_readable_output_contracts: usize,
    output_mode_parse_successes: usize,
    output_mode_precondition_blocked: usize,
    side_effect_files_total: usize,
    side_effect_probe_count: usize,
    credential_like_side_effects: usize,
}

impl BenchmarkCalibration {
    fn from_targets(targets: &[BenchmarkTargetReport]) -> Self {
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
struct BenchmarkTargetReport {
    id: String,
    target: String,
    resolved_target: String,
    required: bool,
    tags: Vec<String>,
    status: BenchmarkTargetStatus,
    score: Option<f64>,
    expected_score: Option<ScoreBand>,
    duration_ms: Option<u128>,
    max_duration_ms: Option<u128>,
    probes_completed: Option<usize>,
    findings: Option<usize>,
    traversal_complete: Option<bool>,
    budget_exhausted: Option<bool>,
    traversal_stop_reason: Option<String>,
    observed_max_depth: Option<usize>,
    max_depth: Option<usize>,
    max_probes: Option<usize>,
    concurrency_limit: Option<usize>,
    commands_precondition_blocked: Option<usize>,
    precondition_blocked_probes: Option<usize>,
    auth_required_probes: Option<usize>,
    output_contracts_discovered: Option<usize>,
    machine_readable_output_contracts: Option<usize>,
    output_mode_parse_successes: Option<usize>,
    output_mode_precondition_blocked: Option<usize>,
    side_effect_files_total: Option<usize>,
    side_effect_probe_count: Option<usize>,
    credential_like_side_effects: Option<usize>,
    artifact_dir: PathBuf,
    issues: Vec<String>,
}

impl BenchmarkTargetReport {
    fn pending(target: &BenchmarkTarget, resolved_target: &Path, artifact_dir: PathBuf) -> Self {
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

    fn skipped(
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

    fn failed(
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
enum BenchmarkTargetStatus {
    Passed,
    Failed,
    Skipped,
    Pending,
}

impl BenchmarkTargetStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Pending => "pending",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{is_path_like, sanitize_target_id};

    #[test]
    fn target_id_sanitization_keeps_paths_portable() {
        assert_eq!(sanitize_target_id("git"), "git");
        assert_eq!(sanitize_target_id("foo/bar baz"), "foo_bar_baz");
    }

    #[test]
    fn path_like_detection_keeps_command_names_on_path() {
        assert!(!is_path_like(Path::new("git")));
        assert!(is_path_like(Path::new("./target/debug/cliare")));
        assert!(is_path_like(Path::new("../target/debug/cliare")));
        assert!(is_path_like(Path::new("/usr/bin/git")));
    }
}
