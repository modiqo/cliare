use std::path::{Path, PathBuf};

pub const AGENT_SKILL_MD: &str = "AGENT_SKILL.md";
pub const CI_SUMMARY_MD: &str = "summary.md";
pub const COMMAND_INDEX_JSON: &str = "command-index.json";
pub const COMMAND_INDEX_MD: &str = "command-index.md";
pub const CONTEXT_COMPARE_MD: &str = "context-compare.md";
pub const CONTEXT_SUITE_JSON: &str = "context-suite.json";
pub const EVIDENCE_JSONL: &str = "evidence.jsonl";
pub const JUNIT_XML: &str = "junit.xml";
pub const ISSUES_JSON: &str = "issues.json";
pub const ISSUES_MD: &str = "issues.md";
pub const ISSUE_DISPOSITIONS_JSON: &str = "issue-dispositions.json";
pub const README_MD: &str = "README.md";
pub const REPORT_MD: &str = "report.md";
pub const RUNTIME_CONTEXT_JSON: &str = "runtime-context.json";
pub const SARIF_JSON: &str = "findings.sarif";
pub const SCORECARD_JSON: &str = "scorecard.json";
pub const SHAPE_JSON: &str = "shape.json";

pub const REQUIRED_MEASUREMENT_FILES: &[&str] = &[
    EVIDENCE_JSONL,
    SHAPE_JSON,
    COMMAND_INDEX_JSON,
    COMMAND_INDEX_MD,
    SCORECARD_JSON,
    REPORT_MD,
    CI_SUMMARY_MD,
    SARIF_JSON,
    JUNIT_XML,
];

#[derive(Debug, Clone)]
pub struct MeasurementArtifactPaths {
    pub evidence: PathBuf,
    pub shape: PathBuf,
    pub command_index_json: PathBuf,
    pub command_index_markdown: PathBuf,
    pub scorecard: PathBuf,
    pub report: PathBuf,
    pub ci_summary: PathBuf,
    pub sarif: PathBuf,
    pub junit: PathBuf,
    pub issues_markdown: PathBuf,
    pub issues_json: PathBuf,
    pub readme: PathBuf,
    pub agent_skill: PathBuf,
    pub runtime_context: PathBuf,
}

impl MeasurementArtifactPaths {
    pub fn from_dir(dir: &Path) -> Self {
        Self {
            evidence: dir.join(EVIDENCE_JSONL),
            shape: dir.join(SHAPE_JSON),
            command_index_json: dir.join(COMMAND_INDEX_JSON),
            command_index_markdown: dir.join(COMMAND_INDEX_MD),
            scorecard: dir.join(SCORECARD_JSON),
            report: dir.join(REPORT_MD),
            ci_summary: dir.join(CI_SUMMARY_MD),
            sarif: dir.join(SARIF_JSON),
            junit: dir.join(JUNIT_XML),
            issues_markdown: dir.join(ISSUES_MD),
            issues_json: dir.join(ISSUES_JSON),
            readme: dir.join(README_MD),
            agent_skill: dir.join(AGENT_SKILL_MD),
            runtime_context: dir.join(RUNTIME_CONTEXT_JSON),
        }
    }
}
