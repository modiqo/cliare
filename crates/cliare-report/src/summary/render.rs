use std::fmt::Write;

use crate::report_format::escape_markdown;

use super::model::{FindingBrief, MeasurementSummaryPacket, contract_label};

pub(super) fn render_markdown(packet: &MeasurementSummaryPacket) -> String {
    let mut text = String::new();

    writeln!(text, "# CLIARE Measurement Summary").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "- Target: `{}`", packet.target.requested)
        .expect("writing to string cannot fail");
    writeln!(text, "- Resolved binary: `{}`", packet.target.resolved)
        .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Score: `{:.0}/100`; harness shape confidence: `{:.0}/100`",
        packet.score.total, packet.score.shape_confidence
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Model: `{}` (`{}`), measured weight `{:.2}/{:.2}`",
        packet.score.model,
        packet.score.status,
        packet.score.measured_weight,
        packet.score.max_weight
    )
    .expect("writing to string cannot fail");
    writeln!(text, "- Artifact dir: `{}`", packet.artifact_dir.display())
        .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");

    render_interpretation(&mut text, packet);
    render_evidence_snapshot(&mut text, packet);
    render_agent_navigation(&mut text, packet);
    render_findings(&mut text, packet);
    render_next_actions(&mut text, packet);
    render_caveats(&mut text, packet);

    text
}

fn render_interpretation(text: &mut String, packet: &MeasurementSummaryPacket) {
    writeln!(text, "## Interpretation").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    for item in &packet.interpretation {
        writeln!(text, "- {}", escape_markdown(item)).expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn render_evidence_snapshot(text: &mut String, packet: &MeasurementSummaryPacket) {
    let surface = &packet.command_surface;
    let traversal = &packet.traversal;

    writeln!(text, "## Evidence Snapshot").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "| Area | Value |").expect("writing to string cannot fail");
    writeln!(text, "|---|---:|").expect("writing to string cannot fail");
    writeln!(
        text,
        "| Traversal | `{}`; `{}`; depth `{}/{}`; probes `{}/{}` |",
        if traversal.profile_complete {
            "complete"
        } else {
            "incomplete"
        },
        traversal.stop_reason,
        traversal.observed_depth,
        traversal.max_depth,
        traversal.probes_completed,
        traversal.max_probes
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Command suitability | ready `{}`, conditional `{}`, fixture `{}`, blocked `{}`, candidate `{}` |",
        surface.ready,
        surface.conditional,
        surface.needs_fixture,
        surface.blocked,
        surface.candidate
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Runtime recognition | confirmed `{}` of `{}` discovered; indexed `{}`; precondition-blocked `{}` |",
        surface.runtime_confirmed,
        surface.discovered,
        surface.total,
        surface.precondition_blocked
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Help extraction | `{:.1}%` from `{}` help probes |",
        surface.parser_extraction_rate * 100.0,
        surface.help_text_probes
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Output contracts | parse successes `{}/{}`; blocked `{}` |",
        surface.output_mode_parse_successes,
        surface.machine_readable_output_contracts,
        surface.output_mode_precondition_blocked
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Preconditions | blocked probes `{}`, actionable `{}` |",
        surface.precondition_blocked_probes, surface.actionable_precondition_probes
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "| Side effects | files `{}`, probes `{}`, credential-like `{}` |",
        surface.side_effect_files_total,
        surface.side_effect_probe_count,
        surface.credential_like_side_effects
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
}

fn render_agent_navigation(text: &mut String, packet: &MeasurementSummaryPacket) {
    writeln!(text, "## Agent Navigation Evidence").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(text, "Status: `{}`", packet.agent_navigation_status)
        .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    if packet.agent_navigation.is_empty() {
        writeln!(text, "No agent-navigation metrics were present.")
            .expect("writing to string cannot fail");
        writeln!(text).expect("writing to string cannot fail");
        return;
    }

    writeln!(text, "| Capability | Score | Evidence | Status |")
        .expect("writing to string cannot fail");
    writeln!(text, "|---|---:|---:|---|").expect("writing to string cannot fail");
    for metric in &packet.agent_navigation {
        let score = metric
            .score
            .map_or_else(|| "n/a".to_owned(), |score| format!("{score:.0}/100"));
        writeln!(
            text,
            "| `{}` | `{}` | `{}/{}` | `{}` |",
            metric.capability, score, metric.numerator, metric.denominator, metric.status
        )
        .expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn render_findings(text: &mut String, packet: &MeasurementSummaryPacket) {
    writeln!(text, "## Findings").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    if packet.top_findings.is_empty() {
        writeln!(text, "No open findings were present.").expect("writing to string cannot fail");
        writeln!(text).expect("writing to string cannot fail");
        return;
    }

    for (index, finding) in packet.top_findings.iter().enumerate() {
        render_finding(text, index + 1, finding);
    }
}

fn render_finding(text: &mut String, index: usize, finding: &FindingBrief) {
    writeln!(
        text,
        "### {}. {} `{}`",
        index,
        escape_markdown(&finding.severity),
        escape_markdown(&finding.id)
    )
    .expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "- Assessment: {}",
        escape_markdown(&finding.assessment)
    )
    .expect("writing to string cannot fail");
    writeln!(text, "- Category: `{}`", finding.category).expect("writing to string cannot fail");
    writeln!(text, "- Meaning: {}", escape_markdown(&finding.meaning))
        .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Associated commands: `{}` affected; `{}` shown below",
        finding.affected_count,
        finding.associated_commands.len()
    )
    .expect("writing to string cannot fail");
    writeln!(
        text,
        "- Suggested remedy: {}",
        escape_markdown(&finding.suggested_remedy)
    )
    .expect("writing to string cannot fail");
    if !finding.associated_commands.is_empty() {
        writeln!(text, "- Command associations:").expect("writing to string cannot fail");
        for example in &finding.associated_commands {
            writeln!(
                text,
                "  - `{}`: `{}`; {}",
                escape_markdown(&example.command),
                escape_markdown(&example.state),
                escape_markdown(&example.reason)
            )
            .expect("writing to string cannot fail");
            if !example.required_positionals.is_empty() {
                writeln!(
                    text,
                    "    - Required positionals: `{}`",
                    example.required_positionals.join("`, `")
                )
                .expect("writing to string cannot fail");
            }
            if !example.preconditions.is_empty() {
                writeln!(
                    text,
                    "    - Preconditions: `{}`",
                    example.preconditions.join("`, `")
                )
                .expect("writing to string cannot fail");
            }
            if !example.output_contracts.is_empty() {
                let contracts = example
                    .output_contracts
                    .iter()
                    .map(contract_label)
                    .collect::<Vec<_>>()
                    .join("; ");
                writeln!(
                    text,
                    "    - Output contracts: {}",
                    escape_markdown(&contracts)
                )
                .expect("writing to string cannot fail");
            }
        }
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn render_next_actions(text: &mut String, packet: &MeasurementSummaryPacket) {
    writeln!(text, "## Next Actions").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    for action in &packet.next_actions {
        writeln!(text, "- {}", escape_markdown(action)).expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

fn render_caveats(text: &mut String, packet: &MeasurementSummaryPacket) {
    writeln!(text, "## Caveats").expect("writing to string cannot fail");
    writeln!(text).expect("writing to string cannot fail");
    for caveat in &packet.caveats {
        writeln!(text, "- {}", escape_markdown(caveat)).expect("writing to string cannot fail");
    }
    writeln!(text).expect("writing to string cannot fail");
}

#[cfg(test)]
mod tests {
    use super::super::model::{FindingExample, OutputContractBrief};
    use super::*;

    #[test]
    fn finding_markdown_renders_report_contract_fields() {
        let finding = FindingBrief {
            id: "issue.test".to_owned(),
            status: "open".to_owned(),
            severity: "high".to_owned(),
            category: "output".to_owned(),
            assessment: "JSON mode did not parse".to_owned(),
            meaning: "Agents cannot safely read command output.".to_owned(),
            suggested_remedy: "Return valid JSON for --json.".to_owned(),
            affected_count: 2,
            associated_commands: vec![FindingExample {
                command: "tool inspect".to_owned(),
                state: "runtime_confirmed".to_owned(),
                reason: "advertised output mode failed validation".to_owned(),
                required_positionals: vec!["id".to_owned()],
                preconditions: Vec::new(),
                output_contracts: vec![OutputContractBrief {
                    mode: "json".to_owned(),
                    flag_name: Some("--json".to_owned()),
                    status: "probe_failed".to_owned(),
                    diagnostic: None,
                    skip_reason: None,
                    suggested_validation: None,
                }],
            }],
        };
        let mut text = String::new();

        render_finding(&mut text, 1, &finding);

        assert!(text.contains("- Assessment: JSON mode did not parse"));
        assert!(text.contains("- Meaning: Agents cannot safely read command output."));
        assert!(text.contains("- Associated commands: `2` affected; `1` shown below"));
        assert!(text.contains("- Suggested remedy: Return valid JSON for --json."));
        assert!(text.contains("- Command associations:"));
        assert!(text.contains("`tool inspect`: `runtime_confirmed`"));
    }
}
