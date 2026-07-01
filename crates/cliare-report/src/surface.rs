use std::path::{Path, PathBuf};

use cliare_cli::cli::{
    SurfaceArgs, SurfaceCommand, SurfaceExplainArgs, SurfaceListArgs, SurfaceQueryArgs,
};
use cliare_context as context;
use cliare_core::error::Result;

mod index;
mod matching;
mod model;
mod packets;
mod render;
mod tokens;

#[cfg(test)]
mod tests;

use index::CommandIndexArtifact;
use packets::{SurfaceExplainPacket, SurfaceListPacket, SurfaceQueryPacket};
use render::{render_explain_packet, render_list_packet, render_query_packet};
use tokens::normalize_command_path;

const SURFACE_QUERY_SCHEMA_VERSION: &str = "cliare.surface-query.v1";
const SURFACE_EXPLAIN_SCHEMA_VERSION: &str = "cliare.surface-explain.v1";
const SURFACE_LIST_SCHEMA_VERSION: &str = "cliare.surface-list.v1";

#[derive(Debug, Clone)]
pub struct SurfaceSummary {
    artifact_dir: PathBuf,
    rendered: String,
}

impl SurfaceSummary {
    pub fn terminal_summary(&self) -> &str {
        &self.rendered
    }

    pub fn artifact_dir(&self) -> &Path {
        &self.artifact_dir
    }
}

pub async fn surface(args: SurfaceArgs) -> Result<SurfaceSummary> {
    match args.command {
        SurfaceCommand::Query(args) => query(args).await,
        SurfaceCommand::Explain(args) => explain(args).await,
        SurfaceCommand::List(args) => list(args).await,
    }
}

async fn query(args: SurfaceQueryArgs) -> Result<SurfaceSummary> {
    let artifact_dir = context::resolve_measurement_dir(
        &args.out,
        args.context.as_deref(),
        "cliare surface query",
    )
    .await?;
    let index = CommandIndexArtifact::read(&artifact_dir).await?;
    let packet = SurfaceQueryPacket::build(
        &artifact_dir,
        &args.intent,
        args.require_output,
        args.limit.min(20),
        &index,
    );
    let rendered = render_query_packet(&packet, args.format)?;

    Ok(SurfaceSummary {
        artifact_dir,
        rendered,
    })
}

async fn explain(args: SurfaceExplainArgs) -> Result<SurfaceSummary> {
    let artifact_dir = context::resolve_measurement_dir(
        &args.out,
        args.context.as_deref(),
        "cliare surface explain",
    )
    .await?;
    let index = CommandIndexArtifact::read(&artifact_dir).await?;
    let command_path = normalize_command_path(&args.command);
    let packet =
        SurfaceExplainPacket::build(&artifact_dir, command_path, args.require_output, &index);
    let rendered = render_explain_packet(&packet, args.format)?;

    Ok(SurfaceSummary {
        artifact_dir,
        rendered,
    })
}

async fn list(args: SurfaceListArgs) -> Result<SurfaceSummary> {
    let artifact_dir =
        context::resolve_measurement_dir(&args.out, args.context.as_deref(), "cliare surface list")
            .await?;
    let index = CommandIndexArtifact::read(&artifact_dir).await?;
    let packet = SurfaceListPacket::build(
        &artifact_dir,
        args.state,
        args.require_output,
        args.limit.min(200),
        &index,
    );
    let rendered = render_list_packet(&packet, args.format)?;

    Ok(SurfaceSummary {
        artifact_dir,
        rendered,
    })
}
