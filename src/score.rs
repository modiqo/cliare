use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use tokio::fs;

use crate::claims::{ClaimSet, CommandClaim};
use crate::error::{CliareError, Result};
use crate::evidence::{ProbeIntent, ProcessStatus};
use crate::fingerprint::TargetFingerprint;
use crate::observation::ShapeObservation;

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
    commands_discovered: usize,
    commands_runtime_confirmed: usize,
    command_confirmation_rate: f64,
    flags_discovered: usize,
    avg_command_confidence: f64,
    avg_flag_confidence: f64,
    probes_completed: usize,
    probes_timed_out: usize,
    probes_failed_to_spawn: usize,
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

pub async fn write_score_artifacts(
    out_dir: &Path,
    target: TargetFingerprint,
    observations: &[ShapeObservation],
) -> Result<()> {
    let scorecard = scorecard(target, observations);
    write_scorecard(out_dir, &scorecard).await?;
    write_report(out_dir, &scorecard).await
}

async fn write_scorecard(out_dir: &Path, scorecard: &Scorecard) -> Result<()> {
    let path = out_dir.join("scorecard.json");
    let bytes = serde_json::to_vec_pretty(&scorecard).map_err(CliareError::SerializeScorecard)?;
    fs::write(&path, bytes)
        .await
        .map_err(|source| CliareError::WriteScorecard { path, source })
}

async fn write_report(out_dir: &Path, scorecard: &Scorecard) -> Result<()> {
    let path = out_dir.join("report.md");
    fs::write(&path, render_report(scorecard))
        .await
        .map_err(|source| CliareError::WriteReport { path, source })
}

pub fn scorecard(target: TargetFingerprint, observations: &[ShapeObservation]) -> Scorecard {
    let binary_name = target_binary_name(&target);
    let claims = ClaimSet::from_observations(&binary_name, observations);
    let metrics = Metrics::from_claims_and_observations(&claims, observations);

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
                70.0 * metrics.coverage.command_confirmation_rate
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
            score: None,
            weight: 0.05,
            status: DimensionStatus::NotMeasured,
            rationale: "machine-readable output probes are not implemented in score v0".to_owned(),
        },
    );
    subscores.insert(
        Dimension::Safety,
        DimensionScore {
            score: None,
            weight: 0.05,
            status: DimensionStatus::NotMeasured,
            rationale: "side-effect and dry-run classification are not implemented in score v0"
                .to_owned(),
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
        measured_weight: round_score(measured_weight),
        max_weight: round_score(max_weight),
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
        + 40.0 * metrics.coverage.avg_flag_confidence
        + 30.0 * (1.0 - grammar_gap_rate)
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
                "{} of {} command candidates were runtime-confirmed.",
                metrics.coverage.commands_runtime_confirmed, metrics.coverage.commands_discovered
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
        "- Command confirmation rate: `{:.1}%`\n",
        scorecard.coverage.command_confirmation_rate * 100.0
    ));
    report.push_str(&format!(
        "- Flags discovered: `{}`\n",
        scorecard.coverage.flags_discovered
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
        "- Probes completed: `{}`\n",
        scorecard.coverage.probes_completed
    ));
    report.push_str(&format!(
        "- Probe timeouts: `{}`\n",
        scorecard.coverage.probes_timed_out
    ));
    report.push_str(&format!(
        "- Probe spawn failures: `{}`\n\n",
        scorecard.coverage.probes_failed_to_spawn
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

fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

#[derive(Debug)]
struct Metrics {
    coverage: Coverage,
    grammar_gap_count: usize,
    invalid_probe_count: usize,
    invalid_probe_rejections: usize,
}

impl Metrics {
    fn from_claims_and_observations(claims: &ClaimSet, observations: &[ShapeObservation]) -> Self {
        let commands = claims.commands().collect::<Vec<_>>();
        let flags = claims.flags().collect::<Vec<_>>();
        let commands_discovered = commands.len();
        let commands_runtime_confirmed = commands
            .iter()
            .filter(|command| command.runtime_confirmed())
            .count();
        let avg_command_confidence = average(commands.iter().map(|command| command.confidence()));
        let avg_flag_confidence = average(flags.iter().map(|flag| flag.confidence()));
        let grammar_gap_count = commands
            .iter()
            .filter(|command| command.runtime_confirmed())
            .map(|command| grammar_gaps_for(command))
            .sum();

        let process_metrics = ProcessMetrics::from_observations(observations);

        Self {
            coverage: Coverage {
                commands_discovered,
                commands_runtime_confirmed,
                command_confirmation_rate: ratio(commands_runtime_confirmed, commands_discovered),
                flags_discovered: flags.len(),
                avg_command_confidence,
                avg_flag_confidence,
                probes_completed: observations.len(),
                probes_timed_out: process_metrics.timed_out,
                probes_failed_to_spawn: process_metrics.failed_to_spawn,
            },
            grammar_gap_count,
            invalid_probe_count: process_metrics.invalid_probe_count,
            invalid_probe_rejections: process_metrics.invalid_probe_rejections,
        }
    }

    fn grammar_gap_rate(&self) -> f64 {
        let possible = self.coverage.commands_runtime_confirmed.saturating_mul(2);
        ratio(self.grammar_gap_count, possible)
    }
}

#[derive(Debug, Default)]
struct ProcessMetrics {
    timed_out: usize,
    failed_to_spawn: usize,
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

            if matches!(
                observation.intent,
                ProbeIntent::InvalidCommand | ProbeIntent::InvalidChild | ProbeIntent::InvalidFlag
            ) {
                metrics.invalid_probe_count += 1;
                if exited_nonzero(&observation.process.status) {
                    metrics.invalid_probe_rejections += 1;
                }
            }
        }

        metrics
    }
}

fn grammar_gaps_for(command: &CommandClaim) -> usize {
    let mut gaps = 2_usize;
    if command.invalid_flag_rejected() {
        gaps = gaps.saturating_sub(1);
    }
    if !command.has_child_candidates() || command.invalid_child_rejected() {
        gaps = gaps.saturating_sub(1);
    }
    gaps
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
    use super::{Dimension, render_report, scorecard};
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

        let weak_score = scorecard(target.clone(), &weak);
        let strong_score = scorecard(target, &strong);

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

        let scorecard = scorecard(target, &observations);

        assert_eq!(dimension_score(&scorecard, Dimension::Recovery), 100.0);
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
        );

        let report = render_report(&scorecard);

        assert!(report.contains("# CLIARE Report"));
        assert!(report.contains("| output | not measured |"));
        assert!(report.contains("experimental partial"));
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
