use std::collections::BTreeMap;
use std::path::Path;

use tokio::fs;

use crate::report_format::{command_path_label, shell_arg, shell_words};
use cliare_core::artifacts::ISSUES_JSON;
use cliare_core::error::{CliareError, Result};
use cliare_issues::issue_disposition::{IssueDispositions, disposition_path};

use super::model::{
    IssueCommandProjection, IssueCommandSample, IssueDispositionList, IssueDispositionListItem,
    IssueDispositionSummary, IssueLedgerProjection, IssueProjection, IssueVerificationProjection,
};
use super::{ISSUE_COMMAND_SAMPLE_LIMIT, ISSUES_LIST_SCHEMA_VERSION};

impl IssueDispositionList {
    pub(super) async fn build(
        artifact_dir: &Path,
        dispositions: &IssueDispositions,
    ) -> Result<Self> {
        let issues_source = artifact_dir.join(ISSUES_JSON);
        let issue_rows = read_issue_projection(&issues_source).await?;
        let disposition_by_id = dispositions.by_issue_id();
        let mut rows = BTreeMap::<String, IssueDispositionListItem>::new();

        for issue in issue_rows {
            let disposition = disposition_by_id.get(issue.id.as_str()).cloned().cloned();
            let affected_command_count = issue.affected_commands.len();
            let command_samples = issue
                .affected_commands
                .into_iter()
                .take(ISSUE_COMMAND_SAMPLE_LIMIT)
                .map(IssueCommandSample::from)
                .collect();
            rows.insert(
                issue.id.clone(),
                IssueDispositionListItem {
                    issue_id: issue.id,
                    issue_status: Some(issue.status),
                    title: Some(issue.title),
                    severity: issue.severity,
                    category: issue.category,
                    agent_readiness_area: issue.agent_readiness_area,
                    confidence: issue.confidence,
                    impact: issue.impact,
                    why_it_matters: issue.why_it_matters,
                    recommendation: issue.recommendation,
                    affected_command_count,
                    command_samples,
                    verification: issue
                        .verification
                        .map(|verification| normalize_verification(verification, artifact_dir)),
                    action_required: disposition
                        .as_ref()
                        .is_none_or(|entry| entry.status.is_action_required()),
                    disposition,
                },
            );
        }

        for disposition in &dispositions.dispositions {
            rows.entry(disposition.issue_id.clone())
                .or_insert_with(|| IssueDispositionListItem {
                    issue_id: disposition.issue_id.clone(),
                    issue_status: None,
                    title: None,
                    severity: None,
                    category: None,
                    agent_readiness_area: None,
                    confidence: None,
                    impact: None,
                    why_it_matters: None,
                    recommendation: None,
                    affected_command_count: 0,
                    command_samples: Vec::new(),
                    verification: None,
                    action_required: disposition.status.is_action_required(),
                    disposition: Some(disposition.clone()),
                });
        }

        let issues = rows.into_values().collect::<Vec<_>>();
        let dispositioned = issues
            .iter()
            .filter(|issue| issue.disposition.is_some())
            .count();
        let action_required = issues.iter().filter(|issue| issue.action_required).count();
        let reviewed_decisions = dispositioned.saturating_sub(
            issues
                .iter()
                .filter(|issue| {
                    issue
                        .disposition
                        .as_ref()
                        .is_some_and(|entry| entry.status.is_action_required())
                })
                .count(),
        );

        Ok(Self {
            schema_version: ISSUES_LIST_SCHEMA_VERSION,
            artifact_dir: artifact_dir.to_path_buf(),
            dispositions_path: disposition_path(artifact_dir),
            issues_source: issues_source.exists().then_some(issues_source),
            summary: IssueDispositionSummary {
                issues_total: issues.len(),
                dispositioned,
                action_required,
                reviewed_decisions,
            },
            issues,
        })
    }
}

impl From<IssueCommandProjection> for IssueCommandSample {
    fn from(command: IssueCommandProjection) -> Self {
        let label = if command.argv.is_empty() {
            command_path_label(&command.path)
        } else {
            shell_words(&command.argv)
        };
        Self {
            command: label,
            path: command.path,
            argv: command.argv,
            state: command.state,
            confidence: command.confidence,
            summary: command.summary,
            reason: command.reason,
            required_positionals: command.required_positionals,
        }
    }
}

fn normalize_verification(
    mut verification: IssueVerificationProjection,
    artifact_dir: &Path,
) -> IssueVerificationProjection {
    let out = shell_arg(&artifact_dir.display().to_string());
    verification.command = verification
        .command
        .replace(" --out .cliare ", &format!(" --out {out} "));
    verification
}

async fn read_issue_projection(path: &Path) -> Result<Vec<IssueProjection>> {
    match fs::read(path).await {
        Ok(bytes) => {
            let projection: IssueLedgerProjection =
                serde_json::from_slice(&bytes).map_err(|source| CliareError::ParseIssueLedger {
                    path: path.to_path_buf(),
                    source,
                })?;
            Ok(projection.issues)
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(source) => Err(CliareError::ReadIssueLedger {
            path: path.to_path_buf(),
            source,
        }),
    }
}
