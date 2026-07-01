use std::path::PathBuf;

use cliare_cli::cli::{DescribeArgs, DescribeFormat};
use cliare_context as context;
use cliare_core::error::{CliareError, Result};

mod inspect;
mod map;
mod model;
mod navigation;
mod render;
mod specs;
mod summaries;
mod util;

#[cfg(test)]
mod tests;

pub use model::{
    ArtifactFile, ArtifactHealth, ArtifactKind, ArtifactMap, ArtifactSummaries, BenchmarkSummary,
    CommandIndexSummary, ContextSuiteSummary, ContextSummaryItem, FileKind, IssuesSummary,
    JobSummary, NavigationStep, ScorecardSummary,
};

use map::build_artifact_map;
use render::render_markdown;
use util::{ensure_directory, relative_to, write_json, write_markdown};

const ARTIFACT_MAP_SCHEMA_VERSION: &str = "cliare.artifact-map.v1";

#[derive(Debug)]
pub struct DescribeSummary {
    pub folder: PathBuf,
    pub artifact_kind: ArtifactKind,
    pub files_total: usize,
    pub missing_required: usize,
    pub artifact_map_json_path: Option<PathBuf>,
    pub artifact_map_markdown_path: Option<PathBuf>,
    rendered: String,
}

impl DescribeSummary {
    pub fn terminal_summary(&self) -> &str {
        &self.rendered
    }
}

pub async fn describe(args: DescribeArgs) -> Result<DescribeSummary> {
    let folder = if args.context.is_some() {
        context::resolve_measurement_dir(&args.folder, args.context.as_deref(), "cliare describe")
            .await?
    } else {
        args.folder.clone()
    };
    ensure_directory(&folder).await?;
    let mut map = build_artifact_map(&folder).await?;
    let mut artifact_map_json_path = None;
    let mut artifact_map_markdown_path = None;

    if args.write {
        let json_path = folder.join("artifact-map.json");
        let markdown_path = folder.join("artifact-map.md");
        map.written_files = vec![
            relative_to(&folder, &json_path),
            relative_to(&folder, &markdown_path),
        ];
        write_json(&json_path, &map).await?;
        write_markdown(&markdown_path, &render_markdown(&map)).await?;
        map = build_artifact_map(&folder).await?;
        map.written_files = vec![
            relative_to(&folder, &json_path),
            relative_to(&folder, &markdown_path),
        ];
        write_json(&json_path, &map).await?;
        write_markdown(&markdown_path, &render_markdown(&map)).await?;
        artifact_map_json_path = Some(json_path);
        artifact_map_markdown_path = Some(markdown_path);
    }

    let rendered = match args.format {
        DescribeFormat::Markdown => render_markdown(&map),
        DescribeFormat::Json => {
            serde_json::to_string_pretty(&map).map_err(CliareError::SerializeArtifactMap)?
                + "
"
        }
    };

    Ok(DescribeSummary {
        folder,
        artifact_kind: map.artifact_kind,
        files_total: map.files.len(),
        missing_required: map.missing_required.len(),
        artifact_map_json_path,
        artifact_map_markdown_path,
        rendered,
    })
}
