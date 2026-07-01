use std::fmt::Write as _;
use std::path::Path;

use crate::report_format::{command_path_label, escape_markdown, shell_arg};
use crate::report_model::*;

use super::PERSONA_FINDING_LIMIT;
use super::guidance::persona_issue_action;
use super::samples::{issue_command_samples, issue_has_runtime_confirmed_commands};

pub(super) fn render_ci_action_brief(text: &mut String, packet: &PersonaOutcomePacket) {
    let heading = if packet.persona == Persona::Maintainer {
        "## Maintainer Action Queue"
    } else {
        "## CI Action Brief"
    };
    writeln!(text, "{heading}").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");

    if packet.top_issues.is_empty() {
        writeln!(
            text,
            "No persona-prioritized fixes are required by this run. Keep the scorecard and command index in CI so future drift is visible."
        )
        .expect("writing to string cannot fail");
        writeln!(text).expect("writing to string cannot fail");
        writeln!(text, "| Check | Result |").expect("writing to string cannot fail");
        writeln!(text, "|---|---|").expect("writing to string cannot fail");
        writeln!(text, "| Score | `{:.0}/100` |", packet.summary.score)
            .expect("writing to string cannot fail");
        writeln!(
            text,
            "| Command coverage | `{}/{}` runtime-confirmed |",
            packet.summary.commands_runtime_confirmed, packet.summary.commands_discovered
        )
        .expect("writing to string cannot fail");
        writeln!(
            text,
            "| Command drill-down | `{}` |",
            packet.source_artifacts.command_index.display()
        )
        .expect("writing to string cannot fail");
        writeln!(text).expect("writing to string cannot fail");
        return;
    }

    let high = packet
        .top_issues
        .iter()
        .filter(|issue| issue.severity == ActionSeverity::High)
        .count();
    let fixture_required = packet
        .top_issues
        .iter()
        .filter(|issue| issue.confidence == IssueConfidence::NeedsFixture)
        .count();
    if packet.persona == Persona::Maintainer {
        render_maintainer_action_brief(text, packet, high, fixture_required);
        return;
    }

    writeln!(
        text,
        "Treat the rows below as the persona-specific CI work queue. Fix the first row first, rerun the verification command, and use `command-index.json` only when you need exact command parameters or evidence pointers."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Field | Value |").expect("writing to string cannot fail");
    writeln!(text, "|---|---|").expect("writing to string cannot fail");
    writeln!(text, "| Score | `{:.0}/100` |", packet.summary.score)
        .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Persona work queue | `{}` prioritized issue(s), `{}` high severity, `{}` need fixtures |",
        packet.top_issues.len(),
        high,
        fixture_required
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Command drill-down | `{}` |",
        packet.source_artifacts.command_index.display()
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");

    writeln!(text, "| Priority | Meaning | Where | Do this | Verify |")
        .expect("writing to string cannot fail");
    writeln!(text, "|---:|---|---|---|---|").expect("writing to string cannot fail");
    for (index, issue) in packet
        .top_issues
        .iter()
        .take(PERSONA_FINDING_LIMIT)
        .enumerate()
    {
        writeln!(
            text,
            "| P{} | {} | {} | {} | `{}` |",
            index + 1,
            escape_markdown(issue_meaning(issue)),
            escape_markdown(&issue_where_label(issue)),
            escape_markdown(&ci_action_text(packet.persona, issue)),
            escape_markdown(&issue.verification.command)
        )
        .expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn render_maintainer_action_brief(
    text: &mut String,
    packet: &PersonaOutcomePacket,
    high: usize,
    fixture_required: usize,
) {
    writeln!(
        text,
        "Start here. Each item says what is wrong, what agents lose because of it, how to fix it, and how to record a disposition when the behavior is acceptable for this run."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Field | Value |").expect("writing to string cannot fail");
    writeln!(text, "|---|---|").expect("writing to string cannot fail");
    writeln!(text, "| Score | `{:.0}/100` |", packet.summary.score)
        .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Work queue | `{}` issue(s), `{}` high severity, `{}` need fixtures |",
        packet.top_issues.len(),
        high,
        fixture_required
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Command drill-down | `{}` |",
        packet.source_artifacts.command_index.display()
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");

    for (index, issue) in packet
        .top_issues
        .iter()
        .take(PERSONA_FINDING_LIMIT)
        .enumerate()
    {
        render_maintainer_action_item(
            text,
            &packet.source_artifacts.artifact_dir,
            issue,
            index + 1,
        );
    }
}

pub(super) fn ci_action_text(persona: Persona, issue: &Issue) -> String {
    match issue.confidence {
        IssueConfidence::NeedsFixture => {
            format!(
                "{} {}",
                persona_issue_action(persona, issue),
                fixture_hint(issue)
            )
        }
        IssueConfidence::Blocked => {
            format!(
                "{} Document or provision the runtime precondition before treating affected commands as available.",
                persona_issue_action(persona, issue)
            )
        }
        IssueConfidence::Inferred => {
            format!(
                "{} Confirm the candidate set before making broad CLI changes.",
                persona_issue_action(persona, issue)
            )
        }
        IssueConfidence::Observed | IssueConfidence::Advisory => {
            persona_issue_action(persona, issue).to_owned()
        }
    }
}

pub(super) fn issue_meaning(issue: &Issue) -> &'static str {
    match issue.confidence {
        IssueConfidence::Observed if issue.category == ActionCategory::Safety => {
            "CLIARE directly saw this runtime behavior; review the evidence and decide whether to fix, allow, or block it."
        }
        IssueConfidence::Observed => {
            "CLIARE directly saw this behavior; treat it as a concrete contract gap."
        }
        IssueConfidence::Blocked => {
            "CLIARE reached this area, but setup or required inputs blocked safe confirmation."
        }
        IssueConfidence::NeedsFixture => {
            "CLIARE found the contract, but needs safe sample data before it can validate it."
        }
        IssueConfidence::Inferred if issue_has_runtime_confirmed_commands(issue) => {
            "CLIARE saw real commands, but this specific gap still needs confirmation."
        }
        IssueConfidence::Inferred => {
            "CLIARE inferred this from help text; confirm it before routing agents or changing broad behavior."
        }
        IssueConfidence::Advisory => {
            "This is optional compatibility or polish; handle it after blockers and concrete failures."
        }
    }
}

pub(super) fn fixture_hint(issue: &Issue) -> &'static str {
    if issue
        .affected_commands
        .iter()
        .any(|command| !command.required_positionals.is_empty())
    {
        "Add a safe sample operand or fixture profile so CLIARE can validate the advertised contract."
    } else {
        "Add a safe fixture or metadata path so CLIARE can validate the advertised contract."
    }
}

pub(super) fn issue_where_label(issue: &Issue) -> String {
    if issue.affected_commands.is_empty() {
        return "scorecard-level finding".to_owned();
    }
    let mut commands = issue_command_samples(issue);
    let mut labels = commands
        .drain(..)
        .take(3)
        .map(|command| format!("`{}`", command_path_label(&command.path)))
        .collect::<Vec<_>>();
    if issue.affected_commands.len() > 3 {
        labels.push(format!("... {} more", issue.affected_commands.len() - 3));
    }
    labels.join(", ")
}

pub(super) fn maintainer_agent_outcome(issue: &Issue) -> String {
    let outcome = match issue.agent_readiness_area {
        AgentReadinessArea::OutputContracts => {
            "Agents cannot safely read command results, so they may guess at text output or drop state."
        }
        AgentReadinessArea::Preconditions => {
            "Agents waste discovery loops because they cannot tell whether a command is missing or just needs setup."
        }
        AgentReadinessArea::CommandDiscovery | AgentReadinessArea::HelpCoverage => {
            "Agents waste discovery loops because the command surface is not clearly confirmable."
        }
        AgentReadinessArea::Compatibility => {
            "Agents may try an optional navigation path and spend extra retries before falling back to canonical help."
        }
        AgentReadinessArea::Diagnostics => {
            "Agents cannot repair failed command attempts without guessing at the next command."
        }
        AgentReadinessArea::Execution => {
            "Agents cannot route to this command confidently because execution behavior is not reliable enough."
        }
        AgentReadinessArea::Safety => {
            "Agents need extra policy review because the run touched behavior with safety or side-effect implications."
        }
        AgentReadinessArea::Coverage => {
            "Maintainers cannot trust that the measured surface is complete enough for agent routing decisions."
        }
        AgentReadinessArea::Policy => {
            "Agents and CI need an explicit policy decision before this behavior can be treated as allowed."
        }
        AgentReadinessArea::Publishing => {
            "Public claims may overstate what the current evidence actually proves."
        }
        AgentReadinessArea::Calibration => {
            "Benchmark or calibration users may label the run incorrectly without a clearer decision."
        }
    };

    if issue.impact.trim().is_empty() {
        outcome.to_owned()
    } else {
        format!("{outcome} {}", issue.impact)
    }
}

pub(super) fn maintainer_fix_text(issue: &Issue) -> String {
    format!(
        "{} {}",
        persona_issue_action(Persona::Maintainer, issue),
        issue.recommendation
    )
}

pub(super) fn render_maintainer_action_item(
    text: &mut String,
    artifact_dir: &Path,
    issue: &Issue,
    priority: usize,
) {
    writeln!(text, "### {}. {}", priority, escape_markdown(&issue.title))
        .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "- Issue: `{}` (`{}`, {} affected).",
        escape_markdown(&issue.id),
        issue.severity.label(),
        issue.affected_commands.len()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Agent outcome: {}",
        escape_markdown(&maintainer_agent_outcome(issue))
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Fix: {}",
        escape_markdown(&maintainer_fix_text(issue))
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- If acceptable: {}",
        escape_markdown(&maintainer_disposition_text(artifact_dir, issue))
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Evidence: {}",
        escape_markdown(&maintainer_evidence_text(issue))
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Verify: `{}`",
        escape_markdown(&issue.verification.command)
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn maintainer_disposition_text(artifact_dir: &Path, issue: &Issue) -> String {
    if let Some(disposition) = &issue.disposition {
        return format!(
            "Already dispositioned as `{}`: {}",
            disposition.status.label(),
            disposition.reason
        );
    }

    format!(
        "record a disposition: `{}`",
        maintainer_disposition_command(artifact_dir, issue)
    )
}

pub(super) fn maintainer_disposition_command(artifact_dir: &Path, issue: &Issue) -> String {
    let status = match issue.confidence {
        IssueConfidence::NeedsFixture => "needs-fixture",
        IssueConfidence::Advisory => "deferred",
        _ => "intentional",
    };
    let reason = match issue.confidence {
        IssueConfidence::NeedsFixture => "Needs safe fixture operands before CI can judge it.",
        IssueConfidence::Blocked => {
            "Expected precondition for this measured profile; document the required setup."
        }
        IssueConfidence::Advisory => "Optional compatibility; not part of the current fix queue.",
        _ => "Reviewed and accepted for this measured profile.",
    };

    format!(
        "cliare issues mark {} --out {} --status {} --reason {}",
        shell_arg(&issue.id),
        shell_arg(&artifact_dir.display().to_string()),
        status,
        shell_arg(reason)
    )
}

pub(super) fn maintainer_evidence_text(issue: &Issue) -> String {
    let command_part = match issue.affected_commands.len() {
        0 => "No affected commands listed.".to_owned(),
        1 => "1 affected command.".to_owned(),
        count => format!("{count} affected commands."),
    };
    let evidence_part = match issue.evidence.len() {
        0 => "Open the drill-down for context.".to_owned(),
        1 => "1 evidence reference.".to_owned(),
        count => format!("{count} evidence references."),
    };
    format!("{command_part} {evidence_part}")
}
