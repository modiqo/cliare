use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Measurement,
    Benchmark,
    ContextSuite,
    Mixed,
    Unknown,
}

impl ArtifactKind {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Measurement => "measurement",
            Self::Benchmark => "benchmark",
            Self::ContextSuite => "context_suite",
            Self::Mixed => "mixed",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    AgentSkill,
    Additional,
    ArtifactMap,
    ArtifactMapReport,
    BenchmarkReport,
    Cache,
    CiSummary,
    CommandIndex,
    CommandIndexReport,
    ContextCompare,
    ContextSuite,
    ContextsDirectory,
    Directory,
    Evidence,
    Guide,
    IssueLedger,
    IssueReport,
    JobPointer,
    JobLog,
    JobStderr,
    JobStdout,
    Junit,
    PersonaOutcome,
    PersonaReport,
    Sarif,
    Sandbox,
    Scorecard,
    ScoreReport,
    Shape,
}

impl FileKind {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::AgentSkill => "agent_skill",
            Self::Additional => "additional",
            Self::ArtifactMap => "artifact_map",
            Self::ArtifactMapReport => "artifact_map_report",
            Self::BenchmarkReport => "benchmark_report",
            Self::Cache => "cache",
            Self::CiSummary => "ci_summary",
            Self::CommandIndex => "command_index",
            Self::CommandIndexReport => "command_index_report",
            Self::ContextCompare => "context_compare",
            Self::ContextSuite => "context_suite",
            Self::ContextsDirectory => "contexts_directory",
            Self::Directory => "directory",
            Self::Evidence => "evidence",
            Self::Guide => "guide",
            Self::IssueLedger => "issue_ledger",
            Self::IssueReport => "issue_report",
            Self::JobPointer => "job_pointer",
            Self::JobLog => "job_log",
            Self::JobStderr => "job_stderr",
            Self::JobStdout => "job_stdout",
            Self::Junit => "junit",
            Self::PersonaOutcome => "persona_outcome",
            Self::PersonaReport => "persona_report",
            Self::Sarif => "sarif",
            Self::Sandbox => "sandbox",
            Self::Scorecard => "scorecard",
            Self::ScoreReport => "score_report",
            Self::Shape => "shape",
        }
    }
}

pub(super) struct FileSpec {
    pub(super) path: String,
    pub(super) kind: FileKind,
    pub(super) role: String,
    pub(super) required: bool,
    pub(super) navigation_rank: u8,
    pub(super) agent_use: String,
}

impl FileSpec {
    pub(super) fn new(
        path: impl Into<String>,
        kind: FileKind,
        role: impl Into<String>,
        required: bool,
        navigation_rank: u8,
        agent_use: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            kind,
            role: role.into(),
            required,
            navigation_rank,
            agent_use: agent_use.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMap {
    pub schema_version: String,
    pub cliare_version: String,
    pub generated_at: String,
    pub folder: String,
    pub artifact_kind: ArtifactKind,
    pub health: ArtifactHealth,
    pub navigation: Vec<NavigationStep>,
    pub summaries: ArtifactSummaries,
    pub missing_required: Vec<String>,
    pub files: Vec<ArtifactFile>,
    pub written_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactHealth {
    pub status: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationStep {
    pub rank: u8,
    pub path: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactFile {
    pub path: String,
    pub kind: FileKind,
    pub role: String,
    pub required: bool,
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub records: Option<u64>,
    pub schema_version: Option<String>,
    pub parse_status: Option<String>,
    pub navigation_rank: u8,
    pub agent_use: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSummaries {
    pub scorecard: Option<ScorecardSummary>,
    pub command_index: Option<CommandIndexSummary>,
    pub issues: Option<IssuesSummary>,
    pub benchmark: Option<BenchmarkSummary>,
    pub contexts: Option<ContextSuiteSummary>,
    pub job: Option<JobSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScorecardSummary {
    pub total: Option<f64>,
    pub status: Option<String>,
    pub model: Option<String>,
    pub probes_completed: Option<u64>,
    pub max_probes: Option<u64>,
    pub traversal_complete: Option<bool>,
    pub budget_exhausted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandIndexSummary {
    pub commands_total: u64,
    pub runtime_states: BTreeMap<String, u64>,
    pub suitability: BTreeMap<String, u64>,
    pub preconditioned: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuesSummary {
    pub issues_total: u64,
    pub severity: BTreeMap<String, u64>,
    pub confidence: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub targets: Option<u64>,
    pub measured: Option<u64>,
    pub skipped: Option<u64>,
    pub failed: Option<u64>,
    pub passed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSuiteSummary {
    pub contexts_total: u64,
    pub contexts: Vec<ContextSummaryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummaryItem {
    pub name: String,
    pub profile: Option<String>,
    pub artifact_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub status: String,
    pub job_id: Option<String>,
    pub progress_log: Option<String>,
    pub stdout_log: Option<String>,
    pub stderr_log: Option<String>,
    pub last_progress: Option<String>,
    pub last_error: Option<String>,
}
