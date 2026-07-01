use std::cmp::Ordering;
use std::fmt::Write as _;

use crate::report_format::{command_path_label, escape_markdown, output_mode_label, shell_words};
use crate::report_model::*;
use cliare_issues::issue_disposition::IssueDispositionStatus;

use super::PERSONA_FINDING_LIMIT;

pub(super) fn render_reviewed_decisions(text: &mut String, packet: &PersonaOutcomePacket) {
    if packet.reviewed_issues.is_empty() {
        return;
    }

    writeln!(text, "## Reviewed Decisions").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "These findings remain in the evidence ledger, but maintainer disposition removes them from the action queue for this persona."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Disposition | Issue | Reason | Harness Guidance |")
        .expect("writing to string cannot fail");
    writeln!(text, "|---|---|---|---|").expect("writing to string cannot fail");
    for issue in packet.reviewed_issues.iter().take(PERSONA_FINDING_LIMIT) {
        let disposition = issue
            .disposition
            .as_ref()
            .expect("reviewed issues always have a disposition");
        writeln!(
            text,
            "| `{}` | `{}` {} | {} | {} |",
            disposition.status.label(),
            escape_markdown(&issue.id),
            escape_markdown(&issue.title),
            escape_markdown(&disposition.reason),
            escape_markdown(reviewed_harness_guidance(issue))
        )
        .expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn issue_disposition_label(issue: &Issue) -> &'static str {
    issue
        .disposition
        .as_ref()
        .map(|disposition| disposition.status.label())
        .unwrap_or("open")
}

pub(super) fn reviewed_harness_guidance(issue: &Issue) -> &'static str {
    match issue
        .disposition
        .as_ref()
        .map(|disposition| disposition.status)
    {
        Some(IssueDispositionStatus::Intentional) => "follow the recorded project convention",
        Some(IssueDispositionStatus::NotApplicable) => {
            "ignore for this CLI unless local policy says otherwise"
        }
        Some(IssueDispositionStatus::FalsePositive) => "do not route around this finding",
        Some(IssueDispositionStatus::AcceptedRisk) => {
            "route only when the harness can tolerate the recorded risk"
        }
        Some(IssueDispositionStatus::Deferred) => {
            "treat as known debt rather than an immediate blocker"
        }
        Some(IssueDispositionStatus::Open)
        | Some(IssueDispositionStatus::Accepted)
        | Some(IssueDispositionStatus::NeedsFixture)
        | None => issue.agent_readiness_area.agent_impact(),
    }
}

pub(super) fn command_section_heading(issue: &Issue) -> &'static str {
    match issue.confidence {
        IssueConfidence::Observed => "Affected commands",
        IssueConfidence::Blocked => "Blocked command examples",
        IssueConfidence::NeedsFixture => "Fixture examples",
        IssueConfidence::Inferred if issue_has_runtime_confirmed_commands(issue) => {
            "Commands to verify"
        }
        IssueConfidence::Inferred => "Candidate examples to review",
        IssueConfidence::Advisory => "Related examples",
    }
}

pub(super) fn command_overflow_label(issue: &Issue) -> &'static str {
    match issue.confidence {
        IssueConfidence::Observed => "affected command(s)",
        IssueConfidence::Blocked => "blocked command example(s)",
        IssueConfidence::NeedsFixture => "fixture example(s)",
        IssueConfidence::Inferred if issue_has_runtime_confirmed_commands(issue) => {
            "command example(s)"
        }
        IssueConfidence::Inferred => "candidate example(s)",
        IssueConfidence::Advisory => "related example(s)",
    }
}

pub(super) fn evidence_section_heading(issue: &Issue) -> &'static str {
    match issue.confidence {
        IssueConfidence::Inferred | IssueConfidence::Advisory => "Evidence references",
        _ => "Evidence to open",
    }
}

pub(super) fn issue_command_samples(issue: &Issue) -> Vec<&IssueCommand> {
    let mut commands = issue.affected_commands.iter().collect::<Vec<_>>();
    commands.sort_by(issue_command_sample_order);
    commands
}

pub(super) fn issue_has_runtime_confirmed_commands(issue: &Issue) -> bool {
    issue
        .affected_commands
        .iter()
        .any(|command| command.state == "runtime_confirmed")
}

pub(super) fn issue_command_sample_order(left: &&IssueCommand, right: &&IssueCommand) -> Ordering {
    right
        .confidence
        .unwrap_or(f64::NEG_INFINITY)
        .total_cmp(&left.confidence.unwrap_or(f64::NEG_INFINITY))
        .then_with(|| left.path.len().cmp(&right.path.len()))
        .then_with(|| left.path.cmp(&right.path))
}

pub(super) fn render_issue_command_sample(text: &mut String, command: &IssueCommand) {
    let detail = if command.required_positionals.is_empty() {
        command.reason.clone()
    } else {
        let required = required_positionals_label(&command.required_positionals);
        if command.reason.contains(&required) {
            command.reason.clone()
        } else {
            format!(
                "{}; required operands {}",
                command.reason.trim_end_matches('.'),
                required
            )
        }
    };
    writeln!(
        text,
        "- `{}`: {}",
        escape_markdown(&command_path_label(&command.path)),
        escape_markdown(&detail)
    )
    .expect("writing to string cannot fail");
    for contract in command.output_contracts.iter().take(1) {
        writeln!(
            text,
            "  - Contract: `{}` via `{}` is `{}`.{}{}",
            escape_markdown(&output_mode_label(&contract.mode)),
            escape_markdown(&shell_words(&contract.argv_fragment)),
            escape_markdown(&contract.status),
            contract
                .skip_reason
                .as_ref()
                .map(|reason| format!(" {}", escape_markdown(reason)))
                .unwrap_or_default(),
            contract
                .suggested_validation
                .as_ref()
                .map(|suggestion| format!(" {}", escape_markdown(suggestion)))
                .unwrap_or_default()
        )
        .expect("writing to string cannot fail");
    }
}

pub(super) fn required_positionals_label(required_positionals: &[String]) -> String {
    required_positionals
        .iter()
        .map(|name| format!("<{name}>"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn render_issue_evidence_sample(text: &mut String, evidence: &IssueEvidence) {
    let status = evidence
        .status
        .as_ref()
        .map(|status| format!(" `{}`", escape_markdown(status)))
        .unwrap_or_default();
    let argv = if evidence.argv.is_empty() {
        String::new()
    } else {
        format!("; `{}`", escape_markdown(&evidence.argv.join(" ")))
    };
    let detail = if evidence.detail.is_empty() {
        String::new()
    } else {
        format!(" - {}", escape_markdown(&evidence.detail))
    };
    writeln!(
        text,
        "- `{}`{}{}{}",
        escape_markdown(&evidence.reference),
        status,
        argv,
        detail
    )
    .expect("writing to string cannot fail");
}
