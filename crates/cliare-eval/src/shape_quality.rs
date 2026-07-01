use std::path::PathBuf;

use cliare_cli::cli::ShapeQualityArgs;
use cliare_core::error::{CliareError, Result};

mod io;
mod metrics;
mod model;
mod render;

#[cfg(test)]
mod tests;

pub use model::{
    EvidenceCompleteness, MetricReport, ProvenanceReport, ShapeQualityMetrics, ShapeQualityOverall,
    ShapeQualityReport,
};

const REPORT_SCHEMA_VERSION: &str = "cliare.shape-quality.v1";
const TRUTH_SCHEMA_VERSION: &str = "cliare.shape-truth.v1";

#[derive(Debug)]
pub struct ShapeQualitySummary {
    pub shape_path: PathBuf,
    pub truth_path: PathBuf,
    pub json_path: PathBuf,
    pub markdown_path: PathBuf,
    pub overall_score: Option<f64>,
    pub metrics_scored: usize,
}

impl ShapeQualitySummary {
    pub fn terminal_summary(&self) -> String {
        let score = self
            .overall_score
            .map(|score| format!("{score:.1}"))
            .unwrap_or_else(|| "n/a".to_owned());
        let lines = [
            "CLIARE shape-quality evaluation complete".to_owned(),
            format!("shape: {}", self.shape_path.display()),
            format!("truth: {}", self.truth_path.display()),
            format!("overall_score: {score}"),
            format!("metrics_scored: {}", self.metrics_scored),
            "artifacts:".to_owned(),
            format!("  json: {}", self.json_path.display()),
            format!("  markdown: {}", self.markdown_path.display()),
        ];

        format!("{}\n", lines.join("\n"))
    }
}

pub async fn shape_quality(args: ShapeQualityArgs) -> Result<ShapeQualitySummary> {
    let shape = io::read_shape(&args.shape).await?;
    let truth = io::read_truth(&args.truth).await?;
    if truth.schema_version != TRUTH_SCHEMA_VERSION {
        return Err(CliareError::UnsupportedEvalSchema {
            schema_version: truth.schema_version.clone(),
        });
    }

    tokio::fs::create_dir_all(&args.out)
        .await
        .map_err(|source| CliareError::CreateEvalDir {
            path: args.out.clone(),
            source,
        })?;

    let report = metrics::evaluate_shape_quality(
        REPORT_SCHEMA_VERSION,
        &args.shape,
        &args.truth,
        &shape,
        &truth,
    );
    let json_path = io::write_json_report(&args.out, &report).await?;
    let markdown_path =
        io::write_markdown_report(&args.out, &render::render_report(&report)).await?;

    Ok(ShapeQualitySummary {
        shape_path: args.shape,
        truth_path: args.truth,
        json_path,
        markdown_path,
        overall_score: report.overall.score,
        metrics_scored: report.overall.metrics_scored,
    })
}
