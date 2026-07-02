use std::path::PathBuf;
use std::time::Instant;

use crate::artifact_guide;
use crate::cli::BenchmarkArgs;
use crate::error::{CliareError, Result};

mod corpus;
mod format;
mod io;
mod paths;
mod report_model;
mod runner;
mod writer;

#[cfg(test)]
mod tests;

use corpus::{read_corpus, validate_corpus};
use format::optional_percent;
use io::BenchmarkOutputLock;
use report_model::BenchmarkReport;
use runner::run_benchmark_targets;
use writer::{write_json_report, write_markdown_report};

const CORPUS_SCHEMA_VERSION: &str = "cliare.benchmark-corpus.v1";
const REPORT_SCHEMA_VERSION: &str = "cliare.benchmark-report.v1";

#[derive(Debug, Clone)]
pub struct BenchmarkSummary {
    pub manifest_path: PathBuf,
    pub report_path: PathBuf,
    pub markdown_path: PathBuf,
    pub readme_path: PathBuf,
    pub agent_skill_path: PathBuf,
    pub condition_dictionary_path: PathBuf,
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
            format!("  readme: {}", self.readme_path.display()),
            format!("  agent guide: {}", self.agent_skill_path.display()),
            format!(
                "  condition dictionary: {}",
                self.condition_dictionary_path.display()
            ),
        ];

        format!(
            "{}
",
            lines.join(
                "
"
            )
        )
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
    tokio::fs::create_dir_all(&args.out)
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
        .unwrap_or_else(|| std::path::Path::new("."));
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
    let guide_artifacts = artifact_guide::write_benchmark_guides(&args.out).await?;

    Ok(BenchmarkSummary {
        manifest_path: args.manifest,
        report_path,
        markdown_path,
        readme_path: guide_artifacts.readme_path,
        agent_skill_path: guide_artifacts.agent_skill_path,
        condition_dictionary_path: guide_artifacts.condition_dictionary_path,
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
