use std::cmp::Ordering;
use std::fmt::Write as _;
use std::path::PathBuf;

use crate::artifact_guide::ArtifactGuideSummary;
use crate::report_format::{command_path_label, escape_markdown, output_mode_label, shell_words};
use crate::report_model::*;

const COMMAND_SAMPLE_LIMIT: usize = 5;
const PERSONA_FINDING_LIMIT: usize = 5;
const PERSONA_COMMAND_SAMPLE_LIMIT: usize = 3;
const PERSONA_EVIDENCE_SAMPLE_LIMIT: usize = 3;
const COMMAND_GUIDANCE_SAMPLE_LIMIT: usize = 10;

pub(crate) fn render_markdown(packet: &PersonaOutcomePacket) -> String {
    let mut text = String::new();
    writeln!(&mut text, "# CLIARE {} Report", packet.persona_title)
        .expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(&mut text, "{}", packet.primary_question).expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");

    render_persona_decision(&mut text, packet);
    render_plain_english_guide(&mut text);
    render_ci_action_brief(&mut text, packet);
    render_persona_score_summary(&mut text, packet);
    render_persona_findings(&mut text, packet);
    render_reviewed_decisions(&mut text, packet);
    render_persona_command_guidance(&mut text, packet);
    render_run_recommendations(&mut text, packet);
    render_artifact_navigation(&mut text, packet);
    render_notes(&mut text, packet);

    text
}

pub(crate) fn render_drilldown_markdown(packet: &ReportDrilldownPacket) -> String {
    let mut text = String::new();
    writeln!(&mut text, "# CLIARE {} Drilldown", packet.persona_title)
        .expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(&mut text, "{}", escape_markdown(packet.primary_question))
        .expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "| Field | Value |\n|---|---|\n| Score | `{:.0}/100` |\n| Filter | `{}` `{}` |\n| Issues | `{}` |\n| Evidence attached | `{}` |\n| Command drill-down | `{}` |",
        packet.summary.score,
        drilldown_filter_kind_label(packet.filter.kind),
        escape_markdown(&packet.filter.value),
        packet.issues.len(),
        packet.evidence_included,
        packet.source_artifacts.command_index.display()
    )
    .expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");

    if packet.issues.is_empty() {
        writeln!(
            &mut text,
            "No issues matched this filter for the selected persona."
        )
        .expect("writing to string cannot fail");
        return text;
    }

    writeln!(
        &mut text,
        "| Area | Severity | Meaning | Disposition | Affected | Issue | Action |"
    )
    .expect("writing to string cannot fail");
    writeln!(&mut text, "|---|---|---|---|---:|---|---|").expect("writing to string cannot fail");
    for issue in &packet.issues {
        writeln!(
            &mut text,
            "| {} | `{}` | {} | `{}` | {} | `{}` {} | {} |",
            escape_markdown(issue.agent_readiness_area.label()),
            issue.severity.label(),
            escape_markdown(issue_meaning(issue)),
            issue_disposition_label(issue),
            issue.affected_commands.len(),
            escape_markdown(&issue.id),
            escape_markdown(&issue.title),
            escape_markdown(persona_issue_action(packet.persona, issue))
        )
        .expect("writing to string cannot fail");
    }
    writeln!(&mut text).expect("writing to string cannot fail");

    writeln!(&mut text, "## Drill-Down").expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    for issue in &packet.issues {
        if packet.persona == Persona::Maintainer {
            render_maintainer_finding(&mut text, issue);
        } else {
            render_named_finding(&mut text, packet.persona, issue);
        }
    }

    text
}

fn drilldown_filter_kind_label(kind: ReportDrilldownFilterKind) -> &'static str {
    match kind {
        ReportDrilldownFilterKind::Area => "area",
        ReportDrilldownFilterKind::Issue => "issue",
    }
}

fn render_persona_decision(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## Decision").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "{}", escape_markdown(&persona_decision(packet)))
        .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

fn render_plain_english_guide(text: &mut String) {
    writeln!(text, "## Plain-English Guide").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "Use this section to decode the report before opening JSON artifacts. The tables below say what each label means and what a reviewer should do next."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Report label | Plain meaning | First action |")
        .expect("writing to string cannot fail");
    writeln!(text, "|---|---|---|").expect("writing to string cannot fail");
    for (label, meaning, action) in [
        (
            "observed",
            "CLIARE directly saw this behavior at runtime.",
            "Treat it as concrete evidence and fix or explicitly accept it.",
        ),
        (
            "blocked",
            "CLIARE reached the command or probe, but setup, auth, context, fixtures, or dependencies stopped safe confirmation.",
            "Document or provide the missing precondition before exposing the command to agents.",
        ),
        (
            "needs_fixture",
            "CLIARE found a contract, but needs safe sample input before it can validate it.",
            "Add a safe fixture, sample operand, or dry-run validation path.",
        ),
        (
            "inferred",
            "CLIARE inferred this from help/layout evidence, but runtime confirmation is incomplete.",
            "Confirm it before changing broad behavior or routing agents through it.",
        ),
        (
            "advisory",
            "This is useful polish, not the first blocker.",
            "Handle it after concrete failures, blockers, and fixture gaps.",
        ),
    ] {
        writeln!(
            text,
            "| `{}` | {} | {} |",
            label,
            escape_markdown(meaning),
            escape_markdown(action)
        )
        .expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "| Command state | Plain meaning | Harness treatment |"
    )
    .expect("writing to string cannot fail");
    writeln!(text, "|---|---|---|").expect("writing to string cannot fail");
    for (state, meaning, treatment) in [
        (
            "ready",
            "Runtime-confirmed and no blocking gaps were found.",
            "Candidate for automatic routing, subject to local policy.",
        ),
        (
            "conditional",
            "The command can work when a known condition is satisfied.",
            "Expose only when the harness can satisfy that condition.",
        ),
        (
            "needs_fixture",
            "The command needs safe input or sample data before validation.",
            "Do not route automatically until fixtures exist.",
        ),
        (
            "blocked",
            "A required precondition blocked safe confirmation.",
            "Treat as unavailable until the precondition is provisioned.",
        ),
        (
            "candidate",
            "The command is inferred but not runtime-confirmed.",
            "Do not expose automatically.",
        ),
    ] {
        writeln!(
            text,
            "| `{}` | {} | {} |",
            state,
            escape_markdown(meaning),
            escape_markdown(treatment)
        )
        .expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn render_ci_action_brief(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## CI Action Brief").expect("writing to string cannot fail");
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
        "Treat the rows below as agent-readiness work areas. Start with concrete contract gaps before advisory compatibility work, and use `command-index.json` only when you need exact command parameters or evidence pointers."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Field | Value |").expect("writing to string cannot fail");
    writeln!(text, "|---|---|").expect("writing to string cannot fail");
    writeln!(text, "| Score | `{:.0}/100` |", packet.summary.score)
        .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Work queue | `{}` area(s), `{}` high severity, `{}` need fixtures |",
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

    writeln!(
        text,
        "| Area | Severity | Meaning | Affected | Do this | Verify |"
    )
    .expect("writing to string cannot fail");
    writeln!(text, "|---|---|---|---:|---|---|").expect("writing to string cannot fail");
    for issue in packet.top_issues.iter().take(PERSONA_FINDING_LIMIT) {
        writeln!(
            text,
            "| {} | `{}` | {} | {} | {} | `{}` |",
            escape_markdown(issue.agent_readiness_area.label()),
            issue.severity.label(),
            escape_markdown(issue_meaning(issue)),
            issue.affected_commands.len(),
            escape_markdown(persona_issue_action(packet.persona, issue)),
            escape_markdown(&issue.verification.command)
        )
        .expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn render_persona_score_summary(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## Score Context").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Field | Value |").expect("writing to string cannot fail");
    writeln!(text, "|---|---|").expect("writing to string cannot fail");
    writeln!(
        text,
        "| Target | `{}` |",
        escape_markdown(&packet.target.requested.display().to_string())
    )
    .expect("writing to string cannot fail");
    writeln!(text, "| Score | `{:.0}/100` |", packet.summary.score)
        .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Runtime confirmation | `{}/{}` commands ({}) |",
        packet.summary.commands_runtime_confirmed,
        packet.summary.commands_discovered,
        percent(packet.coverage.command_confirmation_rate)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Output contracts | `{}` machine-readable, `{}` parse successes |",
        packet.summary.machine_readable_output_contracts,
        packet.summary.output_mode_parse_successes
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Agent routing | `{}` ready, `{}` conditional, `{}` need fixtures, `{}` blocked, `{}` candidates |",
        packet.summary.commands_ready,
        packet.summary.commands_conditional,
        packet.summary.commands_needs_fixture,
        packet.summary.commands_blocked,
        packet.summary.commands_candidate
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Preconditions | `{}` blocked commands, `{}` blocked probes |",
        packet.summary.commands_precondition_blocked, packet.summary.precondition_blocked_probes
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Side effects | `{}` file changes, `{}` credential-like paths |",
        packet.summary.side_effect_files_total, packet.summary.credential_like_side_effects
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Coverage | `{}`; depth `{}/{}`, probes `{}/{}` |",
        packet.summary.traversal_stop_reason,
        packet.summary.observed_max_depth,
        packet.summary.max_depth,
        packet.summary.probes_completed,
        packet.summary.max_probes
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

fn ci_action_text(persona: Persona, issue: &Issue) -> String {
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

fn issue_meaning(issue: &Issue) -> &'static str {
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

fn fixture_hint(issue: &Issue) -> &'static str {
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

fn issue_where_label(issue: &Issue) -> String {
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

fn render_persona_findings(text: &mut String, packet: &PersonaOutcomePacket) {
    let heading = if packet.persona == Persona::Maintainer {
        "## Agent Readiness Findings"
    } else {
        "## Priority Findings"
    };
    writeln!(text, "{heading}").expect("writing to string cannot fail");
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

    if packet.persona == Persona::Maintainer {
        writeln!(
            text,
            "Start with the area table, then open a drill-down only when you need command samples, evidence, or verification details."
        )
        .expect("writing to string cannot fail");
        writeln!(text).expect("writing to string cannot fail");
        writeln!(
            text,
            "| Area | Severity | Meaning | Disposition | Affected | Issue | Action |"
        )
        .expect("writing to string cannot fail");
        writeln!(text, "|---|---|---|---|---:|---|---|").expect("writing to string cannot fail");
        for issue in packet.top_issues.iter().take(PERSONA_FINDING_LIMIT) {
            render_maintainer_finding_row(text, issue);
        }
    } else {
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
        if packet.persona == Persona::Maintainer {
            render_maintainer_finding(text, issue);
        } else {
            render_persona_finding(text, packet.persona, issue, index + 1);
        }
    }
}

fn render_maintainer_finding_row(text: &mut String, issue: &Issue) {
    writeln!(
        text,
        "| {} | `{}` | {} | `{}` | {} | `{}` {} | {} |",
        escape_markdown(issue.agent_readiness_area.label()),
        issue.severity.label(),
        escape_markdown(issue_meaning(issue)),
        issue_disposition_label(issue),
        issue.affected_commands.len(),
        escape_markdown(&issue.id),
        escape_markdown(&issue.title),
        escape_markdown(persona_issue_action(Persona::Maintainer, issue))
    )
    .expect("writing to string cannot fail");
}

fn render_persona_finding_row(text: &mut String, persona: Persona, issue: &Issue, priority: usize) {
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

fn render_maintainer_finding(text: &mut String, issue: &Issue) {
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
    render_finding_body(text, Persona::Maintainer, issue);
    writeln!(text, "</details>").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

fn render_named_finding(text: &mut String, persona: Persona, issue: &Issue) {
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

fn render_persona_finding(text: &mut String, persona: Persona, issue: &Issue, priority: usize) {
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

fn render_finding_body(text: &mut String, persona: Persona, issue: &Issue) {
    writeln!(
        text,
        "- Issue: `{}` (`{}`, `{}`, `{}`)",
        escape_markdown(&issue.id),
        issue.severity.label(),
        issue.category.label(),
        issue.confidence.label()
    )
    .expect("writing to string cannot fail");
    writeln!(text, "- Meaning: {}", escape_markdown(issue_meaning(issue)))
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
        "- Role action: {}",
        escape_markdown(persona_issue_action(persona, issue))
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Recommended change: {}",
        escape_markdown(&issue.recommendation)
    )
    .expect("writing to string cannot fail");
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

fn render_reviewed_decisions(text: &mut String, packet: &PersonaOutcomePacket) {
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

fn issue_disposition_label(issue: &Issue) -> &'static str {
    issue
        .disposition
        .as_ref()
        .map(|disposition| disposition.status.label())
        .unwrap_or("open")
}

fn reviewed_harness_guidance(issue: &Issue) -> &'static str {
    match issue
        .disposition
        .as_ref()
        .map(|disposition| disposition.status)
    {
        Some(crate::issue_disposition::IssueDispositionStatus::Intentional) => {
            "follow the recorded project convention"
        }
        Some(crate::issue_disposition::IssueDispositionStatus::NotApplicable) => {
            "ignore for this CLI unless local policy says otherwise"
        }
        Some(crate::issue_disposition::IssueDispositionStatus::FalsePositive) => {
            "do not route around this finding"
        }
        Some(crate::issue_disposition::IssueDispositionStatus::AcceptedRisk) => {
            "route only when the harness can tolerate the recorded risk"
        }
        Some(crate::issue_disposition::IssueDispositionStatus::Deferred) => {
            "treat as known debt rather than an immediate blocker"
        }
        Some(crate::issue_disposition::IssueDispositionStatus::Open)
        | Some(crate::issue_disposition::IssueDispositionStatus::Accepted)
        | Some(crate::issue_disposition::IssueDispositionStatus::NeedsFixture)
        | None => issue.agent_readiness_area.agent_impact(),
    }
}

fn command_section_heading(issue: &Issue) -> &'static str {
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

fn command_overflow_label(issue: &Issue) -> &'static str {
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

fn evidence_section_heading(issue: &Issue) -> &'static str {
    match issue.confidence {
        IssueConfidence::Inferred | IssueConfidence::Advisory => "Evidence references",
        _ => "Evidence to open",
    }
}

fn issue_command_samples(issue: &Issue) -> Vec<&IssueCommand> {
    let mut commands = issue.affected_commands.iter().collect::<Vec<_>>();
    commands.sort_by(issue_command_sample_order);
    commands
}

fn issue_has_runtime_confirmed_commands(issue: &Issue) -> bool {
    issue
        .affected_commands
        .iter()
        .any(|command| command.state == "runtime_confirmed")
}

fn issue_command_sample_order(left: &&IssueCommand, right: &&IssueCommand) -> Ordering {
    right
        .confidence
        .unwrap_or(f64::NEG_INFINITY)
        .total_cmp(&left.confidence.unwrap_or(f64::NEG_INFINITY))
        .then_with(|| left.path.len().cmp(&right.path.len()))
        .then_with(|| left.path.cmp(&right.path))
}

fn render_issue_command_sample(text: &mut String, command: &IssueCommand) {
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

fn required_positionals_label(required_positionals: &[String]) -> String {
    required_positionals
        .iter()
        .map(|name| format!("<{name}>"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_issue_evidence_sample(text: &mut String, evidence: &IssueEvidence) {
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

fn render_persona_command_guidance(text: &mut String, packet: &PersonaOutcomePacket) {
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

fn render_command_guidance_row(
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

fn render_run_recommendations(text: &mut String, packet: &PersonaOutcomePacket) {
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

fn render_artifact_navigation(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## Working Files").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Artifact | Use |").expect("writing to string cannot fail");
    writeln!(text, "|---|---|").expect("writing to string cannot fail");
    writeln!(
        text,
        "| `{}` | Full issue ledger for remediation and complete affected-command lists. |",
        packet
            .source_artifacts
            .artifact_dir
            .join("issues.json")
            .display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| `{}` | Human-readable issue ledger. |",
        packet
            .source_artifacts
            .artifact_dir
            .join("issues.md")
            .display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| `{}` | Persona packet with full command health. |",
        packet
            .source_artifacts
            .artifact_dir
            .join(format!("persona-{}.json", packet.persona.label()))
            .display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| `{}` | Command-centric lookup table with suitability, parameters, preconditions, output contracts, gaps, and evidence pointers. |",
        packet.source_artifacts.command_index.display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| `{}` | Human-readable command index table. |",
        packet.source_artifacts.command_index_markdown.display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| `{}` | Inferred command catalog, flags, gaps, and output contracts. |",
        packet.source_artifacts.shape.display()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| `{}` | Runtime evidence log used to verify claims. |",
        packet.source_artifacts.evidence.display()
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Evidence summary | Value |").expect("writing to string cannot fail");
    writeln!(text, "|---|---:|").expect("writing to string cannot fail");
    writeln!(
        text,
        "| Probes scheduled | {} |",
        packet.evidence_summary.probes_scheduled
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Processes completed | {} |",
        packet.evidence_summary.processes_completed
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Probe failures | {} |",
        packet.evidence_summary.probe_failures_total
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Side-effect records | {} |",
        packet.evidence_summary.side_effects_total
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

fn render_notes(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## Notes").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    for note in &packet.notes {
        writeln!(text, "- `{}` {}", note.level, escape_markdown(&note.text))
            .expect("writing to string cannot fail");
    }
}

fn persona_decision(packet: &PersonaOutcomePacket) -> String {
    let high_issues = packet
        .top_issues
        .iter()
        .filter(|issue| issue.severity == ActionSeverity::High)
        .count();
    let fixture_issues = packet
        .top_issues
        .iter()
        .filter(|issue| issue.confidence == IssueConfidence::NeedsFixture)
        .count();
    match packet.persona {
        Persona::Maintainer => {
            if packet.top_issues.is_empty() {
                "No maintainer-prioritized fixes are currently blocking the measured posture. Keep the issue ledger in CI and watch for drift.".to_owned()
            } else {
                format!(
                    "Fix the top CLI contract gaps before treating this surface as stable for agents. This report prioritizes {} issue(s), including {} high-severity item(s) and {} fixture-required output contract item(s).",
                    packet.top_issues.len(),
                    high_issues,
                    fixture_issues
                )
            }
        }
        Persona::Harness => {
            format!(
                "Expose the {} ready command(s) first. Treat {} conditional command(s), {} fixture-required command(s), {} blocked command(s), and {} candidate command(s) as policy or remediation work before automatic routing.",
                packet.summary.commands_ready,
                packet.summary.commands_conditional,
                packet.summary.commands_needs_fixture,
                packet.summary.commands_blocked,
                packet.summary.commands_candidate
            )
        }
        Persona::Platform => {
            "Use this run as CI feedback, not a final gate, until policy thresholds and side-effect rules are explicit. Convert the top findings into guard policy before enforcing readiness across teams.".to_owned()
        }
        Persona::Security => {
            if packet.summary.side_effect_files_total > 0 || packet.summary.credential_like_side_effects > 0 {
                if packet.summary.credential_like_side_effects > 0 {
                    "Require review before approving unrestricted safe-probe use. The measurement observed filesystem side effects, including credential-like paths, that need policy treatment.".to_owned()
                } else {
                    "Require review before approving unrestricted safe-probe use. The measurement observed filesystem side effects that need policy treatment.".to_owned()
                }
            } else {
                "No credential-like side effects were observed, but approval still depends on traversal completeness, auth state, and fixture coverage.".to_owned()
            }
        }
        Persona::Oss => {
            "Publish the scorecard only with its profile, fingerprint, and caveats. Do not present the score as certified while score v0 and local calibration are still explicit.".to_owned()
        }
        Persona::Devrel => {
            "Use the report to teach concrete CLI improvements, not to market a raw score. Public claims should cite the specific measured contracts and known gaps.".to_owned()
        }
        Persona::Research => {
            "Treat this run as a labeled-candidate artifact. It is useful for replay and study only after command existence, output contracts, preconditions, and side effects are independently labeled.".to_owned()
        }
    }
}

fn persona_issue_action(persona: Persona, issue: &Issue) -> &'static str {
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

fn persona_command_guidance(persona: Persona) -> &'static str {
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

fn command_sample_list(commands: &[&CommandHealth], limit: usize) -> String {
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

fn command_health_sample_order(left: &&CommandHealth, right: &&CommandHealth) -> Ordering {
    right
        .confidence
        .total_cmp(&left.confidence)
        .then_with(|| left.path.len().cmp(&right.path.len()))
        .then_with(|| left.path.cmp(&right.path))
}

fn use_when_text(value: &str) -> String {
    value
        .trim()
        .strip_prefix("Use ")
        .unwrap_or_else(|| value.trim())
        .to_owned()
}

fn percent(value: f64) -> String {
    if value.is_finite() {
        format!("{:.1}%", value * 100.0)
    } else {
        "n/a".to_owned()
    }
}

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
    writeln!(text, "- Meaning: {}", escape_markdown(issue_meaning(issue)))
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
        "- Affected commands: `{}`",
        issue.affected_commands.len()
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "**Impact:** {}", escape_markdown(&issue.impact))
        .expect("writing to string cannot fail");
    writeln!(
        text,
        "**Why it matters:** {}",
        escape_markdown(&issue.why_it_matters)
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "**Recommended fix:** {}",
        escape_markdown(&issue.recommendation)
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

pub(crate) fn render_written_summary(
    packet: &PersonaOutcomePacket,
    markdown_path: Option<&PathBuf>,
    json_path: Option<&PathBuf>,
    guide_artifacts: Option<&ArtifactGuideSummary>,
) -> String {
    let mut text = String::new();
    writeln!(
        &mut text,
        "CLIARE {} outcome packet written",
        packet.persona.label()
    )
    .expect("writing to string cannot fail");
    writeln!(&mut text, "score: {:.0}/100", packet.summary.score)
        .expect("writing to string cannot fail");
    writeln!(&mut text, "action items: {}", packet.action_items.len())
        .expect("writing to string cannot fail");
    writeln!(&mut text, "top issues: {}", packet.top_issues.len())
        .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "command health entries: {}",
        packet.command_health.len()
    )
    .expect("writing to string cannot fail");
    writeln!(
        &mut text,
        "traversal complete: {}",
        packet.summary.traversal_complete
    )
    .expect("writing to string cannot fail");
    if let Some(path) = markdown_path {
        writeln!(&mut text, "markdown: {}", path.display()).expect("writing to string cannot fail");
    }
    if let Some(path) = json_path {
        writeln!(&mut text, "json: {}", path.display()).expect("writing to string cannot fail");
    }
    if let Some(path) = markdown_path {
        writeln!(
            &mut text,
            "issue ledger markdown: {}",
            path.with_file_name("issues.md").display()
        )
        .expect("writing to string cannot fail");
    }
    if let Some(path) = json_path {
        writeln!(
            &mut text,
            "issue ledger json: {}",
            path.with_file_name("issues.json").display()
        )
        .expect("writing to string cannot fail");
    }
    if let Some(artifacts) = guide_artifacts {
        writeln!(&mut text, "readme: {}", artifacts.readme_path.display())
            .expect("writing to string cannot fail");
        writeln!(
            &mut text,
            "agent guide: {}",
            artifacts.agent_skill_path.display()
        )
        .expect("writing to string cannot fail");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::{
        ci_action_text, command_section_heading, issue_meaning, persona_issue_action,
        render_maintainer_finding, render_maintainer_finding_row, render_persona_finding,
        render_persona_finding_row, use_when_text,
    };
    use crate::report_model::{
        ActionCategory, ActionSeverity, AgentReadinessArea, Issue, IssueCommand, IssueConfidence,
        IssueVerification, Persona,
    };

    #[test]
    fn inferred_runtime_confirmed_issues_are_commands_to_verify() {
        let issue = test_issue(
            IssueConfidence::Inferred,
            ActionCategory::Discovery,
            Some("runtime_confirmed"),
        );

        assert_eq!(command_section_heading(&issue), "Commands to verify");
        assert!(
            persona_issue_action(Persona::Maintainer, &issue)
                .starts_with("Verify the measured gap")
        );
    }

    #[test]
    fn inferred_unconfirmed_issues_are_candidate_examples() {
        let issue = test_issue(
            IssueConfidence::Inferred,
            ActionCategory::Discovery,
            Some("inferred"),
        );

        assert_eq!(
            command_section_heading(&issue),
            "Candidate examples to review"
        );
        assert!(
            persona_issue_action(Persona::Harness, &issue)
                .starts_with("Do not expose inferred candidates")
        );
    }

    #[test]
    fn harness_safety_action_is_profile_scoped() {
        let issue = test_issue(IssueConfidence::Observed, ActionCategory::Safety, None);

        let action = persona_issue_action(Persona::Harness, &issue);
        assert!(action.starts_with("Do not run this probe profile"));
        assert!(!action.contains("affected commands"));
    }

    #[test]
    fn persona_issue_markdown_is_table_row_with_drilldown() {
        let issue = test_issue(
            IssueConfidence::Observed,
            ActionCategory::Safety,
            Some("runtime_confirmed"),
        );

        let mut row = String::new();
        render_persona_finding_row(&mut row, Persona::Harness, &issue, 1);
        assert!(
            row.starts_with(
                "| P1 | CLIARE directly saw this runtime behavior; review the evidence"
            )
        );
        assert!(row.contains("`open` | 1 | `issue.test` Test issue"));
        assert!(row.contains("Do not run this probe profile"));

        let mut detail = String::new();
        render_persona_finding(&mut detail, Persona::Harness, &issue, 1);
        assert!(detail.contains("<details>"));
        assert!(detail.contains("<summary>P1: Test issue (`issue.test`)</summary>"));
        assert!(detail.contains("- Meaning: CLIARE directly saw this runtime behavior"));
        assert!(detail.contains("</details>"));
    }

    #[test]
    fn maintainer_issue_markdown_uses_agent_readiness_area() {
        let mut issue = test_issue(
            IssueConfidence::NeedsFixture,
            ActionCategory::Output,
            Some("runtime_confirmed"),
        );
        issue.agent_readiness_area = AgentReadinessArea::OutputContracts;

        let mut row = String::new();
        render_maintainer_finding_row(&mut row, &issue);
        assert!(row.starts_with("| Output Contracts | `medium` | CLIARE found the contract"));
        assert!(row.contains("`issue.test` Test issue"));
        assert!(row.contains("Add a safe validation path"));

        let mut detail = String::new();
        render_maintainer_finding(&mut detail, &issue);
        assert!(detail.contains("<summary>Output Contracts: Test issue (`issue.test`)</summary>"));
        assert!(detail.contains("- Meaning: CLIARE found the contract"));
        assert!(detail.contains("- Area: Output Contracts"));
        assert!(detail.contains("- Agent impact: Agents cannot reliably read command results."));
        assert!(!detail.contains("P1"));
    }

    #[test]
    fn issue_meaning_translates_report_conditions() {
        assert!(
            issue_meaning(&test_issue(
                IssueConfidence::Blocked,
                ActionCategory::Discovery,
                Some("runtime_confirmed"),
            ))
            .contains("setup or required inputs blocked")
        );
        assert!(
            issue_meaning(&test_issue(
                IssueConfidence::Inferred,
                ActionCategory::Discovery,
                None,
            ))
            .contains("inferred this from help text")
        );
    }

    #[test]
    fn ci_action_text_makes_fixture_work_actionable() {
        let mut issue = test_issue(
            IssueConfidence::NeedsFixture,
            ActionCategory::Output,
            Some("runtime_confirmed"),
        );
        issue.affected_commands[0]
            .required_positionals
            .push("persona".to_owned());

        let action = ci_action_text(Persona::Devrel, &issue);

        assert!(action.contains("safe sample operand or fixture profile"));
        assert!(action.contains("advertised contract"));
    }

    #[test]
    fn use_when_text_removes_redundant_prefix() {
        assert_eq!(
            use_when_text("Use before exposing a CLI subset to agents."),
            "before exposing a CLI subset to agents."
        );
        assert_eq!(
            use_when_text("whenever policy changes."),
            "whenever policy changes."
        );
    }

    fn test_issue(
        confidence: IssueConfidence,
        category: ActionCategory,
        command_state: Option<&str>,
    ) -> Issue {
        Issue {
            id: "issue.test".to_owned(),
            status: "open",
            severity: ActionSeverity::Medium,
            category,
            agent_readiness_area: AgentReadinessArea::Diagnostics,
            confidence,
            title: "Test issue".to_owned(),
            impact: "impact".to_owned(),
            why_it_matters: "why".to_owned(),
            recommendation: "recommendation".to_owned(),
            verification: IssueVerification {
                command: "cliare measure test".to_owned(),
                expected_change: "expected".to_owned(),
            },
            affected_commands: command_state
                .map(|state| {
                    vec![IssueCommand {
                        path: vec!["cmd".to_owned()],
                        argv: vec!["target".to_owned(), "cmd".to_owned()],
                        state: state.to_owned(),
                        confidence: Some(0.8),
                        summary: None,
                        required_positionals: Vec::new(),
                        output_contracts: Vec::new(),
                        reason: "reason".to_owned(),
                    }]
                })
                .unwrap_or_default(),
            evidence: Vec::new(),
            disposition: None,
            personas: Vec::new(),
            score_dimensions: Vec::new(),
        }
    }
}
