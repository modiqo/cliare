use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use cliare_core::artifacts::{COMMAND_INDEX_JSON, ISSUES_JSON, SCORECARD_JSON};
use cliare_core::error::{CliareError, Result};
use cliare_runtime::fingerprint::TargetFingerprint;
use serde::Deserialize;
use tokio::fs;

#[derive(Debug)]
pub(super) struct SummaryArtifacts {
    pub(super) scorecard: ScorecardArtifact,
    pub(super) command_index: CommandIndexArtifact,
    pub(super) issues: Vec<IssueArtifact>,
}

impl SummaryArtifacts {
    pub(super) async fn read(artifact_dir: &Path) -> Result<Self> {
        let scorecard = read_json::<ScorecardArtifact>(&artifact_dir.join(SCORECARD_JSON)).await?;
        let command_index =
            read_json::<CommandIndexArtifact>(&artifact_dir.join(COMMAND_INDEX_JSON)).await?;
        let issues = read_optional_json::<IssueLedgerArtifact>(&artifact_dir.join(ISSUES_JSON))
            .await?
            .map(|ledger| ledger.issues)
            .unwrap_or_default();

        Ok(Self {
            scorecard,
            command_index,
            issues,
        })
    }
}

async fn read_json<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let bytes = fs::read(path)
        .await
        .map_err(|source| CliareError::ReadReportArtifact {
            path: path.to_path_buf(),
            source,
        })?;
    serde_json::from_slice(&bytes).map_err(|source| CliareError::ParseReportArtifact {
        path: path.to_path_buf(),
        source,
    })
}

async fn read_optional_json<T>(path: &Path) -> Result<Option<T>>
where
    T: for<'de> Deserialize<'de>,
{
    match fs::read(path).await {
        Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|source| {
            CliareError::ParseReportArtifact {
                path: path.to_path_buf(),
                source,
            }
        }),
        Err(source) if source.kind() == ErrorKind::NotFound => Ok(None),
        Err(source) => Err(CliareError::ReadReportArtifact {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ScorecardArtifact {
    pub(super) target: TargetFingerprint,
    pub(super) score: ScoreSummaryArtifact,
    pub(super) subscores: BTreeMap<String, SubscoreArtifact>,
    #[serde(default)]
    pub(super) agent_navigation: AgentNavigationArtifact,
    pub(super) coverage: CoverageArtifact,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ScoreSummaryArtifact {
    pub(super) total: f64,
    pub(super) maintainer_readiness: f64,
    pub(super) shape_confidence: f64,
    pub(super) measured_weight: f64,
    pub(super) max_weight: f64,
    pub(super) model: String,
    pub(super) status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct SubscoreArtifact {
    pub(super) score: Option<f64>,
    pub(super) status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct AgentNavigationArtifact {
    pub(super) status: String,
    #[serde(default)]
    pub(super) dimensions: BTreeMap<String, AgentNavigationMetricArtifact>,
    #[serde(default)]
    pub(super) limitations: Vec<String>,
}

impl Default for AgentNavigationArtifact {
    fn default() -> Self {
        Self {
            status: "not_available".to_owned(),
            dimensions: BTreeMap::new(),
            limitations: vec!["Scorecard did not include agent navigation metrics.".to_owned()],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct AgentNavigationMetricArtifact {
    pub(super) score: Option<f64>,
    pub(super) numerator: usize,
    pub(super) denominator: usize,
    pub(super) status: String,
    pub(super) rationale: String,
    #[serde(default)]
    pub(super) limitations: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct CoverageArtifact {
    pub(super) commands_discovered: usize,
    pub(super) commands_runtime_confirmed: usize,
    pub(super) commands_precondition_blocked: usize,
    pub(super) help_text_probes: usize,
    pub(super) help_text_probes_with_shape: usize,
    pub(super) help_text_probes_without_shape: usize,
    pub(super) help_text_probes_not_recognized: usize,
    pub(super) parser_extraction_rate: f64,
    pub(super) flags_discovered: usize,
    pub(super) output_contracts_discovered: usize,
    pub(super) machine_readable_output_contracts: usize,
    pub(super) output_mode_probes_completed: usize,
    pub(super) output_mode_parse_successes: usize,
    pub(super) output_mode_precondition_blocked: usize,
    pub(super) precondition_blocked_probes: usize,
    pub(super) auth_required_probes: usize,
    pub(super) local_context_required_probes: usize,
    pub(super) fixture_required_probes: usize,
    pub(super) actionable_precondition_probes: usize,
    pub(super) precondition_recovery_rate: f64,
    pub(super) side_effect_files_total: usize,
    pub(super) side_effect_probe_count: usize,
    pub(super) credential_like_side_effects: usize,
    pub(super) observed_max_depth: usize,
    pub(super) max_depth: usize,
    pub(super) max_probes: usize,
    pub(super) probes_completed: usize,
    pub(super) frontier_remaining: usize,
    pub(super) budget_exhausted: bool,
    pub(super) traversal_stop_reason: String,
    pub(super) traversal_complete: bool,
    pub(super) sandbox_profile: String,
    pub(super) sandbox_env_policy: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct CommandIndexArtifact {
    pub(super) summary: CommandIndexSummaryArtifact,
    pub(super) commands: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct CommandIndexSummaryArtifact {
    pub(super) ready: usize,
    pub(super) conditional: usize,
    pub(super) needs_fixture: usize,
    pub(super) blocked: usize,
    pub(super) candidate: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct IssueLedgerArtifact {
    pub(super) issues: Vec<IssueArtifact>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct IssueArtifact {
    pub(super) id: String,
    pub(super) status: String,
    pub(super) severity: String,
    pub(super) category: String,
    pub(super) title: String,
    pub(super) impact: String,
    pub(super) recommendation: String,
    #[serde(default)]
    pub(super) affected_commands: Vec<IssueCommandArtifact>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct IssueCommandArtifact {
    pub(super) path: Vec<String>,
    pub(super) state: String,
    #[serde(default)]
    pub(super) required_positionals: Vec<String>,
    #[serde(default)]
    pub(super) preconditions: Vec<String>,
    #[serde(default)]
    pub(super) output_contracts: Vec<OutputContractArtifact>,
    pub(super) reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct OutputContractArtifact {
    pub(super) mode: String,
    pub(super) flag_name: Option<String>,
    pub(super) status: String,
    pub(super) diagnostic: Option<String>,
    pub(super) skip_reason: Option<String>,
    pub(super) suggested_validation: Option<String>,
}

pub(super) fn artifact_paths(artifact_dir: &Path) -> SummaryArtifactPaths {
    SummaryArtifactPaths {
        artifact_dir: artifact_dir.to_path_buf(),
        scorecard: artifact_dir.join(SCORECARD_JSON),
        command_index: artifact_dir.join(COMMAND_INDEX_JSON),
        issues: artifact_dir.join(ISSUES_JSON),
    }
}

#[derive(Debug, Clone)]
pub(super) struct SummaryArtifactPaths {
    pub(super) artifact_dir: PathBuf,
    pub(super) scorecard: PathBuf,
    pub(super) command_index: PathBuf,
    pub(super) issues: PathBuf,
}
