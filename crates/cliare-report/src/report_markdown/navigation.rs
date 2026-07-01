use std::fmt::Write as _;

use crate::report_model::*;

pub(super) fn render_artifact_navigation(text: &mut String, packet: &PersonaOutcomePacket) {
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
