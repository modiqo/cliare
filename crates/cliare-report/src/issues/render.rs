use std::fmt::Write as _;
use std::path::Path;

use crate::report_format::{escape_markdown, shell_arg};
use cliare_issues::issue_disposition::IssueDispositionStatus;

use super::model::{IssueDispositionList, IssueDispositionListItem};

pub(super) fn render_mark_summary(
    artifact_dir: &Path,
    dispositions_path: &Path,
    issue_id: &str,
    status: IssueDispositionStatus,
) -> String {
    format!(
        "Recorded issue disposition: `{}` -> `{}`\nArtifact dir: `{}`\nDispositions: `{}`\n",
        escape_markdown(issue_id),
        status.label(),
        artifact_dir.display(),
        dispositions_path.display()
    )
}

pub(super) fn render_list_markdown(packet: &IssueDispositionList) -> String {
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

pub(super) fn render_list_human(packet: &IssueDispositionList) -> String {
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
