use std::path::{Path, PathBuf};

use serde::Deserialize;
use tokio::fs;

use crate::cli::{GuardArgs, MeasureArgs};
use crate::error::{CliareError, Result};
use crate::measure::{self, MeasurementSummary};

#[derive(Debug, Clone)]
pub struct GuardSummary {
    pub measurement: MeasurementSummary,
    pub baseline_path: PathBuf,
    pub baseline_total: f64,
    pub current_total: f64,
    pub delta: f64,
    pub allowed_drop: f64,
    pub passed: bool,
}

impl GuardSummary {
    pub fn terminal_summary(&self) -> String {
        let result = if self.passed { "pass" } else { "fail" };
        let lines = [
            self.measurement.terminal_summary().trim_end().to_owned(),
            "guard:".to_owned(),
            format!("  result: {result}"),
            format!("  baseline: {}", self.baseline_path.display()),
            format!("  baseline score: {:.1}", self.baseline_total),
            format!("  current score: {:.1}", self.current_total),
            format!("  delta: {:+.1}", self.delta),
            format!("  allowed drop: {:.1}", self.allowed_drop),
        ];

        format!("{}\n", lines.join("\n"))
    }
}

pub async fn guard(args: GuardArgs) -> Result<GuardSummary> {
    let baseline = read_baseline(&args.baseline).await?;
    let allowed_drop = allowed_drop(args.allowed_drop)?;
    let baseline_path = args.baseline.clone();
    let measurement = measure::measure(MeasureArgs::from(&args)).await?;
    let current_total = measurement.score_total;
    let baseline_total = baseline.total()?;
    let delta = current_total - baseline_total;
    let passed = delta + allowed_drop >= 0.0;

    Ok(GuardSummary {
        measurement,
        baseline_path,
        baseline_total,
        current_total,
        delta,
        allowed_drop,
        passed,
    })
}

async fn read_baseline(path: &Path) -> Result<BaselineScorecard> {
    let bytes = fs::read(path)
        .await
        .map_err(|source| CliareError::ReadBaselineScorecard {
            path: path.to_path_buf(),
            source,
        })?;
    serde_json::from_slice(&bytes).map_err(|source| CliareError::ParseBaselineScorecard {
        path: path.to_path_buf(),
        source,
    })
}

#[derive(Debug, Deserialize)]
struct BaselineScorecard {
    score: BaselineScore,
}

impl BaselineScorecard {
    fn total(&self) -> Result<f64> {
        if self.score.total.is_finite() && (0.0..=100.0).contains(&self.score.total) {
            Ok(self.score.total)
        } else {
            Err(CliareError::InvalidBaselineScore {
                total: self.score.total,
            })
        }
    }
}

#[derive(Debug, Deserialize)]
struct BaselineScore {
    total: f64,
}

fn allowed_drop(value: f64) -> Result<f64> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(CliareError::InvalidAllowedDrop { value })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::fingerprint::TargetFingerprint;
    use crate::measure::MeasurementSummary;

    use super::GuardSummary;

    #[test]
    fn guard_summary_reports_pass_and_delta() {
        let summary = GuardSummary {
            measurement: measurement_summary(82.0),
            baseline_path: ".cliare/baseline.scorecard.json".into(),
            baseline_total: 80.0,
            current_total: 82.0,
            delta: 2.0,
            allowed_drop: 0.0,
            passed: true,
        };

        let text = summary.terminal_summary();

        assert!(text.contains("result: pass"));
        assert!(text.contains("delta: +2.0"));
    }

    #[test]
    fn guard_summary_reports_failure() {
        let summary = GuardSummary {
            measurement: measurement_summary(77.0),
            baseline_path: ".cliare/baseline.scorecard.json".into(),
            baseline_total: 80.0,
            current_total: 77.0,
            delta: -3.0,
            allowed_drop: 1.0,
            passed: false,
        };

        let text = summary.terminal_summary();

        assert!(text.contains("result: fail"));
        assert!(text.contains("allowed drop: 1.0"));
    }

    #[test]
    fn allowed_drop_rejects_negative_values() {
        let error = super::allowed_drop(-1.0).expect_err("negative drop is invalid");

        assert!(matches!(
            error,
            crate::error::CliareError::InvalidAllowedDrop { value } if value == -1.0
        ));
    }

    fn measurement_summary(score_total: f64) -> MeasurementSummary {
        MeasurementSummary {
            target: TargetFingerprint {
                requested: "tool".into(),
                resolved: "/tmp/tool".into(),
                binary_sha256: "abc".to_owned(),
                size_bytes: 1,
            },
            probes_completed: 7,
            evidence_path: PathBuf::from(".cliare/evidence.jsonl"),
            shape_path: PathBuf::from(".cliare/shape.json"),
            scorecard_path: PathBuf::from(".cliare/scorecard.json"),
            report_path: PathBuf::from(".cliare/report.md"),
            sandbox_profile: "isolated".to_owned(),
            sandbox_root: PathBuf::from(".cliare/sandbox"),
            sandbox_home: PathBuf::from(".cliare/sandbox/home"),
            sandbox_workdir: PathBuf::from(".cliare/sandbox/cwd"),
            sandbox_env_policy: "cleared_with_allowlist".to_owned(),
            score_total,
            score_measured_weight: 0.9,
            score_max_weight: 1.0,
            score_model: "cliare-score-v0".to_owned(),
            score_status: "experimental partial".to_owned(),
            findings: 0,
            observed_max_depth: 1,
            traversal_profile: "standard".to_owned(),
            max_depth: 5,
            max_probes: 256,
            min_expected_value: 150,
            frontier_remaining: 0,
            highest_pending_expected_value: None,
            candidates_skipped_by_depth: 0,
            candidates_skipped_by_convergence: 0,
            probes_skipped_by_budget: 0,
            budget_exhausted: false,
            traversal_stop_reason: "converged".to_owned(),
            traversal_complete: true,
            cache_hit: false,
        }
    }
}
