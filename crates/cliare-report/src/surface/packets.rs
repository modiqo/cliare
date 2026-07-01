use std::path::{Path, PathBuf};

use serde::Serialize;

use cliare_cli::cli::{SurfaceOutputRequirement, SurfaceReadiness};
use cliare_core::artifacts::COMMAND_INDEX_JSON;

use super::index::CommandIndexArtifact;
use super::matching::{match_reason, output_requirement_matches, readiness_rank, score_command};
use super::model::SurfaceMatch;
use super::tokens::TokenSet;
use super::{
    SURFACE_EXPLAIN_SCHEMA_VERSION, SURFACE_LIST_SCHEMA_VERSION, SURFACE_QUERY_SCHEMA_VERSION,
};

#[derive(Debug, Serialize)]
pub(super) struct SurfaceQueryPacket {
    pub(super) schema_version: &'static str,
    pub(super) artifact_dir: PathBuf,
    pub(super) command_index: PathBuf,
    pub(super) intent: String,
    pub(super) require_output: Option<SurfaceOutputRequirement>,
    pub(super) limit: usize,
    pub(super) matches: Vec<SurfaceMatch>,
    pub(super) no_match_reason: Option<String>,
}

impl SurfaceQueryPacket {
    pub(super) fn build(
        artifact_dir: &Path,
        intent: &str,
        require_output: Option<SurfaceOutputRequirement>,
        limit: usize,
        index: &CommandIndexArtifact,
    ) -> Self {
        let intent_tokens = TokenSet::from_text(intent);
        let mut scored = index
            .commands
            .iter()
            .filter(|command| output_requirement_matches(command, require_output))
            .filter_map(|command| {
                let score = score_command(command, &intent_tokens, require_output);
                (score > 0).then_some((score, command))
            })
            .collect::<Vec<_>>();

        scored.sort_by(|(left_score, left), (right_score, right)| {
            right_score
                .cmp(left_score)
                .then_with(|| {
                    readiness_rank(&right.agent_suitability)
                        .cmp(&readiness_rank(&left.agent_suitability))
                })
                .then_with(|| right.confidence.total_cmp(&left.confidence))
                .then_with(|| left.path.len().cmp(&right.path.len()))
                .then_with(|| left.path.cmp(&right.path))
        });

        let matches = scored
            .into_iter()
            .take(limit)
            .map(|(score, command)| {
                SurfaceMatch::from_command(
                    command,
                    Some(score),
                    require_output,
                    Some(&intent_tokens),
                    match_reason(command, &intent_tokens, require_output),
                )
            })
            .collect::<Vec<_>>();
        let no_match_reason = matches.is_empty().then(|| {
            if intent.trim().is_empty() {
                "Intent was empty; provide task words such as `check job status`.".to_owned()
            } else if require_output.is_some() {
                "No measured command matched the intent and requested output capability.".to_owned()
            } else {
                "No measured command matched the intent tokens.".to_owned()
            }
        });

        Self {
            schema_version: SURFACE_QUERY_SCHEMA_VERSION,
            artifact_dir: artifact_dir.to_path_buf(),
            command_index: artifact_dir.join(COMMAND_INDEX_JSON),
            intent: intent.to_owned(),
            require_output,
            limit,
            matches,
            no_match_reason,
        }
    }
}

#[derive(Debug, Serialize)]
pub(super) struct SurfaceExplainPacket {
    pub(super) schema_version: &'static str,
    pub(super) artifact_dir: PathBuf,
    pub(super) command_index: PathBuf,
    pub(super) command: String,
    pub(super) require_output: Option<SurfaceOutputRequirement>,
    pub(super) surface: Option<SurfaceMatch>,
    pub(super) no_match_reason: Option<String>,
}

impl SurfaceExplainPacket {
    pub(super) fn build(
        artifact_dir: &Path,
        command_path: Vec<String>,
        require_output: Option<SurfaceOutputRequirement>,
        index: &CommandIndexArtifact,
    ) -> Self {
        let command = command_path.join(" ");
        let surface = index
            .commands
            .iter()
            .find(|candidate| candidate.path == command_path || candidate.command == command)
            .map(|candidate| {
                SurfaceMatch::from_command(
                    candidate,
                    None,
                    require_output,
                    None,
                    "Exact measured command path.".to_owned(),
                )
            });
        let no_match_reason = surface
            .is_none()
            .then(|| "No command with this path exists in command-index.json.".to_owned());

        Self {
            schema_version: SURFACE_EXPLAIN_SCHEMA_VERSION,
            artifact_dir: artifact_dir.to_path_buf(),
            command_index: artifact_dir.join(COMMAND_INDEX_JSON),
            command,
            require_output,
            surface,
            no_match_reason,
        }
    }
}

#[derive(Debug, Serialize)]
pub(super) struct SurfaceListPacket {
    pub(super) schema_version: &'static str,
    pub(super) artifact_dir: PathBuf,
    pub(super) command_index: PathBuf,
    pub(super) state: Option<SurfaceReadiness>,
    pub(super) require_output: Option<SurfaceOutputRequirement>,
    pub(super) limit: usize,
    pub(super) commands: Vec<SurfaceMatch>,
}

impl SurfaceListPacket {
    pub(super) fn build(
        artifact_dir: &Path,
        state: Option<SurfaceReadiness>,
        require_output: Option<SurfaceOutputRequirement>,
        limit: usize,
        index: &CommandIndexArtifact,
    ) -> Self {
        let mut commands = index
            .commands
            .iter()
            .filter(|command| state.is_none_or(|state| command.agent_suitability == state.label()))
            .filter(|command| output_requirement_matches(command, require_output))
            .collect::<Vec<_>>();
        commands.sort_by(|left, right| {
            readiness_rank(&right.agent_suitability)
                .cmp(&readiness_rank(&left.agent_suitability))
                .then_with(|| right.confidence.total_cmp(&left.confidence))
                .then_with(|| left.path.len().cmp(&right.path.len()))
                .then_with(|| left.path.cmp(&right.path))
        });

        Self {
            schema_version: SURFACE_LIST_SCHEMA_VERSION,
            artifact_dir: artifact_dir.to_path_buf(),
            command_index: artifact_dir.join(COMMAND_INDEX_JSON),
            state,
            require_output,
            limit,
            commands: commands
                .into_iter()
                .take(limit)
                .map(|command| {
                    SurfaceMatch::from_command(
                        command,
                        None,
                        require_output,
                        None,
                        "Listed from measured command index.".to_owned(),
                    )
                })
                .collect(),
        }
    }
}
