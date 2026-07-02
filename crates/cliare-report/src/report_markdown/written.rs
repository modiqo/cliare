use std::fmt::Write as _;
use std::path::PathBuf;

use crate::artifact_guide::ArtifactGuideSummary;
use crate::report_model::*;

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
        writeln!(
            &mut text,
            "condition dictionary: {}",
            artifacts.condition_dictionary_path.display()
        )
        .expect("writing to string cannot fail");
    }
    text
}
