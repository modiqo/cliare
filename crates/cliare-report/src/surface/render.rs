use std::fmt::Write as _;

use serde::Serialize;

use crate::report_format::shell_arg;
use cliare_cli::cli::SurfaceFormat;
use cliare_core::error::{CliareError, Result};

use super::model::SurfaceMatch;
use super::packets::{SurfaceExplainPacket, SurfaceListPacket, SurfaceQueryPacket};

pub(super) fn render_query_packet(
    packet: &SurfaceQueryPacket,
    format: SurfaceFormat,
) -> Result<String> {
    match format {
        SurfaceFormat::Json => serialize_surface(packet),
        SurfaceFormat::Human => {
            let mut text = String::new();
            writeln!(text, "Surface query: {}", packet.intent)
                .expect("writing to string cannot fail");
            writeln!(text, "Artifact: {}", packet.artifact_dir.display())
                .expect("writing to string cannot fail");
            if let Some(requirement) = packet.require_output {
                writeln!(text, "Required output: {}", requirement.label())
                    .expect("writing to string cannot fail");
            }
            render_matches(
                &mut text,
                &packet.matches,
                packet.no_match_reason.as_deref(),
            );
            Ok(text)
        }
    }
}

pub(super) fn render_explain_packet(
    packet: &SurfaceExplainPacket,
    format: SurfaceFormat,
) -> Result<String> {
    match format {
        SurfaceFormat::Json => serialize_surface(packet),
        SurfaceFormat::Human => {
            let mut text = String::new();
            writeln!(text, "Surface explain: {}", packet.command)
                .expect("writing to string cannot fail");
            writeln!(text, "Artifact: {}", packet.artifact_dir.display())
                .expect("writing to string cannot fail");
            match &packet.surface {
                Some(surface) => render_match(&mut text, 1, surface),
                None => {
                    writeln!(
                        text,
                        "{}",
                        packet
                            .no_match_reason
                            .as_deref()
                            .unwrap_or("No command matched.")
                    )
                    .expect("writing to string cannot fail");
                }
            }
            Ok(text)
        }
    }
}

pub(super) fn render_list_packet(
    packet: &SurfaceListPacket,
    format: SurfaceFormat,
) -> Result<String> {
    match format {
        SurfaceFormat::Json => serialize_surface(packet),
        SurfaceFormat::Human => {
            let mut text = String::new();
            writeln!(text, "Surface list").expect("writing to string cannot fail");
            writeln!(text, "Artifact: {}", packet.artifact_dir.display())
                .expect("writing to string cannot fail");
            if let Some(state) = packet.state {
                writeln!(text, "Readiness: {}", state.label())
                    .expect("writing to string cannot fail");
            }
            if let Some(requirement) = packet.require_output {
                writeln!(text, "Required output: {}", requirement.label())
                    .expect("writing to string cannot fail");
            }
            render_matches(&mut text, &packet.commands, None);
            Ok(text)
        }
    }
}

fn render_matches(text: &mut String, matches: &[SurfaceMatch], no_match_reason: Option<&str>) {
    if matches.is_empty() {
        writeln!(text).expect("writing to string cannot fail");
        writeln!(
            text,
            "{}",
            no_match_reason.unwrap_or("No commands matched.")
        )
        .expect("writing to string cannot fail");
        return;
    }
    for (index, surface) in matches.iter().enumerate() {
        render_match(text, index + 1, surface);
    }
}

fn render_match(text: &mut String, index: usize, surface: &SurfaceMatch) {
    writeln!(text).expect("writing to string cannot fail");
    writeln!(
        text,
        "{}. {} [{}]",
        index, surface.command, surface.readiness
    )
    .expect("writing to string cannot fail");
    if let Some(summary) = &surface.summary {
        writeln!(text, "   {}", summary).expect("writing to string cannot fail");
    }
    writeln!(
        text,
        "   invoke: {}",
        surface
            .argv_template
            .iter()
            .map(|arg| shell_arg(arg))
            .collect::<Vec<_>>()
            .join(" ")
    )
    .expect("writing to string cannot fail");
    writeln!(text, "   why: {}", surface.why).expect("writing to string cannot fail");
    if !surface.requires.is_empty() {
        writeln!(
            text,
            "   requires: {}",
            surface
                .requires
                .iter()
                .map(|requirement| requirement.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
        .expect("writing to string cannot fail");
    }
    if !surface.cautions.is_empty() {
        writeln!(text, "   cautions: {}", surface.cautions.join("; "))
            .expect("writing to string cannot fail");
    }
}

fn serialize_surface<T: Serialize>(packet: &T) -> Result<String> {
    serde_json::to_string_pretty(packet)
        .map(|json| json + "\n")
        .map_err(CliareError::SerializeSurface)
}
