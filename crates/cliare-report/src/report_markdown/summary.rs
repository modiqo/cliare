use std::fmt::Write as _;

use crate::report_format::escape_markdown;
use crate::report_model::*;

pub(super) fn render_persona_decision(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## Decision").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "{}", escape_markdown(&persona_decision(packet)))
        .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn render_plain_english_guide(text: &mut String) {
    writeln!(text, "## Plain-English Guide").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "Use this section to decode the report before opening JSON artifacts. CLIARE reports evidence for agent navigation capabilities; it does not assume that a CLI is acceptable for automation just because an agent could explore it by trial and error."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "### Agent Evidence Perspective").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "| Capability | Evidence an agent can rely on | Developer implication |"
    )
    .expect("writing to string cannot fail");
    writeln!(text, "|---|---|---|").expect("writing to string cannot fail");
    for (capability, evidence, implication) in [
        (
            "Discovery",
            "Runtime-confirmed help, direct help-like output, or clear structured command rows.",
            "Make command groups and leaves discoverable without requiring account state or guesswork.",
        ),
        (
            "Grammar",
            "Stable usage lines, explicit operands, flag value names, and actionable missing-argument diagnostics.",
            "Give agents enough shape to construct calls instead of learning by invalid attempts.",
        ),
        (
            "Output",
            "Advertised machine-readable modes that were safely probed and parsed.",
            "Provide parseable output contracts for command results an agent must consume.",
        ),
        (
            "Recovery",
            "Nonzero invalid-input exits with diagnostics that identify the missing flag, operand, precondition, or next step.",
            "Make failures repairable; opaque errors turn agent use into repeated exploration.",
        ),
        (
            "Safety",
            "Help, version, diagnostic, and safe output probes with no unexpected persistent side effects.",
            "Keep discovery paths read-only or document and control expected writes.",
        ),
    ] {
        writeln!(
            text,
            "| {} | {} | {} |",
            escape_markdown(capability),
            escape_markdown(evidence),
            escape_markdown(implication)
        )
        .expect("writing to string cannot fail");
    }
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

pub(super) fn render_persona_score_summary(text: &mut String, packet: &PersonaOutcomePacket) {
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

pub(super) fn render_agent_navigation_evidence(text: &mut String, packet: &PersonaOutcomePacket) {
    writeln!(text, "## Agent Navigation Evidence").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "These metrics describe which navigation capabilities are backed by measured evidence for this run."
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "- Status: `{}`", packet.agent_navigation.status)
        .expect("writing to string cannot fail");
    if !packet.agent_navigation.limitations.is_empty() {
        writeln!(
            text,
            "- Limitations: {}",
            escape_markdown(&packet.agent_navigation.limitations.join("; "))
        )
        .expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");

    if packet.agent_navigation.dimensions.is_empty() {
        writeln!(
            text,
            "This artifact does not include agent navigation metrics."
        )
        .expect("writing to string cannot fail");
        writeln!(text).expect("writing to string cannot fail");
        return;
    }

    writeln!(
        text,
        "| Capability | Score | Evidence ratio | Status | What it means | Evidence | Limits |"
    )
    .expect("writing to string cannot fail");
    writeln!(text, "|---|---:|---:|---|---|---|---|").expect("writing to string cannot fail");
    for (capability, metric) in &packet.agent_navigation.dimensions {
        writeln!(
            text,
            "| `{}` | {} | `{}/{}` | `{}` | {} | {} | {} |",
            capability,
            metric_score_label(metric.score),
            metric.numerator,
            metric.denominator,
            metric.status,
            escape_markdown(&metric.rationale),
            escape_markdown(&list_or_none(&metric.evidence)),
            escape_markdown(&list_or_none(&metric.limitations))
        )
        .expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

pub(super) fn render_notes(text: &mut String, packet: &PersonaOutcomePacket) {
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
                "No maintainer-prioritized fixes are currently blocking the measured posture. Keep the issue ledger in CI and watch for drift; this means the measured evidence supports current agent navigation, not that every hidden surface is proven acceptable.".to_owned()
            } else {
                format!(
                    "Fix the top CLI contract gaps before treating this surface as evidence-backed for agents. Agents may still explore weak surfaces, but automatic use should require measured navigation evidence. This report prioritizes {} issue(s), including {} high-severity item(s) and {} fixture-required output contract item(s).",
                    packet.top_issues.len(),
                    high_issues,
                    fixture_issues
                )
            }
        }
        Persona::Harness => {
            format!(
                "Expose the {} ready command(s) first. Agents can explore beyond that, but automatic routing should require evidence, policy, or explicit remediation. Treat {} conditional command(s), {} fixture-required command(s), {} blocked command(s), and {} candidate command(s) as work before automatic routing.",
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

fn percent(value: f64) -> String {
    if value.is_finite() {
        format!("{:.1}%", value * 100.0)
    } else {
        "n/a".to_owned()
    }
}

fn metric_score_label(score: Option<f64>) -> String {
    score.map_or_else(
        || "`not measured`".to_owned(),
        |score| format!("`{score:.0}/100`"),
    )
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(", ")
    }
}
