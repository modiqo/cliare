use std::fmt::Write as _;

use crate::report_format::{command_path_label, escape_markdown, output_mode_label, shell_words};
use crate::report_model::*;

use super::COMMAND_SAMPLE_LIMIT;
use super::actions::issue_meaning;

pub(crate) fn render_issue_ledger_markdown(ledger: &IssueLedger) -> String {
    let mut text = String::new();
    writeln!(&mut text, "# CLIARE Issue Ledger").expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Target: `{}`",
        escape_markdown(&ledger.target.requested.display().to_string())
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Issues: `{}` (`{}` high, `{}` medium, `{}` low)",
        ledger.summary.issues_total, ledger.summary.high, ledger.summary.medium, ledger.summary.low
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Affected commands: `{}`",
        ledger.summary.affected_commands
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Fixture-required issues: `{}`",
        ledger.summary.requires_fixtures
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Precondition-blocked issues: `{}`",
        ledger.summary.blocked_by_preconditions
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Dispositioned issues: `{}`",
        ledger.summary.dispositioned
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Action required: `{}`",
        ledger.summary.action_required
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "- Reviewed decisions: `{}`",
        ledger.summary.reviewed_decisions
    )
    .expect("writing to string cannot fail");

    for issue in &ledger.issues {
        render_issue_markdown(&mut text, issue, 2);
    }

    text
}

fn render_issue_markdown(text: &mut String, issue: &Issue, heading_level: usize) {
    let heading = "#".repeat(heading_level);
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "{} {}", heading, escape_markdown(&issue.title))
        .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "- ID: `{}`", escape_markdown(&issue.id))
        .expect("writing to string cannot fail");
    writeln!(text, "- Severity: `{}`", issue.severity.label())
        .expect("writing to string cannot fail");
    writeln!(text, "- Category: `{}`", issue.category.label())
        .expect("writing to string cannot fail");
    writeln!(text, "- Confidence: `{}`", issue.confidence.label())
        .expect("writing to string cannot fail");
    writeln!(text, "- Assessment: {}", escape_markdown(&issue.title))
        .expect("writing to string cannot fail");
    writeln!(text, "- Meaning: {}", escape_markdown(&issue.impact))
        .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Evidence interpretation: {}",
        escape_markdown(issue_meaning(issue))
    )
    .expect("writing to string cannot fail");
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
        "- Associated commands: `{}`",
        issue.affected_commands.len()
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "**Suggested remedy:** {}",
        escape_markdown(&issue.recommendation)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "**Why it matters:** {}",
        escape_markdown(&issue.why_it_matters)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "**Verify:** `{}`",
        escape_markdown(&issue.verification.command)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "**Expected change:** {}",
        escape_markdown(&issue.verification.expected_change)
    )
    .expect("writing to string cannot fail");

    if !issue.affected_commands.is_empty() {
        writeln!(text).expect("writing to string cannot fail");
        writeln!(text, "Affected command samples:").expect("writing to string cannot fail");
        for command in issue.affected_commands.iter().take(COMMAND_SAMPLE_LIMIT) {
            let required = if command.required_positionals.is_empty()
                || !command.output_contracts.is_empty()
            {
                String::new()
            } else {
                format!(
                    " Required operands: {}.",
                    command
                        .required_positionals
                        .iter()
                        .map(|name| format!("<{name}>"))
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            };
            writeln!(
                text,
                "- `{}` ({}, confidence {}) - {}{}",
                escape_markdown(&command_path_label(&command.path)),
                escape_markdown(&command.state),
                command
                    .confidence
                    .map(|value| format!("{value:.3}"))
                    .unwrap_or_else(|| "n/a".to_owned()),
                escape_markdown(&command.reason),
                escape_markdown(&required)
            )
            .expect("writing to string cannot fail");
            for contract in &command.output_contracts {
                writeln!(
                    text,
                    "  - Output contract: `{}` via `{}`; status `{}`.{}{}",
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
    }

    if !issue.evidence.is_empty() {
        writeln!(text).expect("writing to string cannot fail");
        writeln!(text, "Evidence samples:").expect("writing to string cannot fail");
        for evidence in issue.evidence.iter().take(5) {
            let argv = if evidence.argv.is_empty() {
                String::new()
            } else {
                format!(" argv `{}`", escape_markdown(&evidence.argv.join(" ")))
            };
            let status = evidence
                .status
                .as_ref()
                .map(|status| format!(" status `{}`", escape_markdown(status)))
                .unwrap_or_default();
            writeln!(
                text,
                "- `{}` {}{} - {}",
                escape_markdown(&evidence.reference),
                evidence
                    .intent
                    .as_ref()
                    .map(|intent| format!("intent `{}`", escape_markdown(intent)))
                    .unwrap_or_else(|| format!("kind `{}`", escape_markdown(&evidence.kind))),
                status,
                escape_markdown(&format!("{}{}", evidence.detail, argv))
            )
            .expect("writing to string cannot fail");
        }
    }
}
