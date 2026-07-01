use std::collections::BTreeSet;
use std::path::Path;

use cliare_core::error::Result;

use super::ARTIFACT_MAP_SCHEMA_VERSION;
use super::inspect::{dynamic_artifact_paths, dynamic_file_spec, inspect_entry, read_top_level};
use super::model::{ArtifactMap, ArtifactSummaries};
use super::navigation::{health, navigation_plan};
use super::specs::{detect_artifact_kind, known_file_specs};
use super::summaries::{
    benchmark_summary, command_index_summary, contexts_summary, issues_summary, job_summary,
    scorecard_summary,
};
use super::util::timestamp;

pub(super) async fn build_artifact_map(folder: &Path) -> Result<ArtifactMap> {
    let top_level = read_top_level(folder).await?;
    let artifact_kind = detect_artifact_kind(&top_level);
    let mut known = BTreeSet::new();
    let mut files = Vec::new();

    for spec in known_file_specs(artifact_kind) {
        known.insert(spec.path.to_owned());
        files.push(inspect_entry(folder, spec).await);
    }

    for path in dynamic_artifact_paths(folder, &top_level).await? {
        if known.insert(path.clone()) {
            files.push(inspect_entry(folder, dynamic_file_spec(path)).await);
        }
    }

    files.sort_by(|left, right| {
        left.navigation_rank
            .cmp(&right.navigation_rank)
            .then_with(|| left.path.cmp(&right.path))
    });

    let missing_required = files
        .iter()
        .filter(|file| file.required && !file.exists)
        .map(|file| file.path.clone())
        .collect::<Vec<_>>();

    let summaries = ArtifactSummaries {
        scorecard: scorecard_summary(folder).await,
        command_index: command_index_summary(folder).await,
        issues: issues_summary(folder).await,
        benchmark: benchmark_summary(folder).await,
        contexts: contexts_summary(folder).await,
        job: job_summary(folder).await,
    };

    Ok(ArtifactMap {
        schema_version: ARTIFACT_MAP_SCHEMA_VERSION.to_owned(),
        cliare_version: env!("CARGO_PKG_VERSION").to_owned(),
        generated_at: timestamp()?,
        folder: folder.display().to_string(),
        artifact_kind,
        health: health(&missing_required, &summaries),
        navigation: navigation_plan(artifact_kind),
        summaries,
        missing_required,
        files,
        written_files: Vec::new(),
    })
}
