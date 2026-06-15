use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::artifacts::ISSUES_JSON;
use crate::cli::{IssuesArgs, IssuesCommand, IssuesListArgs, IssuesListFormat, IssuesMarkArgs};
use crate::context;
use crate::error::{CliareError, Result};
use crate::issue_disposition::{IssueDisposition, IssueDispositions, disposition_path};
use crate::report_format::escape_markdown;

const ISSUES_LIST_SCHEMA_VERSION: &str = "cliare.issue-disposition-list.v1";

#[derive(Debug, Clone)]
pub struct IssuesSummary {
    artifact_dir: PathBuf,
    dispositions_path: PathBuf,
    stdout: String,
}

impl IssuesSummary {
    pub fn terminal_summary(&self) -> &str {
        &self.stdout
    }

    pub fn artifact_dir(&self) -> &Path {
        &self.artifact_dir
    }

    pub fn dispositions_path(&self) -> &Path {
        &self.dispositions_path
    }
}

pub async fn issues(args: IssuesArgs) -> Result<IssuesSummary> {
    match args.command {
        IssuesCommand::Mark(args) => mark(args).await,
        IssuesCommand::List(args) => list(args).await,
    }
}

async fn mark(args: IssuesMarkArgs) -> Result<IssuesSummary> {
    let artifact_dir =
        context::resolve_measurement_dir(&args.out, args.context.as_deref(), "cliare issues mark")
            .await?;
    let issue_id = args.issue_id;
    let status = args.status;
    let mut dispositions = IssueDispositions::read_optional(&artifact_dir).await?;
    dispositions.mark(issue_id.clone(), status, args.reason);
    let dispositions_path = dispositions.write(&artifact_dir).await?;
    let stdout = render_mark_summary(&artifact_dir, &dispositions_path, &issue_id, status);

    Ok(IssuesSummary {
        artifact_dir,
        dispositions_path,
        stdout,
    })
}

async fn list(args: IssuesListArgs) -> Result<IssuesSummary> {
    let artifact_dir =
        context::resolve_measurement_dir(&args.out, args.context.as_deref(), "cliare issues list")
            .await?;
    let dispositions = IssueDispositions::read_optional(&artifact_dir).await?;
    let packet = IssueDispositionList::build(&artifact_dir, &dispositions).await?;
    let dispositions_path = disposition_path(&artifact_dir);
    let stdout = match args.format {
        IssuesListFormat::Markdown => render_list_markdown(&packet),
        IssuesListFormat::Json => {
            format!(
                "{}\n",
                serde_json::to_string_pretty(&packet)
                    .map_err(CliareError::SerializeIssueDispositions)?
            )
        }
    };

    Ok(IssuesSummary {
        artifact_dir,
        dispositions_path,
        stdout,
    })
}

#[derive(Debug, Serialize)]
struct IssueDispositionList {
    schema_version: &'static str,
    artifact_dir: PathBuf,
    dispositions_path: PathBuf,
    issues_source: Option<PathBuf>,
    summary: IssueDispositionSummary,
    issues: Vec<IssueDispositionListItem>,
}

#[derive(Debug, Serialize)]
struct IssueDispositionSummary {
    issues_total: usize,
    dispositioned: usize,
    action_required: usize,
    reviewed_decisions: usize,
}

#[derive(Debug, Serialize)]
struct IssueDispositionListItem {
    issue_id: String,
    issue_status: Option<String>,
    title: Option<String>,
    disposition: Option<IssueDisposition>,
    action_required: bool,
}

#[derive(Debug, Deserialize)]
struct IssueLedgerProjection {
    issues: Vec<IssueProjection>,
}

#[derive(Debug, Deserialize)]
struct IssueProjection {
    id: String,
    status: String,
    title: String,
}

impl IssueDispositionList {
    async fn build(artifact_dir: &Path, dispositions: &IssueDispositions) -> Result<Self> {
        let issues_source = artifact_dir.join(ISSUES_JSON);
        let issue_rows = read_issue_projection(&issues_source).await?;
        let disposition_by_id = dispositions.by_issue_id();
        let mut rows = BTreeMap::<String, IssueDispositionListItem>::new();

        for issue in issue_rows {
            let disposition = disposition_by_id.get(issue.id.as_str()).cloned().cloned();
            rows.insert(
                issue.id.clone(),
                IssueDispositionListItem {
                    issue_id: issue.id,
                    issue_status: Some(issue.status),
                    title: Some(issue.title),
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

fn render_mark_summary(
    artifact_dir: &Path,
    dispositions_path: &Path,
    issue_id: &str,
    status: crate::issue_disposition::IssueDispositionStatus,
) -> String {
    format!(
        "Recorded issue disposition: `{}` -> `{}`\nArtifact dir: `{}`\nDispositions: `{}`\n",
        escape_markdown(issue_id),
        status.label(),
        artifact_dir.display(),
        dispositions_path.display()
    )
}

fn render_list_markdown(packet: &IssueDispositionList) -> String {
    let mut text = String::new();
    writeln!(&mut text, "# CLIARE Issue Dispositions").expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Artifact dir: `{}`",
        packet.artifact_dir.display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Dispositions: `{}`",
        packet.dispositions_path.display()
    )
    .expect("writing to string cannot fail");
    writeln!(&mut text, "- Issues: `{}`", packet.summary.issues_total)
        .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Dispositioned: `{}`",
        packet.summary.dispositioned
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Action required: `{}`",
        packet.summary.action_required
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Reviewed decisions: `{}`",
        packet.summary.reviewed_decisions
    )
    .expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "| Issue | CLIARE Status | Disposition | Action Required | Reason |"
    )
    .expect("writing to string cannot fail");
    writeln!(&mut text, "|---|---|---|---|---|").expect("writing to string cannot fail");
    for issue in &packet.issues {
        let disposition = issue
            .disposition
            .as_ref()
            .map(|entry| entry.status.label())
            .unwrap_or("open");
        let reason = issue
            .disposition
            .as_ref()
            .map(|entry| entry.reason.as_str())
            .unwrap_or("");
        let title = issue
            .title
            .as_ref()
            .map(|title| format!(" {}", escape_markdown(title)))
            .unwrap_or_default();
        writeln!(
            &mut text,
            "| `{}`{} | `{}` | `{}` | `{}` | {} |",
            escape_markdown(&issue.issue_id),
            title,
            escape_markdown(issue.issue_status.as_deref().unwrap_or("unknown")),
            disposition,
            issue.action_required,
            escape_markdown(reason)
        )
        .expect("writing to string cannot fail");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::{IssueDispositionList, render_list_markdown};
    use crate::issue_disposition::{IssueDispositionStatus, IssueDispositions, disposition_path};

    #[tokio::test]
    async fn list_includes_dispositions_without_issue_ledger() {
        let root =
            std::env::temp_dir().join(format!("cliare-issues-list-test-{}", std::process::id()));
        tokio::fs::create_dir_all(&root)
            .await
            .expect("creates temp issue dir");
        let mut dispositions = IssueDispositions::default();
        dispositions.mark(
            "issue.test".to_owned(),
            IssueDispositionStatus::Intentional,
            "deliberate".to_owned(),
        );

        let packet = IssueDispositionList::build(&root, &dispositions)
            .await
            .expect("list packet builds");
        let markdown = render_list_markdown(&packet);

        assert_eq!(packet.dispositions_path, disposition_path(&root));
        assert_eq!(packet.summary.issues_total, 1);
        assert_eq!(packet.summary.reviewed_decisions, 1);
        assert!(markdown.contains("issue.test"));
        assert!(markdown.contains("intentional"));
    }
}
