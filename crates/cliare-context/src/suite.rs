use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs;

use cliare_core::artifacts::{CONTEXT_COMPARE_MD, CONTEXT_SUITE_JSON, SCORECARD_JSON};
use cliare_core::error::{CliareError, Result};

use super::CONTEXT_SUITE_SCHEMA_VERSION;
use super::artifact::discover_context_dirs;
use super::cli::{ContextArgs, ContextCommand, ContextCompareArgs, ContextCompareFormat};
use super::render::render_context_suite;
use super::runtime::{RuntimeContext, RuntimeContextInput, RuntimeContextProfile};

pub async fn refresh_context_suite(root: &Path) -> Result<Option<ContextSummary>> {
    let dirs = discover_context_dirs(root).await?;
    if dirs.is_empty() {
        return Ok(None);
    }
    let suite = build_context_suite(&dirs).await?;
    let json = serde_json::to_string_pretty(&suite).map_err(CliareError::SerializeContextSuite)?;
    let markdown = render_context_suite(&suite);
    write_context_suite(root, json.as_bytes(), markdown.as_bytes()).await?;
    Ok(Some(ContextSummary {
        suite,
        markdown,
        json,
        wrote: true,
        out: root.to_path_buf(),
        format: ContextCompareFormat::Markdown,
    }))
}

pub async fn context(args: ContextArgs) -> Result<ContextSummary> {
    match args.command {
        ContextCommand::Compare(args) => compare(args).await,
    }
}

#[derive(Debug, Clone)]
pub struct ContextSummary {
    pub suite: ContextSuite,
    pub markdown: String,
    pub json: String,
    pub wrote: bool,
    pub out: PathBuf,
    pub format: ContextCompareFormat,
}

impl ContextSummary {
    pub fn terminal_summary(&self) -> String {
        match self.format {
            ContextCompareFormat::Json => {
                let mut output = self.json.clone();
                output.push('\n');
                output
            }
            ContextCompareFormat::Markdown => self.markdown.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContextSuite {
    pub(super) schema_version: String,
    pub(super) target: Option<ContextTarget>,
    pub(super) contexts: Vec<ContextSuiteEntry>,
    pub(super) precondition_chain: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct ContextTarget {
    pub(super) requested: String,
    pub(super) resolved: String,
    pub(super) binary_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct ContextSuiteEntry {
    pub(super) name: String,
    pub(super) profile: RuntimeContextProfile,
    pub(super) artifact_dir: PathBuf,
    pub(super) score: f64,
    pub(super) status: String,
    pub(super) commands_discovered: u64,
    pub(super) commands_runtime_confirmed: u64,
    pub(super) commands_precondition_blocked: u64,
    pub(super) preconditions_observed: Vec<String>,
    pub(super) traversal_complete: bool,
}

#[derive(Debug, Deserialize)]
struct ScorecardArtifact {
    target: ScoreTarget,
    score: ScoreSection,
    coverage: CoverageSection,
    #[serde(default)]
    runtime_context: Option<RuntimeContext>,
}

#[derive(Debug, Deserialize)]
struct ScoreTarget {
    requested: String,
    resolved: String,
    binary_sha256: String,
}

#[derive(Debug, Deserialize)]
struct ScoreSection {
    total: f64,
    status: String,
}

#[derive(Debug, Deserialize)]
struct CoverageSection {
    commands_discovered: u64,
    commands_runtime_confirmed: u64,
    commands_precondition_blocked: u64,
    #[serde(default)]
    auth_required_probes: u64,
    #[serde(default)]
    local_context_required_probes: u64,
    #[serde(default)]
    fixture_required_probes: u64,
    #[serde(default)]
    output_mode_precondition_blocked: u64,
    traversal_complete: bool,
}

pub(super) async fn compare(args: ContextCompareArgs) -> Result<ContextSummary> {
    let suite = build_context_suite(&args.context_dirs).await?;
    let json = serde_json::to_string_pretty(&suite).map_err(CliareError::SerializeContextSuite)?;
    let markdown = render_context_suite(&suite);

    if args.write {
        write_context_suite(&args.out, json.as_bytes(), markdown.as_bytes()).await?;
    }

    Ok(ContextSummary {
        suite,
        markdown,
        json,
        wrote: args.write,
        out: args.out,
        format: args.format,
    })
}

pub(super) async fn build_context_suite(context_dirs: &[PathBuf]) -> Result<ContextSuite> {
    let mut entries = Vec::new();
    let mut target = None;

    for dir in context_dirs {
        let scorecard = read_scorecard(dir).await?;
        if target.is_none() {
            target = Some(ContextTarget {
                requested: scorecard.target.requested.clone(),
                resolved: scorecard.target.resolved.clone(),
                binary_sha256: scorecard.target.binary_sha256.clone(),
            });
        }

        let runtime_context = scorecard.runtime_context.unwrap_or_else(|| {
            RuntimeContext::from_input(RuntimeContextInput {
                profile: Some(RuntimeContextProfile::Single),
                name: dir
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_owned),
                auth_state: None,
                local_context_state: None,
                fixture_state: None,
                network_state: None,
                runtime_dependency_state: None,
                workdir: None,
            })
        });

        entries.push(ContextSuiteEntry {
            name: runtime_context.name,
            profile: runtime_context.profile,
            artifact_dir: dir.clone(),
            score: scorecard.score.total,
            status: scorecard.score.status,
            commands_discovered: scorecard.coverage.commands_discovered,
            commands_runtime_confirmed: scorecard.coverage.commands_runtime_confirmed,
            commands_precondition_blocked: scorecard.coverage.commands_precondition_blocked,
            preconditions_observed: observed_preconditions(&scorecard.coverage),
            traversal_complete: scorecard.coverage.traversal_complete,
        });
    }

    let precondition_chain = precondition_chain(&entries);
    Ok(ContextSuite {
        schema_version: CONTEXT_SUITE_SCHEMA_VERSION.to_owned(),
        target,
        contexts: entries,
        precondition_chain,
    })
}

pub(super) async fn write_context_suite(root: &Path, json: &[u8], markdown: &[u8]) -> Result<()> {
    fs::create_dir_all(root)
        .await
        .map_err(|source| CliareError::CreateContextSuiteDir {
            path: root.to_path_buf(),
            source,
        })?;
    let json_path = root.join(CONTEXT_SUITE_JSON);
    fs::write(&json_path, json)
        .await
        .map_err(|source| CliareError::WriteContextSuite {
            path: json_path,
            source,
        })?;
    let markdown_path = root.join(CONTEXT_COMPARE_MD);
    fs::write(&markdown_path, markdown)
        .await
        .map_err(|source| CliareError::WriteContextSuite {
            path: markdown_path,
            source,
        })
}

async fn read_scorecard(dir: &Path) -> Result<ScorecardArtifact> {
    let path = dir.join(SCORECARD_JSON);
    let bytes = fs::read(&path)
        .await
        .map_err(|source| CliareError::ReadContextScorecard {
            path: path.clone(),
            source,
        })?;
    serde_json::from_slice(&bytes)
        .map_err(|source| CliareError::ParseContextScorecard { path, source })
}

fn observed_preconditions(coverage: &CoverageSection) -> Vec<String> {
    let mut preconditions = Vec::new();
    if coverage.auth_required_probes > 0 {
        preconditions.push("auth_required".to_owned());
    }
    if coverage.local_context_required_probes > 0 {
        preconditions.push("local_context_required".to_owned());
    }
    if coverage.fixture_required_probes > 0 {
        preconditions.push("fixture_required".to_owned());
    }
    if coverage.output_mode_precondition_blocked > 0
        && preconditions.is_empty()
        && coverage.commands_precondition_blocked > 0
    {
        preconditions.push("runtime_precondition".to_owned());
    }
    preconditions
}

pub(super) fn precondition_chain(entries: &[ContextSuiteEntry]) -> Vec<String> {
    let mut chain = Vec::new();
    for precondition in [
        "auth_required",
        "local_context_required",
        "fixture_required",
        "runtime_precondition",
    ] {
        if entries.iter().any(|entry| {
            entry
                .preconditions_observed
                .iter()
                .any(|observed| observed == precondition)
        }) {
            chain.push(precondition.to_owned());
        }
    }
    chain
}
