use std::fmt::Write as _;
use std::path::Path;

use crate::report_format::escape_markdown;
use crate::report_model::*;

use super::actions::{
    issue_meaning, maintainer_agent_outcome, maintainer_disposition_text, maintainer_fix_text,
};
use super::guidance::persona_issue_action;
use super::samples::{
    command_overflow_label, command_section_heading, evidence_section_heading,
    issue_command_samples, issue_disposition_label, render_issue_command_sample,
    render_issue_evidence_sample,
};
use super::{PERSONA_COMMAND_SAMPLE_LIMIT, PERSONA_EVIDENCE_SAMPLE_LIMIT, PERSONA_FINDING_LIMIT};

pub(super) fn render_persona_findings(text: &mut String, packet: &PersonaOutcomePacket) {
    if packet.persona == Persona::Maintainer {
        render_maintainer_evidence_drilldown(text, packet);
        return;
    }

    writeln!(text, "## Priority Findings").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    if packet.top_issues.is_empty() {
        writeln!(
            text,
            "No findings are prioritized for this persona. Use `issues.json` for the full ledger."
        )
        .expect("writing to string cannot fail");
        writeln!(text).expect("writing to string cannot fail");
        return;
    }

    writeln!(
        text,
        "Start with this table, then open the matching drill-down section only when you need command samples, evidence, or verification details."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "| Priority | Meaning | Disposition | Affected | Issue | Action |"
    )
    .expect("writing to string cannot fail");
    writeln!(text, "|---:|---|---|---:|---|---|").expect("writing to string cannot fail");
    for (index, issue) in packet
        .top_issues
        .iter()
        .take(PERSONA_FINDING_LIMIT)
        .enumerate()
    {
        render_persona_finding_row(text, packet.persona, issue, index + 1);
    }
    writeln!(text).expect("writing to string cannot fail");
    if packet.top_issues.len() > PERSONA_FINDING_LIMIT {
        writeln!(
            text,
            "This report shows the top {} persona findings. See `issues.json` for the remaining {} issue(s).",
            PERSONA_FINDING_LIMIT,
            packet.top_issues.len() - PERSONA_FINDING_LIMIT
        )
        .expect("writing to string cannot fail");
        writeln!(text).expect("writing to string cannot fail");
    }

    writeln!(text, "## Finding Drill-Down").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    for (index, issue) in packet
        .top_issues
        .iter()
        .take(PERSONA_FINDING_LIMIT)
        .enumerate()
    {
        render_persona_finding(text, packet.persona, issue, index + 1);
    }
}

fn render_maintainer_evidence_drilldown(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## Evidence Drill-Down").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    if packet.top_issues.is_empty() {
        writeln!(
            text,
            "No maintainer evidence drill-down is needed for this run."
        )
        .expect("writing to string cannot fail");
        writeln!(text).expect("writing to string cannot fail");
        return;
    }

    writeln!(
        text,
        "Open these details only when an action item needs command examples or evidence references."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    if packet.top_issues.len() > PERSONA_FINDING_LIMIT {
        writeln!(
            text,
            "This report shows the top {} maintainer findings. See `issues.json` for the remaining {} issue(s).",
            PERSONA_FINDING_LIMIT,
            packet.top_issues.len() - PERSONA_FINDING_LIMIT
        )
        .expect("writing to string cannot fail");
        writeln!(text).expect("writing to string cannot fail");
    }

    for issue in packet.top_issues.iter().take(PERSONA_FINDING_LIMIT) {
        render_maintainer_finding(text, &packet.source_artifacts.artifact_dir, issue);
    }
}

pub(super) fn render_persona_finding_row(
    text: &mut String,
    persona: Persona,
    issue: &Issue,
    priority: usize,
) {
    writeln!(
        text,
        "| P{} | {} | `{}` | {} | `{}` {} | {} |",
        priority,
        escape_markdown(issue_meaning(issue)),
        issue_disposition_label(issue),
        issue.affected_commands.len(),
        escape_markdown(&issue.id),
        escape_markdown(&issue.title),
        escape_markdown(persona_issue_action(persona, issue))
    )
    .expect("writing to string cannot fail");
}

pub(super) fn render_maintainer_finding(text: &mut String, artifact_dir: &Path, issue: &Issue) {
    let area = issue.agent_readiness_area.label();
    writeln!(text, "<details>").expect("writing to string cannot fail");
    writeln!(
        text,
        "<summary>{}: {} (`{}`)</summary>",
        escape_markdown(area),
        escape_markdown(&issue.title),
        escape_markdown(&issue.id)
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "### {}: {}",
        escape_markdown(area),
        escape_markdown(&issue.title)
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    render_maintainer_finding_body(text, artifact_dir, issue);
    writeln!(text, "</details>").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn render_named_finding(text: &mut String, persona: Persona, issue: &Issue) {
    writeln!(text, "<details>").expect("writing to string cannot fail");
    writeln!(
        text,
        "<summary>{} (`{}`)</summary>",
        escape_markdown(&issue.title),
        escape_markdown(&issue.id)
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "### {}", escape_markdown(&issue.title)).expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    render_finding_body(text, persona, issue);
    writeln!(text, "</details>").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn render_persona_finding(
    text: &mut String,
    persona: Persona,
    issue: &Issue,
    priority: usize,
) {
    writeln!(text, "<details>").expect("writing to string cannot fail");
    writeln!(
        text,
        "<summary>P{}: {} (`{}`)</summary>",
        priority,
        escape_markdown(&issue.title),
        escape_markdown(&issue.id)
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "### P{}: {}", priority, escape_markdown(&issue.title))
        .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    render_finding_body(text, persona, issue);
    writeln!(text, "</details>").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn render_finding_body(text: &mut String, persona: Persona, issue: &Issue) {
    writeln!(
        text,
        "- Assessment: `{}` {} (`{}`, `{}`, `{}`)",
        escape_markdown(&issue.id),
        escape_markdown(&issue.title),
        issue.severity.label(),
        issue.category.label(),
        issue.confidence.label()
    )
    .expect("writing to string cannot fail");
    writeln!(text, "- Meaning: {}", escape_markdown(&issue.impact))
        .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Evidence interpretation: {}",
        escape_markdown(issue_meaning(issue))
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Associated commands: `{}` affected.",
        issue.affected_commands.len()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Suggested remedy: {}",
        escape_markdown(&issue.recommendation)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Role action: {}",
        escape_markdown(persona_issue_action(persona, issue))
    )
    .expect("writing to string cannot fail");
    if persona == Persona::Maintainer {
        writeln!(
            text,
            "- Area: {}",
            escape_markdown(issue.agent_readiness_area.label())
        )
        .expect("writing to string cannot fail");
        writeln!(
            text,
            "- Agent impact: {}",
            escape_markdown(issue.agent_readiness_area.agent_impact())
        )
        .expect("writing to string cannot fail");
    }
    if let Some(disposition) = &issue.disposition {
        writeln!(
            text,
            "- Maintainer disposition: `{}` - {}",
            disposition.status.label(),
            escape_markdown(&disposition.reason)
        )
        .expect("writing to string cannot fail");
    }
    writeln!(
        text,
        "- Verification: `{}`",
        escape_markdown(&issue.verification.command)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Expected improvement: {}",
        escape_markdown(&issue.verification.expected_change)
    )
    .expect("writing to string cannot fail");

    if !issue.affected_commands.is_empty() {
        writeln!(text).expect("writing to string cannot fail");
        writeln!(text, "{}:", command_section_heading(issue))
            .expect("writing to string cannot fail");
        let commands = issue_command_samples(issue);
        for command in commands.iter().take(PERSONA_COMMAND_SAMPLE_LIMIT) {
            render_issue_command_sample(text, command);
        }
        if issue.affected_commands.len() > PERSONA_COMMAND_SAMPLE_LIMIT {
            writeln!(
                text,
                "- ... {} more {} in `issues.json`.",
                issue.affected_commands.len() - PERSONA_COMMAND_SAMPLE_LIMIT,
                command_overflow_label(issue)
            )
            .expect("writing to string cannot fail");
        }
    }

    if !issue.evidence.is_empty() {
        writeln!(text).expect("writing to string cannot fail");
        writeln!(text, "{}:", evidence_section_heading(issue))
            .expect("writing to string cannot fail");
        for evidence in issue.evidence.iter().take(PERSONA_EVIDENCE_SAMPLE_LIMIT) {
            render_issue_evidence_sample(text, evidence);
        }
    }
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn render_maintainer_finding_body(
    text: &mut String,
    artifact_dir: &Path,
    issue: &Issue,
) {
    writeln!(
        text,
        "- Assessment: `{}` {} (`{}`, `{}`, `{}`)",
        escape_markdown(&issue.id),
        escape_markdown(&issue.title),
        issue.severity.label(),
        issue.category.label(),
        issue.confidence.label()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Meaning: {}",
        escape_markdown(&maintainer_agent_outcome(issue))
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Associated commands: `{}` affected.",
        issue.affected_commands.len()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Suggested remedy: {}",
        escape_markdown(&maintainer_fix_text(issue))
    )
    .expect("writing to string cannot fail");
    if let Some(disposition) = &issue.disposition {
        writeln!(
            text,
            "- Disposition: `{}` - {}",
            disposition.status.label(),
            escape_markdown(&disposition.reason)
        )
        .expect("writing to string cannot fail");
    } else {
        writeln!(
            text,
            "- If acceptable: {}",
            escape_markdown(&maintainer_disposition_text(artifact_dir, issue))
        )
        .expect("writing to string cannot fail");
    }
    writeln!(
        text,
        "- Verify: `{}`",
        escape_markdown(&issue.verification.command)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Expected change: {}",
        escape_markdown(&issue.verification.expected_change)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Area: {}",
        escape_markdown(issue.agent_readiness_area.label())
    )
    .expect("writing to string cannot fail");

    render_command_examples(text, issue);
    render_evidence_refs(text, issue);
}

pub(super) fn render_command_examples(text: &mut String, issue: &Issue) {
    if issue.affected_commands.is_empty() {
        return;
    }

    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "{}:", command_section_heading(issue)).expect("writing to string cannot fail");
    let commands = issue_command_samples(issue);
    for command in commands.iter().take(PERSONA_COMMAND_SAMPLE_LIMIT) {
        render_issue_command_sample(text, command);
    }
    if issue.affected_commands.len() > PERSONA_COMMAND_SAMPLE_LIMIT {
        writeln!(
            text,
            "- ... {} more {} in `issues.json`.",
            issue.affected_commands.len() - PERSONA_COMMAND_SAMPLE_LIMIT,
            command_overflow_label(issue)
        )
        .expect("writing to string cannot fail");
    }
}

pub(super) fn render_evidence_refs(text: &mut String, issue: &Issue) {
    if issue.evidence.is_empty() {
        return;
    }

    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "{}:", evidence_section_heading(issue)).expect("writing to string cannot fail");
    for evidence in issue.evidence.iter().take(PERSONA_EVIDENCE_SAMPLE_LIMIT) {
        render_issue_evidence_sample(text, evidence);
    }
}
