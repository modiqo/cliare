use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use cliare_issues::issue_disposition::IssueDisposition;

#[derive(Debug, Serialize)]
pub(super) struct IssueDispositionList {
    pub(super) schema_version: &'static str,
    pub(super) artifact_dir: PathBuf,
    pub(super) dispositions_path: PathBuf,
    pub(super) issues_source: Option<PathBuf>,
    pub(super) summary: IssueDispositionSummary,
    pub(super) issues: Vec<IssueDispositionListItem>,
}

#[derive(Debug, Serialize)]
pub(super) struct IssueDispositionSummary {
    pub(super) issues_total: usize,
    pub(super) dispositioned: usize,
    pub(super) action_required: usize,
    pub(super) reviewed_decisions: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct IssueDispositionListItem {
    pub(super) issue_id: String,
    pub(super) issue_status: Option<String>,
    pub(super) title: Option<String>,
    pub(super) severity: Option<String>,
    pub(super) category: Option<String>,
    pub(super) agent_readiness_area: Option<String>,
    pub(super) confidence: Option<String>,
    pub(super) impact: Option<String>,
    pub(super) why_it_matters: Option<String>,
    pub(super) recommendation: Option<String>,
    pub(super) affected_command_count: usize,
    pub(super) command_samples: Vec<IssueCommandSample>,
    pub(super) verification: Option<IssueVerificationProjection>,
    pub(super) disposition: Option<IssueDisposition>,
    pub(super) action_required: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct IssueCommandSample {
    pub(super) command: String,
    pub(super) path: Vec<String>,
    pub(super) argv: Vec<String>,
    pub(super) state: String,
    pub(super) confidence: Option<f64>,
    pub(super) summary: Option<String>,
    pub(super) reason: String,
    pub(super) required_positionals: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct IssueLedgerProjection {
    pub(super) issues: Vec<IssueProjection>,
}

#[derive(Debug, Deserialize)]
pub(super) struct IssueProjection {
    pub(super) id: String,
    pub(super) status: String,
    pub(super) title: String,
    pub(super) severity: Option<String>,
    pub(super) category: Option<String>,
    pub(super) agent_readiness_area: Option<String>,
    pub(super) confidence: Option<String>,
    pub(super) impact: Option<String>,
    pub(super) why_it_matters: Option<String>,
    pub(super) recommendation: Option<String>,
    pub(super) verification: Option<IssueVerificationProjection>,
    #[serde(default)]
    pub(super) affected_commands: Vec<IssueCommandProjection>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct IssueVerificationProjection {
    pub(super) command: String,
    pub(super) expected_change: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct IssueCommandProjection {
    pub(super) path: Vec<String>,
    #[serde(default)]
    pub(super) argv: Vec<String>,
    pub(super) state: String,
    pub(super) confidence: Option<f64>,
    pub(super) summary: Option<String>,
    #[serde(default)]
    pub(super) reason: String,
    #[serde(default)]
    pub(super) required_positionals: Vec<String>,
}
