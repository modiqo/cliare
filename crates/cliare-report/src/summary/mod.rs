mod artifacts;
mod model;
mod render;

use std::path::PathBuf;

use cliare_cli::cli::{SummaryArgs, SummaryFormat};
use cliare_context as context;
use cliare_core::error::{CliareError, Result};

use self::artifacts::SummaryArtifacts;
use self::model::MeasurementSummaryPacket;
use self::render::render_markdown;

#[derive(Debug, Clone)]
pub struct SummaryReport {
    pub artifact_dir: PathBuf,
    pub format: SummaryFormat,
    pub packet: MeasurementSummaryPacket,
    stdout: String,
}

impl SummaryReport {
    pub fn terminal_summary(&self) -> &str {
        &self.stdout
    }
}

pub async fn summary(args: SummaryArgs) -> Result<SummaryReport> {
    let artifact_dir =
        context::resolve_measurement_dir(&args.out, args.context.as_deref(), "cliare summary")
            .await?;
    let artifacts = SummaryArtifacts::read(&artifact_dir).await?;
    let packet = MeasurementSummaryPacket::build(
        &artifact_dir,
        artifacts,
        args.max_findings,
        args.max_examples,
    );
    let stdout = match args.format {
        SummaryFormat::Markdown => render_markdown(&packet),
        SummaryFormat::Json => format!(
            "{}\n",
            serde_json::to_string_pretty(&packet).map_err(CliareError::SerializePersonaOutcome)?
        ),
    };

    Ok(SummaryReport {
        artifact_dir,
        format: args.format,
        packet,
        stdout,
    })
}
