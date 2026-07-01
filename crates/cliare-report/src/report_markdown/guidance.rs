use std::cmp::Ordering;
use std::fmt::Write as _;

use crate::report_format::{command_path_label, escape_markdown};
use crate::report_model::*;

use super::COMMAND_GUIDANCE_SAMPLE_LIMIT;
use super::samples::issue_has_runtime_confirmed_commands;

pub(super) fn render_persona_command_guidance(text: &mut String, packet: &PersonaOutcomePacket) {
    let ready = packet
        .command_health
        .iter()
        .filter(|command| command.readiness_state == CommandReadinessState::Ready)
        .collect::<Vec<_>>();
    let blocked = packet
        .command_health
        .iter()
        .filter(|command| command.readiness_state == CommandReadinessState::Blocked)
        .collect::<Vec<_>>();
    let conditional = packet
        .command_health
        .iter()
        .filter(|command| command.readiness_state == CommandReadinessState::Conditional)
        .collect::<Vec<_>>();
    let needs_fixture = packet
        .command_health
        .iter()
        .filter(|command| command.readiness_state == CommandReadinessState::NeedsFixture)
        .collect::<Vec<_>>();
    let candidate = packet
        .command_health
        .iter()
        .filter(|command| command.readiness_state == CommandReadinessState::Candidate)
        .collect::<Vec<_>>();

    writeln!(text, "## Command Set Guidance").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "{}",
        escape_markdown(persona_command_guidance(packet.persona))
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "| State | Count | Persona treatment | Sample commands |"
    )
    .expect("writing to string cannot fail");
    writeln!(text, "|---|---:|---|---|").expect("writing to string cannot fail");
    render_command_guidance_row(
        text,
        "Ready",
        ready.len(),
        "Candidate for automatic routing after local policy review.",
        &ready,
    );
    render_command_guidance_row(
        text,
        "Conditional",
        conditional.len(),
        "Expose only when the harness can satisfy the listed condition or review policy.",
        &conditional,
    );
    render_command_guidance_row(
        text,
        "Needs fixtures",
        needs_fixture.len(),
        "Hold until safe operands, sample data, or command-local output validation exist.",
        &needs_fixture,
    );
    render_command_guidance_row(
        text,
        "Blocked",
        blocked.len(),
        "Unavailable for automatic routing until unsatisfied runtime preconditions are handled.",
        &blocked,
    );
    render_command_guidance_row(
        text,
        "Candidates",
        candidate.len(),
        "Do not expose automatically until runtime confirmation exists.",
        &candidate,
    );
    writeln!(
        text,
        "| Full catalog | {} | Use the command index for command-level drill-down. | `persona-{}.json`, `command-index.json`, `shape.json` |",
        packet.command_health.len(),
        packet.persona.label()
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn render_command_guidance_row(
    text: &mut String,
    state: &str,
    count: usize,
    treatment: &str,
    commands: &[&CommandHealth],
) {
    writeln!(
        text,
        "| {} | {} | {} | {} |",
        escape_markdown(state),
        count,
        escape_markdown(treatment),
        command_sample_list(commands, COMMAND_GUIDANCE_SAMPLE_LIMIT)
    )
    .expect("writing to string cannot fail");
}

pub(super) fn render_run_recommendations(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## Recommended Runs").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    if packet.run_recommendations.is_empty() {
        writeln!(
            text,
            "No follow-up run is required before acting on the findings above."
        )
        .expect("writing to string cannot fail");
    } else {
        writeln!(text, "| Run | Purpose | Command | Use when |")
            .expect("writing to string cannot fail");
        writeln!(text, "|---|---|---|---|").expect("writing to string cannot fail");
        for recommendation in &packet.run_recommendations {
            writeln!(
                text,
                "| `{}` | {} | `{}` | {} |",
                escape_markdown(&recommendation.id),
                escape_markdown(&recommendation.purpose),
                escape_markdown(&recommendation.command),
                escape_markdown(&use_when_text(&recommendation.when_to_use))
            )
            .expect("writing to string cannot fail");
        }
    }
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn persona_issue_action(persona: Persona, issue: &Issue) -> &'static str {
    match persona {
        Persona::Maintainer if issue.id == "issue.alternate_help_form_unavailable" => {
            "Treat this as optional compatibility. Add `help <command path>` only where it improves agent navigation enough to justify the maintenance cost."
        }
        Persona::Maintainer if issue.id == "issue.help_unavailable" => {
            "Make canonical direct `<command> --help` available for real commands, with usage, operands, flags, and safe diagnostics that agents can parse."
        }
        Persona::Maintainer
            if issue.confidence == IssueConfidence::Inferred
                && issue_has_runtime_confirmed_commands(issue) =>
        {
            "Verify the measured gap on the listed commands. If the behavior is intentional, document it as a CLI contract; otherwise make the help or diagnostic path explicit."
        }
        Persona::Maintainer if issue.confidence == IssueConfidence::Inferred => {
            "Review the candidate set before changing code. Confirm true commands by improving help/catalog clarity; classify false candidates as inference noise in the ledger."
        }
        Persona::Maintainer => match issue.category {
            ActionCategory::Output => {
                "Add a safe validation path or fixture for the advertised machine-readable output contract."
            }
            ActionCategory::Discovery => {
                "Make command existence and help discoverable without relying on configured account state."
            }
            ActionCategory::Recovery => {
                "Improve diagnostics so agents can repair invalid command attempts without guessing."
            }
            ActionCategory::Safety => {
                "Keep help/version/diagnostic paths read-only, or provide a documented way to suppress expected writes."
            }
            _ => "Fix the CLI contract behind this finding and rerun the same profile.",
        },
        Persona::Harness if issue.id == "issue.alternate_help_form_unavailable" => {
            "Do not demote otherwise ready commands solely because optional `help <command path>` compatibility is unavailable."
        }
        Persona::Harness
            if issue.confidence == IssueConfidence::Inferred
                && issue_has_runtime_confirmed_commands(issue) =>
        {
            "Treat these commands as conditional for agents until the missing help, diagnostic, or output evidence is confirmed."
        }
        Persona::Harness if issue.confidence == IssueConfidence::Inferred => {
            "Do not expose inferred candidates to agents until runtime probes confirm them or the harness provisions the missing precondition."
        }
        Persona::Harness => match issue.category {
            ActionCategory::Safety => {
                "Do not run this probe profile unattended until side-effect policy, fixtures, and allowed persistent writes are explicit."
            }
            ActionCategory::Output => {
                "Do not route agent state through this output mode until a safe fixture or parseable probe confirms it."
            }
            ActionCategory::Discovery => {
                "Treat blocked or unconfirmed commands as unavailable unless the harness provisions the required precondition."
            }
            _ => {
                "Mark affected commands as conditional in the harness catalog until the finding is resolved."
            }
        },
        Persona::Platform => {
            "Convert this finding into a CI policy decision: fail, warn, or allow with a documented exception."
        }
        Persona::Security => match issue.confidence {
            IssueConfidence::Observed => {
                "Review as observed runtime behavior with direct evidence."
            }
            IssueConfidence::Blocked => {
                "Require documented runtime preconditions before approving automated use."
            }
            IssueConfidence::NeedsFixture => {
                "Require safe fixture definitions before accepting the advertised contract."
            }
            _ => "Keep this as an uncertainty item until stronger runtime evidence exists.",
        },
        Persona::Oss => {
            "Publish this as an open remediation item with evidence and avoid claiming it is fixed until the verification command changes."
        }
        Persona::Devrel => {
            "Turn this into user-facing guidance: what behavior agents need, what the CLI currently does, and how the project will improve it."
        }
        Persona::Research => {
            "Label this finding with its confidence class and evidence references before using it for calibration."
        }
    }
}

pub(super) fn persona_command_guidance(persona: Persona) -> &'static str {
    match persona {
        Persona::Harness => {
            "Use this section as the first-pass agent exposure map. It mirrors `command-index.json` so harness routing, skill generation, and human review use the same readiness labels."
        }
        Persona::Security => {
            "Use this section to separate commands with confirmed safe shape from commands blocked by preconditions or incomplete evidence."
        }
        Persona::Platform => {
            "Use this section to decide which command classes should fail CI, warn, or require an exception."
        }
        Persona::Research => {
            "Use this section to preserve readiness labels for downstream analysis; do not collapse blocked, incomplete, and unconfirmed into one class."
        }
        _ => {
            "Use this section as a compact triage map. The full command catalog remains in machine-readable artifacts."
        }
    }
}

pub(super) fn command_sample_list(commands: &[&CommandHealth], limit: usize) -> String {
    if commands.is_empty() {
        return "`none`".to_owned();
    }
    let mut commands = commands.to_vec();
    commands.sort_by(command_health_sample_order);
    let mut labels = commands
        .iter()
        .take(limit)
        .map(|command| format!("`{}`", escape_markdown(&command_path_label(&command.path))))
        .collect::<Vec<_>>();
    if commands.len() > limit {
        labels.push(format!("... {} more", commands.len() - limit));
    }
    labels.join(", ")
}

pub(super) fn command_health_sample_order(
    left: &&CommandHealth,
    right: &&CommandHealth,
) -> Ordering {
    right
        .confidence
        .total_cmp(&left.confidence)
        .then_with(|| left.path.len().cmp(&right.path.len()))
        .then_with(|| left.path.cmp(&right.path))
}

pub(super) fn use_when_text(value: &str) -> String {
    value
        .trim()
        .strip_prefix("Use ")
        .unwrap_or_else(|| value.trim())
        .to_owned()
}
