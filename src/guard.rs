use std::path::{Path, PathBuf};

use serde::Deserialize;
use tokio::fs;

use crate::ci::{self, GuardCiContext};
use crate::cli::{GuardArgs, MeasureArgs};
use crate::error::{CliareError, Result};
use crate::measure::{self, MeasurementSummary};
use crate::policy::{self, PolicyEvaluation};

#[derive(Debug, Clone)]
pub struct GuardSummary {
    pub measurement: MeasurementSummary,
    pub baseline_path: PathBuf,
    pub baseline_total: f64,
    pub current_total: f64,
    pub delta: f64,
    pub allowed_drop: f64,
    pub regression_passed: bool,
    pub policy: Option<PolicyEvaluation>,
    pub passed: bool,
}

impl GuardSummary {
    pub fn terminal_summary(&self) -> String {
        let result = if self.passed { "pass" } else { "fail" };
        let regression = if self.regression_passed {
            "pass"
        } else {
            "fail"
        };
        let mut lines = vec![
            self.measurement.terminal_summary().trim_end().to_owned(),
            "guard:".to_owned(),
            format!("  result: {result}"),
            format!("  score regression: {regression}"),
            format!("  baseline: {}", self.baseline_path.display()),
            format!("  baseline score: {:.1}", self.baseline_total),
            format!("  current score: {:.1}", self.current_total),
            format!("  delta: {:+.1}", self.delta),
            format!("  allowed drop: {:.1}", self.allowed_drop),
        ];
        if let Some(policy) = &self.policy {
            lines.push("policy:".to_owned());
            lines.push(format!(
                "  result: {}",
                if policy.passed { "pass" } else { "fail" }
            ));
            lines.push(format!("  path: {}", policy.policy_path.display()));
            lines.push(format!("  failures: {}", policy.failures.len()));
            for failure in &policy.failures {
                lines.push(format!("  - {}: {}", failure.id, failure.title));
                lines.push(format!("    {}", failure.detail));
            }
        }

        format!("{}\n", lines.join("\n"))
    }
}

pub async fn guard(args: GuardArgs) -> Result<GuardSummary> {
    let baseline = read_baseline(&args.baseline).await?;
    let allowed_drop = allowed_drop(args.allowed_drop)?;
    let baseline_path = args.baseline.clone();
    let mut measurement = measure::measure(MeasureArgs::from(&args)).await?;
    let artifact_dir = measurement
        .scorecard_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| args.out.clone());
    let current_total = measurement.score_total;
    let baseline_total = baseline.total()?;
    let delta = current_total - baseline_total;
    let regression_passed = delta + allowed_drop >= 0.0;
    let policy = match &args.policy {
        Some(policy_path) => Some(policy::evaluate_policy(policy_path, &artifact_dir).await?),
        None => None,
    };
    let passed = regression_passed && policy.as_ref().is_none_or(|policy| policy.passed);
    let ci_artifacts = ci::write_ci_artifacts(
        &artifact_dir,
        Some(&GuardCiContext {
            baseline_path: baseline_path.clone(),
            baseline_total,
            current_total,
            delta,
            allowed_drop,
            regression_passed,
            policy: policy.clone(),
            passed,
        }),
    )
    .await?;
    measurement.set_ci_artifacts(ci_artifacts);

    Ok(GuardSummary {
        measurement,
        baseline_path,
        baseline_total,
        current_total,
        delta,
        allowed_drop,
        regression_passed,
        policy,
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
            regression_passed: true,
            policy: None,
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
            regression_passed: false,
            policy: None,
            passed: false,
        };

        let text = summary.terminal_summary();

        assert!(text.contains("result: fail"));
        assert!(text.contains("score regression: fail"));
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
            job_id: None,
            job_log_path: None,
            probes_completed: 7,
            evidence_path: PathBuf::from(".cliare/evidence.jsonl"),
            shape_path: PathBuf::from(".cliare/shape.json"),
            command_index_json_path: PathBuf::from(".cliare/command-index.json"),
            command_index_markdown_path: PathBuf::from(".cliare/command-index.md"),
            scorecard_path: PathBuf::from(".cliare/scorecard.json"),
            report_path: PathBuf::from(".cliare/report.md"),
            ci_summary_path: PathBuf::from(".cliare/summary.md"),
            sarif_path: PathBuf::from(".cliare/findings.sarif"),
            junit_path: PathBuf::from(".cliare/junit.xml"),
            issues_markdown_path: PathBuf::from(".cliare/issues.md"),
            issues_json_path: PathBuf::from(".cliare/issues.json"),
            persona_report_count: 7,
            readme_path: PathBuf::from(".cliare/README.md"),
            agent_skill_path: PathBuf::from(".cliare/AGENT_SKILL.md"),
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
            commands_precondition_blocked: 0,
            output_contracts_discovered: 0,
            machine_readable_output_contracts: 0,
            output_mode_probes_completed: 0,
            output_mode_parse_successes: 0,
            output_mode_precondition_blocked: 0,
            precondition_blocked_probes: 0,
            auth_required_probes: 0,
            local_context_required_probes: 0,
            fixture_required_probes: 0,
            actionable_precondition_probes: 0,
            precondition_recovery_rate: 0.0,
            side_effect_files_created: 0,
            side_effect_files_modified: 0,
            side_effect_files_deleted: 0,
            side_effect_files_total: 0,
            side_effect_probe_count: 0,
            credential_like_side_effects: 0,
            observed_max_depth: 1,
            traversal_profile: "standard".to_owned(),
            max_depth: 5,
            max_probes: 256,
            min_expected_value: 150,
            concurrency_limit: 4,
            traversal_rounds: 2,
            probes_scheduled: 7,
            probes_cancelled: 0,
            frontier_remaining: 0,
            highest_pending_expected_value: None,
            candidates_skipped_by_depth: 0,
            candidates_skipped_by_convergence: 0,
            probes_skipped_by_budget: 0,
            budget_exhausted: false,
            traversal_stop_reason: "converged".to_owned(),
            traversal_complete: true,
            cache_hit: false,
            runtime_context: crate::context::RuntimeContext::default(),
            suite_root_path: PathBuf::from(".cliare"),
            runtime_context_path: Some(PathBuf::from(".cliare/runtime-context.json")),
            context_suite_path: None,
            context_compare_path: None,
        }
    }
}
