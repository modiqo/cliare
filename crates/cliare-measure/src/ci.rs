use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::artifacts::{
    CI_SUMMARY_MD, CONDITION_DICTIONARY_CSV, JUNIT_XML, SARIF_JSON, SCORECARD_JSON, write_atomic,
};
use crate::error::{CliareError, Result};
use crate::markdown::MarkdownBuffer;
use crate::policy::PolicyEvaluation;

#[derive(Debug, Clone)]
pub struct CiArtifactSummary {
    pub summary_path: PathBuf,
    pub sarif_path: PathBuf,
    pub junit_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct GuardCiContext {
    pub baseline_path: PathBuf,
    pub baseline_total: f64,
    pub current_total: f64,
    pub delta: f64,
    pub allowed_drop: f64,
    pub regression_passed: bool,
    pub policy: Option<PolicyEvaluation>,
    pub passed: bool,
}

pub async fn write_ci_artifacts(
    out_dir: &Path,
    guard: Option<&GuardCiContext>,
) -> Result<CiArtifactSummary> {
    let scorecard = read_scorecard(out_dir).await?;
    let summary_path = out_dir.join(CI_SUMMARY_MD);
    let sarif_path = out_dir.join(SARIF_JSON);
    let junit_path = out_dir.join(JUNIT_XML);

    write_ci_summary(&summary_path, &scorecard, guard).await?;
    write_sarif(&sarif_path, &scorecard).await?;
    write_junit(&junit_path, &scorecard, guard).await?;

    Ok(CiArtifactSummary {
        summary_path,
        sarif_path,
        junit_path,
    })
}

async fn read_scorecard(out_dir: &Path) -> Result<CiScorecard> {
    let path = out_dir.join(SCORECARD_JSON);
    let bytes = fs::read(&path)
        .await
        .map_err(|source| CliareError::ReadCiScorecard {
            path: path.clone(),
            source,
        })?;
    serde_json::from_slice(&bytes).map_err(|source| CliareError::ParseCiScorecard { path, source })
}

async fn write_ci_summary(
    path: &Path,
    scorecard: &CiScorecard,
    guard: Option<&GuardCiContext>,
) -> Result<()> {
    let mut text = MarkdownBuffer::new();
    text.line(format_args!("# CLIARE CI Summary"));
    text.blank_line();
    text.line(format_args!(
        "| Field | Value |\n|---|---:|\n| Target | `{}` |\n| Resolved | `{}` |\n| Score | {:.0}/100 |\n| Status | `{}` |\n| Findings | {} |\n| Commands discovered | {} |\n| Runtime-confirmed commands | {} |\n| Concurrency limit | {} |\n| Scheduler rounds | {} |\n| Probes scheduled | {} |\n| Machine-readable outputs | {} |\n| Output parse successes | {} |\n| Side-effect file changes | {} |\n| Credential-like side effects | {} |\n| Traversal complete | {} |",
        markdown_escape(&scorecard.target.requested),
        markdown_escape(&scorecard.target.resolved),
        scorecard.score.total,
        markdown_escape(&scorecard.score.status),
        scorecard.findings.len(),
        scorecard.coverage.commands_discovered,
        scorecard.coverage.commands_runtime_confirmed,
        scorecard.coverage.concurrency_limit,
        scorecard.coverage.traversal_rounds,
        scorecard.coverage.probes_scheduled,
        scorecard.coverage.machine_readable_output_contracts,
        scorecard.coverage.output_mode_parse_successes,
        scorecard.coverage.side_effect_files_total,
        scorecard.coverage.credential_like_side_effects,
        scorecard.coverage.traversal_complete
    ));

    if let Some(guard) = guard {
        text.blank_line();
        text.line(format_args!("## Guard"));
        text.blank_line();
        text.line(format_args!(
            "| Field | Value |\n|---|---:|\n| Result | {} |\n| Baseline | `{}` |\n| Baseline score | {:.0} |\n| Current score | {:.0} |\n| Delta | {:+.1} |\n| Allowed drop | {:.1} |",
            if guard.passed { "pass" } else { "fail" },
            markdown_escape(&guard.baseline_path.display().to_string()),
            guard.baseline_total,
            guard.current_total,
            guard.delta,
            guard.allowed_drop
        ));
        if let Some(policy) = &guard.policy {
            text.blank_line();
            text.line(format_args!("## Policy"));
            text.blank_line();
            text.line(format_args!(
                "| Field | Value |\n|---|---:|\n| Result | {} |\n| Policy | `{}` |\n| Failures | {} |",
                if policy.passed { "pass" } else { "fail" },
                markdown_escape(&policy.policy_path.display().to_string()),
                policy.failures.len()
            ));
            if !policy.failures.is_empty() {
                text.blank_line();
                text.line(format_args!("| Rule | Failure | Detail |\n|---|---|---|"));
                for failure in &policy.failures {
                    text.line(format_args!(
                        "| `{}` | {} | {} |",
                        markdown_escape(&failure.id),
                        markdown_escape(&failure.title),
                        markdown_escape(&failure.detail)
                    ));
                }
            }
        }
    }

    text.blank_line();
    text.line(format_args!("## Subscores"));
    text.blank_line();
    text.line(format_args!(
        "| Dimension | Score | Weight | Status | Rationale |\n|---|---:|---:|---|---|"
    ));
    for (dimension, subscore) in &scorecard.subscores {
        let score = subscore
            .score
            .map(|value| format!("{value:.0}"))
            .unwrap_or_else(|| "n/a".to_owned());
        text.line(format_args!(
            "| `{}` | {} | {:.2} | `{}` | {} |",
            markdown_escape(dimension),
            score,
            subscore.weight,
            markdown_escape(&subscore.status),
            markdown_escape(&subscore.rationale)
        ));
    }

    text.blank_line();
    text.line(format_args!("## Findings"));
    text.blank_line();
    if scorecard.findings.is_empty() {
        text.line(format_args!("No findings."));
    } else {
        text.line(format_args!(
            "| Severity | Dimension | Finding | Recommendation |\n|---|---|---|---|"
        ));
        for finding in &scorecard.findings {
            text.line(format_args!(
                "| `{}` | `{}` | {} | {} |",
                markdown_escape(&finding.severity),
                markdown_escape(&finding.dimension),
                markdown_escape(&finding.title),
                markdown_escape(&finding.recommendation)
            ));
        }
    }

    text.blank_line();
    text.line(format_args!("## Reference"));
    text.blank_line();
    text.line(format_args!(
        "- `{}` decodes issue confidence, command suitability, preconditions, output statuses, shape gaps, traversal reasons, and agent-navigation metrics.",
        CONDITION_DICTIONARY_CSV
    ));

    text.blank_line();
    text.line(format_args!("## Artifacts"));
    text.blank_line();
    text.line(format_args!(
        "- `scorecard.json`\n- `shape.json`\n- `command-index.json`\n- `command-index.md`\n- `{}`\n- `evidence.jsonl`\n- `report.md`\n- `issues.json`\n- `persona-*.md`\n- `findings.sarif`\n- `junit.xml`",
        CONDITION_DICTIONARY_CSV
    ));

    let text = text.into_string();
    write_atomic(path, text.as_bytes())
        .await
        .map_err(|source| CliareError::WriteCiSummary {
            path: path.to_path_buf(),
            source,
        })
}

async fn write_sarif(path: &Path, scorecard: &CiScorecard) -> Result<()> {
    let mut seen_rules = BTreeSet::new();
    let rules = scorecard
        .findings
        .iter()
        .filter(|finding| seen_rules.insert(finding.id.clone()))
        .map(|finding| {
            json!({
                "id": finding.id,
                "name": finding.title,
                "shortDescription": {
                    "text": finding.title,
                },
                "fullDescription": {
                    "text": finding.detail,
                },
                "help": {
                    "text": finding.recommendation,
                },
                "properties": {
                    "dimension": finding.dimension,
                    "severity": finding.severity,
                },
            })
        })
        .collect::<Vec<_>>();
    let results = scorecard
        .findings
        .iter()
        .map(|finding| {
            json!({
                "ruleId": finding.id,
                "level": sarif_level(&finding.severity),
                "message": {
                    "text": finding.detail,
                },
                "properties": {
                    "dimension": finding.dimension,
                    "recommendation": finding.recommendation,
                    "target": scorecard.target.requested,
                },
            })
        })
        .collect::<Vec<_>>();
    let sarif = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "CLIARE",
                        "informationUri": "https://github.com/modiqo/cliare",
                        "semanticVersion": env!("CARGO_PKG_VERSION"),
                        "rules": rules,
                    }
                },
                "automationDetails": {
                    "id": "cliare/agent-readiness",
                },
                "results": results,
                "properties": {
                    "score": scorecard.score.total,
                    "maintainer_readiness": scorecard.score.maintainer_readiness,
                    "shape_confidence": scorecard.score.shape_confidence,
                    "measured_weight": scorecard.score.measured_weight,
                    "max_weight": scorecard.score.max_weight,
                    "model": scorecard.score.model,
                    "target": scorecard.target.requested,
                    "resolved": scorecard.target.resolved,
                    "binary_sha256": scorecard.target.binary_sha256,
                }
            }
        ]
    });
    let bytes = serde_json::to_vec_pretty(&sarif).map_err(CliareError::SerializeSarif)?;
    write_atomic(path, &bytes)
        .await
        .map_err(|source| CliareError::WriteSarif {
            path: path.to_path_buf(),
            source,
        })
}

async fn write_junit(
    path: &Path,
    scorecard: &CiScorecard,
    guard: Option<&GuardCiContext>,
) -> Result<()> {
    let mut cases = Vec::new();
    let mut failures = 0_usize;

    if scorecard.findings.is_empty() {
        cases.push(TestCase::passed("cliare.findings", "no_findings"));
    } else {
        for finding in &scorecard.findings {
            failures += 1;
            cases.push(TestCase::failed(
                "cliare.finding",
                &finding.id,
                &finding.severity,
                &finding.title,
                &format!(
                    "{}\n\nRecommendation: {}",
                    finding.detail, finding.recommendation
                ),
            ));
        }
    }

    if let Some(guard) = guard {
        if guard.regression_passed {
            cases.push(TestCase::passed("cliare.guard", "score_regression"));
        } else {
            failures += 1;
            cases.push(TestCase::failed(
                "cliare.guard",
                "score_regression",
                "score_regression",
                "CLIARE score regression exceeded allowed drop",
                &format!(
                    "Baseline: {:.0}\nCurrent: {:.0}\nDelta: {:+.1}\nAllowed drop: {:.1}\nBaseline path: {}",
                    guard.baseline_total,
                    guard.current_total,
                    guard.delta,
                    guard.allowed_drop,
                    guard.baseline_path.display()
                ),
            ));
        }

        if let Some(policy) = &guard.policy {
            if policy.passed {
                cases.push(TestCase::passed("cliare.policy", "policy"));
            } else {
                for failure in &policy.failures {
                    failures += 1;
                    cases.push(TestCase::failed(
                        "cliare.policy",
                        &failure.id,
                        "policy_failure",
                        &failure.title,
                        &format!(
                            "{}\n\nRecommendation: {}",
                            failure.detail, failure.recommendation
                        ),
                    ));
                }
            }
        }
    }

    let mut xml = String::new();
    writeln!(&mut xml, r#"<?xml version="1.0" encoding="UTF-8"?>"#)
        .expect("writing to string cannot fail");
    writeln!(
        &mut xml,
        r#"<testsuite name="cliare" tests="{}" failures="{}" errors="0" skipped="0">"#,
        cases.len(),
        failures
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut xml,
        "  <properties><property name=\"score\" value=\"{:.0}\" /></properties>",
        scorecard.score.total
    )
    .expect("writing to string cannot fail");
    for case in cases {
        case.write_xml(&mut xml);
    }
    writeln!(
        &mut xml,
        "  <system-out>{}</system-out>",
        xml_escape(&format!(
            "score={:.0}; findings={}; traversal_complete={}",
            scorecard.score.total,
            scorecard.findings.len(),
            scorecard.coverage.traversal_complete
        ))
    )
    .expect("writing to string cannot fail");
    writeln!(&mut xml, "</testsuite>").expect("writing to string cannot fail");

    write_atomic(path, xml.as_bytes())
        .await
        .map_err(|source| CliareError::WriteJunit {
            path: path.to_path_buf(),
            source,
        })
}

fn sarif_level(severity: &str) -> &'static str {
    match severity {
        "high" => "error",
        "medium" => "warning",
        _ => "note",
    }
}

fn markdown_escape(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[derive(Debug)]
struct TestCase {
    class_name: String,
    name: String,
    failure: Option<TestFailure>,
}

impl TestCase {
    fn passed(class_name: &str, name: &str) -> Self {
        Self {
            class_name: class_name.to_owned(),
            name: name.to_owned(),
            failure: None,
        }
    }

    fn failed(class_name: &str, name: &str, kind: &str, message: &str, detail: &str) -> Self {
        Self {
            class_name: class_name.to_owned(),
            name: name.to_owned(),
            failure: Some(TestFailure {
                kind: kind.to_owned(),
                message: message.to_owned(),
                detail: detail.to_owned(),
            }),
        }
    }

    fn write_xml(&self, xml: &mut String) {
        if let Some(failure) = &self.failure {
            writeln!(
                xml,
                "  <testcase classname=\"{}\" name=\"{}\"><failure type=\"{}\" message=\"{}\">{}</failure></testcase>",
                xml_escape(&self.class_name),
                xml_escape(&self.name),
                xml_escape(&failure.kind),
                xml_escape(&failure.message),
                xml_escape(&failure.detail)
            )
            .expect("writing to string cannot fail");
        } else {
            writeln!(
                xml,
                "  <testcase classname=\"{}\" name=\"{}\" />",
                xml_escape(&self.class_name),
                xml_escape(&self.name)
            )
            .expect("writing to string cannot fail");
        }
    }
}

#[derive(Debug)]
struct TestFailure {
    kind: String,
    message: String,
    detail: String,
}

#[derive(Debug, Deserialize)]
struct CiScorecard {
    target: CiTarget,
    score: CiScore,
    subscores: BTreeMap<String, CiDimensionScore>,
    coverage: CiCoverage,
    findings: Vec<CiFinding>,
}

#[derive(Debug, Deserialize)]
struct CiTarget {
    requested: String,
    resolved: String,
    binary_sha256: String,
}

#[derive(Debug, Deserialize)]
struct CiScore {
    total: f64,
    #[serde(default)]
    maintainer_readiness: f64,
    #[serde(default)]
    shape_confidence: f64,
    measured_weight: f64,
    max_weight: f64,
    model: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct CiDimensionScore {
    score: Option<f64>,
    weight: f64,
    status: String,
    rationale: String,
}

#[derive(Debug, Deserialize)]
struct CiCoverage {
    commands_discovered: usize,
    commands_runtime_confirmed: usize,
    concurrency_limit: usize,
    traversal_rounds: usize,
    probes_scheduled: usize,
    machine_readable_output_contracts: usize,
    output_mode_parse_successes: usize,
    side_effect_files_total: usize,
    credential_like_side_effects: usize,
    traversal_complete: bool,
}

#[derive(Debug, Deserialize)]
struct CiFinding {
    id: String,
    dimension: String,
    severity: String,
    title: String,
    detail: String,
    recommendation: String,
}

#[cfg(test)]
mod tests {
    use super::{markdown_escape, sarif_level, xml_escape};

    #[test]
    fn sarif_levels_follow_finding_severity() {
        assert_eq!(sarif_level("high"), "error");
        assert_eq!(sarif_level("medium"), "warning");
        assert_eq!(sarif_level("low"), "note");
    }

    #[test]
    fn escapes_markdown_tables_and_xml_attributes() {
        assert_eq!(markdown_escape("a|b\nc"), "a\\|b c");
        assert_eq!(
            xml_escape("<tag attr=\"a&b\">'x'</tag>"),
            "&lt;tag attr=&quot;a&amp;b&quot;&gt;&apos;x&apos;&lt;/tag&gt;"
        );
    }
}
