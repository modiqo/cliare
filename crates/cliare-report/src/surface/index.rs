use std::path::Path;

use serde::Deserialize;
use tokio::fs;

use cliare_core::artifacts::COMMAND_INDEX_JSON;
use cliare_core::error::{CliareError, Result};

#[derive(Debug, Deserialize)]
pub(super) struct CommandIndexArtifact {
    pub(super) commands: Vec<CommandIndexCommand>,
}

impl CommandIndexArtifact {
    pub(super) async fn read(artifact_dir: &Path) -> Result<Self> {
        let path = artifact_dir.join(COMMAND_INDEX_JSON);
        let bytes = fs::read(&path)
            .await
            .map_err(|source| CliareError::ReadCommandIndex {
                path: path.clone(),
                source,
            })?;
        serde_json::from_slice(&bytes)
            .map_err(|source| CliareError::ParseCommandIndex { path, source })
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct CommandIndexCommand {
    pub(super) id: String,
    pub(super) command: String,
    pub(super) path: Vec<String>,
    pub(super) argv: Vec<String>,
    pub(super) summary: Option<String>,
    pub(super) runtime_state: String,
    pub(super) agent_suitability: String,
    #[serde(default)]
    pub(super) suitability_reasons: Vec<String>,
    #[serde(default)]
    pub(super) confidence: f64,
    pub(super) parameters: CommandIndexParameters,
    #[serde(default)]
    pub(super) preconditions: Vec<String>,
    #[serde(default)]
    pub(super) output_contracts: Vec<CommandIndexOutputContract>,
    #[serde(default)]
    pub(super) gaps: Vec<CommandIndexGap>,
    #[serde(default)]
    pub(super) evidence: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct CommandIndexParameters {
    #[serde(default)]
    pub(super) positionals: Vec<CommandIndexPositional>,
    #[serde(default)]
    pub(super) flags: Vec<CommandIndexFlag>,
}

#[derive(Debug, Deserialize)]
pub(super) struct CommandIndexPositional {
    pub(super) name: String,
    #[serde(default)]
    pub(super) required: bool,
    #[serde(default)]
    pub(super) variadic: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct CommandIndexFlag {
    pub(super) name: String,
    pub(super) short: Option<String>,
    pub(super) summary: Option<String>,
    pub(super) value_name: Option<String>,
    #[serde(default)]
    pub(super) required: bool,
    #[serde(default)]
    pub(super) repeatable: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct CommandIndexOutputContract {
    pub(super) mode: String,
    #[serde(default)]
    pub(super) argv_fragment: Vec<String>,
    pub(super) status: String,
    #[serde(default)]
    pub(super) preconditions: Vec<String>,
    pub(super) observed_kind: Option<String>,
    pub(super) diagnostic: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct CommandIndexGap {
    pub(super) kind: String,
    pub(super) reason: String,
}
