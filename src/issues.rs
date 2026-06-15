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
use crate::report_format::{command_path_label, escape_markdown, shell_arg, shell_words};

const ISSUES_LIST_SCHEMA_VERSION: &str = "cliare.issue-list.v2";
const ISSUE_COMMAND_SAMPLE_LIMIT: usize = 5;

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
        IssuesListFormat::Human => render_list_human(&packet),
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
    severity: Option<String>,
    category: Option<String>,
    agent_readiness_area: Option<String>,
    confidence: Option<String>,
    impact: Option<String>,
    why_it_matters: Option<String>,
    recommendation: Option<String>,
    affected_command_count: usize,
    command_samples: Vec<IssueCommandSample>,
    verification: Option<IssueVerificationProjection>,
    disposition: Option<IssueDisposition>,
    action_required: bool,
}

#[derive(Debug, Clone, Serialize)]
struct IssueCommandSample {
    command: String,
    path: Vec<String>,
    argv: Vec<String>,
    state: String,
    confidence: Option<f64>,
    summary: Option<String>,
    reason: String,
    required_positionals: Vec<String>,
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
    severity: Option<String>,
    category: Option<String>,
    agent_readiness_area: Option<String>,
    confidence: Option<String>,
    impact: Option<String>,
    why_it_matters: Option<String>,
    recommendation: Option<String>,
    verification: Option<IssueVerificationProjection>,
    #[serde(default)]
    affected_commands: Vec<IssueCommandProjection>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct IssueVerificationProjection {
    command: String,
    expected_change: String,
}

#[derive(Debug, Deserialize)]
struct IssueCommandProjection {
    path: Vec<String>,
    #[serde(default)]
    argv: Vec<String>,
    state: String,
    confidence: Option<f64>,
    summary: Option<String>,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    required_positionals: Vec<String>,
}

impl IssueDispositionList {
    async fn build(artifact_dir: &Path, dispositions: &IssueDispositions) -> Result<Self> {
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
        "| Issue | Area | Affected Commands | What It Means | Next | Disposition |"
    )
    .expect("writing to string cannot fail");
    writeln!(&mut text, "|---|---|---|---|---|---|").expect("writing to string cannot fail");
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
        writeln!(
            &mut text,
            "| {} | {} | {} | {} | {} | {} |",
            issue_markdown_label(issue),
            issue_area_label(issue),
            issue_commands_label(issue),
            issue_meaning_label(issue),
            issue_next_label(issue),
            issue_disposition_label(disposition, reason),
        )
        .expect("writing to string cannot fail");
    }
    text
}

fn render_list_human(packet: &IssueDispositionList) -> String {
    let mut text = String::new();
    writeln!(&mut text, "CLIARE issues").expect("writing to string cannot fail");
    writeln!(&mut text, "Artifact: {}", packet.artifact_dir.display())
        .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "Review state: {} total, {} action required, {} dispositioned, {} reviewed",
        packet.summary.issues_total,
        packet.summary.action_required,
        packet.summary.dispositioned,
        packet.summary.reviewed_decisions
    )
    .expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");

    if packet.issues.is_empty() {
        writeln!(&mut text, "No issues found.").expect("writing to string cannot fail");
        return text;
    }

    let action_required = packet
        .issues
        .iter()
        .filter(|issue| issue.action_required)
        .collect::<Vec<_>>();
    let reviewed = packet
        .issues
        .iter()
        .filter(|issue| !issue.action_required)
        .collect::<Vec<_>>();

    render_human_issue_group(&mut text, "Action required", &action_required);
    if !reviewed.is_empty() {
        writeln!(&mut text).expect("writing to string cannot fail");
        render_human_issue_group(&mut text, "Reviewed or muted", &reviewed);
    }

    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(&mut text, "Disposition examples:").expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "  cliare issues mark <issue-id> --out {} --status intentional --reason \"Documented and expected.\"",
        shell_arg(&packet.artifact_dir.display().to_string())
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "  cliare issues mark <issue-id> --out {} --status needs-fixture --reason \"Requires safe sample operands.\"",
        shell_arg(&packet.artifact_dir.display().to_string())
    )
    .expect("writing to string cannot fail");

    text
}

fn render_human_issue_group(
    text: &mut String,
    heading: &str,
    issues: &[&IssueDispositionListItem],
) {
    writeln!(text, "{} ({})", heading, issues.len()).expect("writing to string cannot fail");
    if issues.is_empty() {
        writeln!(text, "  none").expect("writing to string cannot fail");
        return;
    }

    for (index, issue) in issues.iter().enumerate() {
        render_human_issue(text, index + 1, issue);
    }
}

fn render_human_issue(text: &mut String, index: usize, issue: &IssueDispositionListItem) {
    let title = issue.title.as_deref().unwrap_or("Untitled issue");
    let area = issue
        .agent_readiness_area
        .as_deref()
        .or(issue.category.as_deref())
        .unwrap_or("unknown");
    let severity = issue.severity.as_deref().unwrap_or("unknown");
    let confidence = issue.confidence.as_deref().unwrap_or("unknown");
    let disposition = issue
        .disposition
        .as_ref()
        .map(|entry| entry.status.label())
        .unwrap_or("open");

    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "{}. {} [{}]", index, title, issue.issue_id)
        .expect("writing to string cannot fail");
    writeln!(
        text,
        "   Area: {} | Severity: {} | Confidence: {} | Disposition: {}",
        area, severity, confidence, disposition
    )
    .expect("writing to string cannot fail");

    if let Some(impact) = issue.impact.as_deref().or(issue.why_it_matters.as_deref()) {
        writeln!(text, "   Meaning: {}", impact).expect("writing to string cannot fail");
    }
    if let Some(recommendation) = &issue.recommendation {
        writeln!(text, "   Next: {}", recommendation).expect("writing to string cannot fail");
    }
    if let Some(verification) = &issue.verification {
        writeln!(text, "   Verify: {}", verification.command)
            .expect("writing to string cannot fail");
    }
    if let Some(disposition) = &issue.disposition
        && !disposition.reason.trim().is_empty()
    {
        writeln!(text, "   Reason: {}", disposition.reason).expect("writing to string cannot fail");
    }

    render_human_commands(text, issue);
}

fn render_human_commands(text: &mut String, issue: &IssueDispositionListItem) {
    if issue.affected_command_count == 0 {
        return;
    }

    writeln!(
        text,
        "   Commands: {} affected{}",
        issue.affected_command_count,
        if issue.affected_command_count > issue.command_samples.len() {
            ", sample below"
        } else {
            ""
        }
    )
    .expect("writing to string cannot fail");

    for command in &issue.command_samples {
        let summary = command
            .summary
            .as_deref()
            .filter(|summary| !summary.trim().is_empty())
            .or_else(|| (!command.reason.trim().is_empty()).then_some(command.reason.as_str()));
        match summary {
            Some(summary) => writeln!(text, "     - {}: {}", command.command, summary),
            None => writeln!(text, "     - {}", command.command),
        }
        .expect("writing to string cannot fail");
    }

    let hidden = issue
        .affected_command_count
        .saturating_sub(issue.command_samples.len());
    if hidden > 0 {
        writeln!(text, "     - ... {} more", hidden).expect("writing to string cannot fail");
    }
}

fn issue_markdown_label(issue: &IssueDispositionListItem) -> String {
    let title = issue.title.as_deref().unwrap_or("Untitled issue");
    let status = issue.issue_status.as_deref().unwrap_or("unknown");
    format!(
        "`{}`<br>{}<br>Status: `{}`",
        escape_markdown(&issue.issue_id),
        escape_markdown(title),
        escape_markdown(status)
    )
}

fn issue_area_label(issue: &IssueDispositionListItem) -> String {
    let severity = issue.severity.as_deref().unwrap_or("unknown");
    let confidence = issue.confidence.as_deref().unwrap_or("unknown");
    let area = issue
        .agent_readiness_area
        .as_deref()
        .or(issue.category.as_deref())
        .unwrap_or("unknown");
    format!(
        "`{}`<br>severity: `{}`<br>confidence: `{}`",
        escape_markdown(area),
        escape_markdown(severity),
        escape_markdown(confidence)
    )
}

fn issue_commands_label(issue: &IssueDispositionListItem) -> String {
    if issue.affected_command_count == 0 {
        return "none listed".to_owned();
    }
    let mut labels = issue
        .command_samples
        .iter()
        .map(|command| {
            let summary = command.summary.as_deref().unwrap_or(&command.reason);
            format!(
                "`{}`: {}",
                escape_markdown(&command.command),
                escape_markdown(summary)
            )
        })
        .collect::<Vec<_>>();
    if issue.affected_command_count > issue.command_samples.len() {
        labels.push(format!(
            "... {} more",
            issue
                .affected_command_count
                .saturating_sub(issue.command_samples.len())
        ));
    }
    labels.join("<br>")
}

fn issue_meaning_label(issue: &IssueDispositionListItem) -> String {
    issue
        .impact
        .as_deref()
        .or(issue.why_it_matters.as_deref())
        .map(escape_markdown)
        .unwrap_or_else(|| "See issue detail.".to_owned())
}

fn issue_next_label(issue: &IssueDispositionListItem) -> String {
    match (&issue.recommendation, &issue.verification) {
        (Some(recommendation), Some(verification)) => format!(
            "{}<br>Verify: `{}`",
            escape_markdown(recommendation),
            escape_markdown(&verification.command),
        ),
        (Some(recommendation), None) => escape_markdown(recommendation),
        (None, Some(verification)) => {
            format!("Verify: `{}`", escape_markdown(&verification.command))
        }
        (None, None) => "Open the focused report for evidence.".to_owned(),
    }
}

fn issue_disposition_label(disposition: &str, reason: &str) -> String {
    if reason.is_empty() {
        format!("`{}`", escape_markdown(disposition))
    } else {
        format!(
            "`{}`<br>{}",
            escape_markdown(disposition),
            escape_markdown(reason)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{IssueDispositionList, render_list_human, render_list_markdown};
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

    #[tokio::test]
    async fn list_projects_issue_commands_and_maintainer_context() {
        let root = std::env::temp_dir().join(format!(
            "cliare-issues-list-rich-test-{}",
            std::process::id()
        ));
        tokio::fs::create_dir_all(&root)
            .await
            .expect("creates temp issue dir");
        let issues = serde_json::json!({
            "issues": [
                {
                    "id": "issue.invalid_flag_diagnostics_unknown",
                    "status": "open",
                    "title": "1 command needs clearer invalid-flag diagnostics",
                    "severity": "medium",
                    "category": "recovery",
                    "agent_readiness_area": "diagnostics",
                    "confidence": "inferred",
                    "impact": "Agents depend on precise nonzero diagnostics.",
                    "why_it_matters": "Diagnostics let agents repair bad command attempts.",
                    "recommendation": "Reject unknown flags with clear nonzero diagnostics.",
                    "verification": {
                        "command": "cliare measure mise --out .cliare --profile deep --refresh",
                        "expected_change": "The issue disappears or is dispositioned."
                    },
                    "affected_commands": [
                        {
                            "path": ["outdated"],
                            "argv": ["mise", "outdated"],
                            "state": "runtime_confirmed",
                            "confidence": 0.99,
                            "summary": "Shows outdated tool versions",
                            "reason": "safe invalid-flag probe has not observed flag diagnostics",
                            "required_positionals": []
                        }
                    ]
                }
            ]
        });
        tokio::fs::write(
            root.join(crate::artifacts::ISSUES_JSON),
            serde_json::to_vec(&issues).expect("issues fixture serializes"),
        )
        .await
        .expect("writes issues fixture");

        let packet = IssueDispositionList::build(&root, &IssueDispositions::default())
            .await
            .expect("list packet builds");
        let markdown = render_list_markdown(&packet);
        let human = render_list_human(&packet);

        assert_eq!(packet.issues.len(), 1);
        let issue = &packet.issues[0];
        assert_eq!(issue.affected_command_count, 1);
        assert_eq!(issue.command_samples[0].command, "mise outdated");
        assert_eq!(
            issue.command_samples[0].summary.as_deref(),
            Some("Shows outdated tool versions")
        );
        let normalized_command = format!(
            "cliare measure mise --out {} --profile deep --refresh",
            crate::report_format::shell_arg(&root.display().to_string())
        );
        assert_eq!(
            issue
                .verification
                .as_ref()
                .map(|entry| entry.command.as_str()),
            Some(normalized_command.as_str())
        );
        assert!(markdown.contains("mise outdated"));
        assert!(markdown.contains("Shows outdated tool versions"));
        assert!(markdown.contains("Reject unknown flags"));
        assert!(markdown.contains(&format!(
            "cliare measure mise --out {}",
            crate::report_format::shell_arg(&root.display().to_string())
        )));
        assert!(human.contains("Action required (1)"));
        assert!(human.contains("1 command needs clearer invalid-flag diagnostics"));
        assert!(human.contains("mise outdated: Shows outdated tool versions"));
        assert!(human.contains("Disposition examples:"));

        let _ = tokio::fs::remove_dir_all(root).await;
    }
}
