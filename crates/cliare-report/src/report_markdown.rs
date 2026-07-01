use std::fmt::Write as _;

use crate::report_format::escape_markdown;
use crate::report_model::*;

mod actions;
mod findings;
mod guidance;
mod ledger;
mod navigation;
mod samples;
mod summary;
#[cfg(test)]
mod tests;
mod written;

pub(crate) use ledger::render_issue_ledger_markdown;
pub(crate) use written::render_written_summary;

const COMMAND_SAMPLE_LIMIT: usize = 5;
const PERSONA_FINDING_LIMIT: usize = 5;
const PERSONA_COMMAND_SAMPLE_LIMIT: usize = 3;
const PERSONA_EVIDENCE_SAMPLE_LIMIT: usize = 3;
const COMMAND_GUIDANCE_SAMPLE_LIMIT: usize = 10;

use actions::{issue_meaning, render_ci_action_brief};
use findings::{render_maintainer_finding, render_named_finding, render_persona_findings};
use guidance::{persona_issue_action, render_persona_command_guidance, render_run_recommendations};
use navigation::render_artifact_navigation;
use samples::{issue_disposition_label, render_reviewed_decisions};
use summary::{
    render_notes, render_persona_decision, render_persona_score_summary, render_plain_english_guide,
};

pub(crate) fn render_markdown(packet: &PersonaOutcomePacket) -> String {
    let mut text = String::new();
    writeln!(&mut text, "# CLIARE {} Report", packet.persona_title)
        .expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");
    writeln!(&mut text, "{}", packet.primary_question).expect("writing to string cannot fail");
    writeln!(&mut text).expect("writing to string cannot fail");

    render_persona_decision(&mut text, packet);
    if packet.persona == Persona::Maintainer {
        render_ci_action_brief(&mut text, packet);
        render_persona_findings(&mut text, packet);
        render_reviewed_decisions(&mut text, packet);
        render_persona_command_guidance(&mut text, packet);
        render_run_recommendations(&mut text, packet);
        render_plain_english_guide(&mut text);
        render_persona_score_summary(&mut text, packet);
        render_artifact_navigation(&mut text, packet);
        render_notes(&mut text, packet);
    } else {
        render_plain_english_guide(&mut text);
        render_ci_action_brief(&mut text, packet);
        render_persona_score_summary(&mut text, packet);
        render_persona_findings(&mut text, packet);
        render_reviewed_decisions(&mut text, packet);
        render_persona_command_guidance(&mut text, packet);
        render_run_recommendations(&mut text, packet);
        render_artifact_navigation(&mut text, packet);
        render_notes(&mut text, packet);
    }

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
            render_maintainer_finding(&mut text, &packet.source_artifacts.artifact_dir, issue);
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
