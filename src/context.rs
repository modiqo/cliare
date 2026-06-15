use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand, ValueEnum, ValueHint};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::artifacts::{
    CONTEXT_COMPARE_MD, CONTEXT_SUITE_JSON, RUNTIME_CONTEXT_JSON, SCORECARD_JSON,
};
use crate::error::{CliareError, Result};

pub const RUNTIME_CONTEXT_SCHEMA_VERSION: &str = "cliare.runtime-context.v1";
const CONTEXT_SUITE_SCHEMA_VERSION: &str = "cliare.context-suite.v1";

#[derive(Debug, Args)]
pub struct ContextArgs {
    #[command(subcommand)]
    pub command: ContextCommand,
}

#[derive(Debug, Subcommand)]
pub enum ContextCommand {
    /// Compare multiple context measurement artifact directories.
    Compare(ContextCompareArgs),
}

#[derive(Debug, Args)]
pub struct ContextCompareArgs {
    /// Context measurement directories to compare.
    #[arg(value_name = "CONTEXT_DIR", value_hint = ValueHint::DirPath, required = true)]
    pub context_dirs: Vec<PathBuf>,

    /// Output directory for context-suite.json and context-compare.md.
    #[arg(
        long,
        value_name = "DIR",
        default_value = ".cliare-context",
        value_hint = ValueHint::DirPath
    )]
    pub out: PathBuf,

    /// Representation to print to stdout.
    #[arg(long, value_enum, default_value_t = ContextCompareFormat::Markdown)]
    pub format: ContextCompareFormat,

    /// Write context-suite.json and context-compare.md to --out.
    #[arg(long)]
    pub write: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ContextCompareFormat {
    Markdown,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeContextProfile {
    Single,
    Clean,
    Authenticated,
    LocalContext,
    Fixture,
    Custom,
}

impl RuntimeContextProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Clean => "clean",
            Self::Authenticated => "authenticated",
            Self::LocalContext => "local_context",
            Self::Fixture => "fixture",
            Self::Custom => "custom",
        }
    }

    pub fn folder_name(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Clean => "clean",
            Self::Authenticated => "authenticated",
            Self::LocalContext => "local-context",
            Self::Fixture => "fixture",
            Self::Custom => "custom",
        }
    }

    pub fn cli_value(self) -> &'static str {
        self.folder_name()
    }

    pub fn default_auth_state(self) -> RuntimeContextState {
        match self {
            Self::Clean => RuntimeContextState::Absent,
            Self::Authenticated => RuntimeContextState::Present,
            Self::Single | Self::LocalContext | Self::Fixture | Self::Custom => {
                RuntimeContextState::Unknown
            }
        }
    }

    pub fn default_local_context_state(self) -> RuntimeContextState {
        match self {
            Self::Clean | Self::Authenticated => RuntimeContextState::Absent,
            Self::LocalContext => RuntimeContextState::Present,
            Self::Single | Self::Fixture | Self::Custom => RuntimeContextState::Unknown,
        }
    }

    pub fn default_fixture_state(self) -> RuntimeContextState {
        match self {
            Self::Clean | Self::Authenticated | Self::LocalContext => RuntimeContextState::Absent,
            Self::Fixture => RuntimeContextState::Present,
            Self::Single | Self::Custom => RuntimeContextState::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeContextState {
    Absent,
    Present,
    Unknown,
    Declared,
}

impl RuntimeContextState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Present => "present",
            Self::Unknown => "unknown",
            Self::Declared => "declared",
        }
    }

    pub fn cli_value(self) -> &'static str {
        self.label()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeContextCwdPolicy {
    Isolated,
    Provided,
}

impl RuntimeContextCwdPolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::Isolated => "isolated",
            Self::Provided => "provided",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RuntimeContext {
    pub schema_version: String,
    pub profile: RuntimeContextProfile,
    pub name: String,
    pub auth_state: RuntimeContextState,
    pub local_context_state: RuntimeContextState,
    pub fixture_state: RuntimeContextState,
    pub network_state: RuntimeContextState,
    pub runtime_dependency_state: RuntimeContextState,
    pub cwd_policy: RuntimeContextCwdPolicy,
    pub workdir: Option<PathBuf>,
    pub declared_by: RuntimeContextDeclaration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeContextDeclaration {
    Default,
    Cli,
}

#[derive(Debug, Clone)]
pub struct RuntimeContextInput {
    pub profile: Option<RuntimeContextProfile>,
    pub name: Option<String>,
    pub auth_state: Option<RuntimeContextState>,
    pub local_context_state: Option<RuntimeContextState>,
    pub fixture_state: Option<RuntimeContextState>,
    pub network_state: Option<RuntimeContextState>,
    pub runtime_dependency_state: Option<RuntimeContextState>,
    pub workdir: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PersistedContext {
    pub name: String,
    pub profile: Option<RuntimeContextProfile>,
    pub artifact_dir: PathBuf,
}

impl RuntimeContext {
    pub fn from_input(input: RuntimeContextInput) -> Self {
        let profile = input.profile.unwrap_or(RuntimeContextProfile::Single);
        let declared_by = if input.profile.is_some()
            || input.name.is_some()
            || input.auth_state.is_some()
            || input.local_context_state.is_some()
            || input.fixture_state.is_some()
            || input.network_state.is_some()
            || input.runtime_dependency_state.is_some()
            || input.workdir.is_some()
        {
            RuntimeContextDeclaration::Cli
        } else {
            RuntimeContextDeclaration::Default
        };
        let cwd_policy = if input.workdir.is_some() {
            RuntimeContextCwdPolicy::Provided
        } else {
            RuntimeContextCwdPolicy::Isolated
        };
        let local_context_state = input.local_context_state.unwrap_or_else(|| {
            if input.workdir.is_some() {
                RuntimeContextState::Present
            } else {
                profile.default_local_context_state()
            }
        });

        Self {
            schema_version: RUNTIME_CONTEXT_SCHEMA_VERSION.to_owned(),
            profile,
            name: input
                .name
                .unwrap_or_else(|| profile.folder_name().to_owned()),
            auth_state: input
                .auth_state
                .unwrap_or_else(|| profile.default_auth_state()),
            local_context_state,
            fixture_state: input
                .fixture_state
                .unwrap_or_else(|| profile.default_fixture_state()),
            network_state: input.network_state.unwrap_or(RuntimeContextState::Unknown),
            runtime_dependency_state: input
                .runtime_dependency_state
                .unwrap_or(RuntimeContextState::Unknown),
            cwd_policy,
            workdir: input.workdir,
            declared_by,
        }
    }

    pub fn is_context_suite_measurement(&self) -> bool {
        self.profile != RuntimeContextProfile::Single
    }

    pub fn folder_name(&self) -> String {
        sanitize_context_name(&self.name)
    }
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self::from_input(RuntimeContextInput {
            profile: None,
            name: None,
            auth_state: None,
            local_context_state: None,
            fixture_state: None,
            network_state: None,
            runtime_dependency_state: None,
            workdir: None,
        })
    }
}

pub fn measurement_dir(root: &Path, context: &RuntimeContext) -> PathBuf {
    if context.is_context_suite_measurement() {
        contexts_dir(root).join(context.folder_name())
    } else {
        root.to_path_buf()
    }
}

pub fn context_artifact_dir(root: &Path, context: &str) -> PathBuf {
    contexts_dir(root).join(sanitize_context_name(context))
}

pub fn contexts_dir(root: &Path) -> PathBuf {
    root.join("contexts")
}

pub async fn persisted_contexts(root: &Path) -> Result<Vec<PersistedContext>> {
    let dirs = discover_context_dirs(root).await?;
    let mut contexts = Vec::new();
    for dir in dirs {
        let runtime_context = read_runtime_context(&dir).await?;
        let fallback_name = dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("context")
            .to_owned();
        contexts.push(PersistedContext {
            name: runtime_context
                .as_ref()
                .map(|context| context.name.clone())
                .unwrap_or(fallback_name),
            profile: runtime_context.map(|context| context.profile),
            artifact_dir: dir,
        });
    }
    contexts.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(contexts)
}

pub async fn resolve_measurement_dir(
    root: &Path,
    context: Option<&str>,
    command: &str,
) -> Result<PathBuf> {
    if let Some(context) = context {
        let artifact_dir = context_artifact_dir(root, context);
        if artifact_dir.join(SCORECARD_JSON).is_file() {
            return Ok(artifact_dir);
        }
        let contexts = persisted_contexts(root).await?;
        return Err(CliareError::ContextSelectionNotFound {
            message: context_not_found_message(root, command, context, &contexts),
        });
    }

    if root.join(SCORECARD_JSON).is_file() {
        return Ok(root.to_path_buf());
    }

    let contexts = persisted_contexts(root).await?;
    match contexts.as_slice() {
        [] => Err(invalid_measurement_artifact(root, command).await?),
        [context] => Ok(context.artifact_dir.clone()),
        _ => Err(CliareError::ContextSelectionRequired {
            message: context_required_message(root, command, &contexts),
        }),
    }
}

pub async fn is_context_suite_root(root: &Path) -> Result<bool> {
    if root.join(CONTEXT_SUITE_JSON).is_file() {
        return Ok(true);
    }
    Ok(!persisted_contexts(root).await?.is_empty())
}

pub async fn write_runtime_context(out_dir: &Path, context: &RuntimeContext) -> Result<PathBuf> {
    fs::create_dir_all(out_dir)
        .await
        .map_err(|source| CliareError::CreateArtifactDir {
            path: out_dir.to_path_buf(),
            source,
        })?;
    let path = out_dir.join(RUNTIME_CONTEXT_JSON);
    let bytes = serde_json::to_vec_pretty(context).map_err(CliareError::SerializeRuntimeContext)?;
    fs::write(&path, bytes)
        .await
        .map_err(|source| CliareError::WriteRuntimeContext {
            path: path.clone(),
            source,
        })?;
    Ok(path)
}

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
    schema_version: String,
    target: Option<ContextTarget>,
    contexts: Vec<ContextSuiteEntry>,
    precondition_chain: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ContextTarget {
    requested: String,
    resolved: String,
    binary_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ContextSuiteEntry {
    name: String,
    profile: RuntimeContextProfile,
    artifact_dir: PathBuf,
    score: f64,
    status: String,
    commands_discovered: u64,
    commands_runtime_confirmed: u64,
    commands_precondition_blocked: u64,
    preconditions_observed: Vec<String>,
    traversal_complete: bool,
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

async fn compare(args: ContextCompareArgs) -> Result<ContextSummary> {
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

async fn build_context_suite(context_dirs: &[PathBuf]) -> Result<ContextSuite> {
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

async fn write_context_suite(root: &Path, json: &[u8], markdown: &[u8]) -> Result<()> {
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

async fn discover_context_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let dir = contexts_dir(root);
    let mut entries = match fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(source) if source.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(CliareError::ReadContextSuiteDir { path: dir, source });
        }
    };
    let mut dirs = Vec::new();
    while let Some(entry) =
        entries
            .next_entry()
            .await
            .map_err(|source| CliareError::ReadContextSuiteDir {
                path: dir.clone(),
                source,
            })?
    {
        let path = entry.path();
        let metadata =
            entry
                .metadata()
                .await
                .map_err(|source| CliareError::ReadContextSuiteDir {
                    path: path.clone(),
                    source,
                })?;
        if metadata.is_dir() && path.join(SCORECARD_JSON).is_file() {
            dirs.push(path);
        }
    }
    dirs.sort();
    Ok(dirs)
}

async fn read_runtime_context(dir: &Path) -> Result<Option<RuntimeContext>> {
    let path = dir.join(RUNTIME_CONTEXT_JSON);
    let bytes = match fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(CliareError::ReadContextScorecard { path, source });
        }
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|source| CliareError::ParseContextScorecard { path, source })
}

fn context_required_message(root: &Path, command: &str, contexts: &[PersistedContext]) -> String {
    format!(
        "{command} needs a concrete measurement context: {} is a context suite with persisted contexts: {}. Use --context <name> or pass a context artifact directory such as {}.",
        root.display(),
        context_list(contexts),
        contexts
            .first()
            .map(|context| context.artifact_dir.display().to_string())
            .unwrap_or_else(|| root.display().to_string())
    )
}

fn context_not_found_message(
    root: &Path,
    command: &str,
    requested: &str,
    contexts: &[PersistedContext],
) -> String {
    format!(
        "{command} could not find context `{}` under {}. Persisted contexts: {}.",
        requested,
        root.display(),
        if contexts.is_empty() {
            "none".to_owned()
        } else {
            context_list(contexts)
        }
    )
}

fn context_list(contexts: &[PersistedContext]) -> String {
    contexts
        .iter()
        .map(|context| format!("{} ({})", context.name, context.artifact_dir.display()))
        .collect::<Vec<_>>()
        .join(", ")
}

async fn invalid_measurement_artifact(root: &Path, command: &str) -> Result<CliareError> {
    let metadata = match fs::metadata(root).await {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == ErrorKind::NotFound => {
            return Ok(CliareError::MeasurementArtifactNotFound {
                command: command.to_owned(),
                path: root.to_path_buf(),
            });
        }
        Err(source) => {
            return Err(CliareError::ReadArtifactDirectory {
                path: root.to_path_buf(),
                source,
            });
        }
    };

    if !metadata.is_dir() {
        return Ok(CliareError::ArtifactPathNotDirectory {
            path: root.to_path_buf(),
        });
    }

    let child_dirs = child_directory_names(root).await?;
    let hint = if child_dirs.is_empty() {
        format!(
            "Run `cliare measure <target-cli> --out {}` first, or pass --out to an existing measurement directory.",
            root.display()
        )
    } else {
        format!(
            "This looks like a workspace root with child directories: {}. Pass --out {}/<project>, add --context <name> when that project has multiple contexts, or pass a concrete context directory.",
            child_dirs.join(", "),
            root.display()
        )
    };

    Ok(CliareError::InvalidMeasurementArtifact {
        message: format!(
            "{command} needs a CLIARE measurement artifact directory, but {} does not contain scorecard.json or contexts/<context>/scorecard.json. {hint}",
            root.display()
        ),
    })
}

async fn child_directory_names(root: &Path) -> Result<Vec<String>> {
    let mut entries =
        fs::read_dir(root)
            .await
            .map_err(|source| CliareError::ReadArtifactDirectory {
                path: root.to_path_buf(),
                source,
            })?;
    let mut dirs = Vec::new();
    while let Some(entry) =
        entries
            .next_entry()
            .await
            .map_err(|source| CliareError::ReadArtifactDirectory {
                path: root.to_path_buf(),
                source,
            })?
    {
        let metadata =
            entry
                .metadata()
                .await
                .map_err(|source| CliareError::ReadArtifactDirectory {
                    path: entry.path(),
                    source,
                })?;
        if metadata.is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            dirs.push(name.to_owned());
        }
    }
    dirs.sort();
    Ok(dirs)
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

fn precondition_chain(entries: &[ContextSuiteEntry]) -> Vec<String> {
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

fn render_context_suite(suite: &ContextSuite) -> String {
    let mut markdown = String::new();
    markdown.push_str("# CLIARE Context Comparison\n\n");
    if let Some(target) = &suite.target {
        markdown.push_str(&format!("- Target: `{}`\n", target.requested));
        markdown.push_str(&format!("- Resolved: `{}`\n", target.resolved));
        markdown.push_str(&format!("- Binary SHA-256: `{}`\n", target.binary_sha256));
    }
    markdown.push_str(&format!("- Contexts: `{}`\n", suite.contexts.len()));
    let chain = if suite.precondition_chain.is_empty() {
        "none".to_owned()
    } else {
        suite.precondition_chain.join(" -> ")
    };
    markdown.push_str(&format!("- Precondition chain: `{chain}`\n\n"));
    markdown.push_str(
        "| Context | Profile | Score | Commands | Preconditions | Traversal | Artifact Dir |\n",
    );
    markdown.push_str("|---|---|---:|---:|---|---|---|\n");
    for entry in &suite.contexts {
        let preconditions = if entry.preconditions_observed.is_empty() {
            "none".to_owned()
        } else {
            entry.preconditions_observed.join(", ")
        };
        markdown.push_str(&format!(
            "| `{}` | `{}` | {:.1} | {}/{} | `{}` | `{}` | `{}` |\n",
            escape_markdown_table(&entry.name),
            entry.profile.label(),
            entry.score,
            entry.commands_runtime_confirmed,
            entry.commands_discovered,
            preconditions,
            if entry.traversal_complete {
                "complete"
            } else {
                "partial"
            },
            entry.artifact_dir.display()
        ));
    }
    markdown.push('\n');
    markdown
}

fn sanitize_context_name(value: &str) -> String {
    let mut sanitized = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '-' | '_' | '.' | ' ') && !sanitized.ends_with('-') {
            sanitized.push('-');
        }
    }
    let sanitized = sanitized.trim_matches('-').to_owned();
    if sanitized.is_empty() {
        "context".to_owned()
    } else {
        sanitized
    }
}

fn escape_markdown_table(value: &str) -> String {
    value.replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        RuntimeContext, RuntimeContextCwdPolicy, RuntimeContextInput, RuntimeContextProfile,
        RuntimeContextState, measurement_dir, resolve_measurement_dir,
    };

    #[test]
    fn context_measurements_write_under_contexts_directory() {
        let context = RuntimeContext::from_input(RuntimeContextInput {
            profile: Some(RuntimeContextProfile::Authenticated),
            name: None,
            auth_state: None,
            local_context_state: None,
            fixture_state: None,
            network_state: None,
            runtime_dependency_state: None,
            workdir: None,
        });

        assert_eq!(
            measurement_dir(std::path::Path::new(".cliare/rote"), &context),
            std::path::Path::new(".cliare/rote/contexts/authenticated")
        );
        assert_eq!(context.auth_state, RuntimeContextState::Present);
        assert_eq!(context.local_context_state, RuntimeContextState::Absent);
    }

    #[test]
    fn context_workdir_marks_local_context_present() {
        let context = RuntimeContext::from_input(RuntimeContextInput {
            profile: Some(RuntimeContextProfile::LocalContext),
            name: Some("repo".to_owned()),
            auth_state: None,
            local_context_state: None,
            fixture_state: None,
            network_state: None,
            runtime_dependency_state: None,
            workdir: Some("/tmp/repo".into()),
        });

        assert_eq!(context.local_context_state, RuntimeContextState::Present);
        assert_eq!(context.cwd_policy, RuntimeContextCwdPolicy::Provided);
        assert_eq!(context.folder_name(), "repo");
    }

    #[tokio::test]
    async fn resolver_requires_context_for_multi_context_suite_root() {
        let root = unique_test_dir("context-resolver-required");
        write_context_fixture(&root, "clean", RuntimeContextProfile::Clean).await;
        write_context_fixture(&root, "local-context", RuntimeContextProfile::LocalContext).await;

        let error = resolve_measurement_dir(&root, None, "cliare report")
            .await
            .expect_err("multi-context suite root needs explicit context");
        let message = error.to_string();

        assert!(message.contains("cliare report needs a concrete measurement context"));
        assert!(message.contains("clean"));
        assert!(message.contains("local-context"));

        let _ = tokio::fs::remove_dir_all(root).await;
    }

    #[tokio::test]
    async fn resolver_selects_explicit_context_from_suite_root() {
        let root = unique_test_dir("context-resolver-explicit");
        let clean = write_context_fixture(&root, "clean", RuntimeContextProfile::Clean).await;
        let local =
            write_context_fixture(&root, "local-context", RuntimeContextProfile::LocalContext)
                .await;

        assert_eq!(
            resolve_measurement_dir(&root, Some("clean"), "cliare report")
                .await
                .expect("clean context resolves"),
            clean
        );
        assert_eq!(
            resolve_measurement_dir(&root, Some("local_context"), "cliare report")
                .await
                .expect("underscore aliases sanitize to context folder"),
            local
        );

        let _ = tokio::fs::remove_dir_all(root).await;
    }

    #[tokio::test]
    async fn resolver_rejects_missing_measurement_root() {
        let root = unique_test_dir("context-resolver-missing");

        let error = resolve_measurement_dir(&root, None, "cliare issues list")
            .await
            .expect_err("missing artifact root is rejected");
        let message = error.to_string();

        assert!(message.contains("cliare issues list could not find"));
        assert!(message.contains(&root.display().to_string()));
    }

    #[tokio::test]
    async fn resolver_rejects_workspace_root_without_project_selection() {
        let root = unique_test_dir("context-resolver-workspace");
        tokio::fs::create_dir_all(root.join("rote"))
            .await
            .expect("creates rote workspace child");
        tokio::fs::create_dir_all(root.join("corpus-runs"))
            .await
            .expect("creates corpus workspace child");

        let error = resolve_measurement_dir(&root, None, "cliare issues list")
            .await
            .expect_err("workspace root is not a measurement");
        let message = error.to_string();

        assert!(message.contains("does not contain scorecard.json"));
        assert!(message.contains("corpus-runs, rote"));
        assert!(message.contains("Pass --out"));
        assert!(message.contains("<project>"));

        let _ = tokio::fs::remove_dir_all(root).await;
    }

    async fn write_context_fixture(
        root: &std::path::Path,
        name: &str,
        profile: RuntimeContextProfile,
    ) -> std::path::PathBuf {
        let dir = root.join("contexts").join(name);
        tokio::fs::create_dir_all(&dir)
            .await
            .expect("context fixture directory is created");
        tokio::fs::write(dir.join("scorecard.json"), "{}")
            .await
            .expect("scorecard marker is written");
        let context = RuntimeContext::from_input(RuntimeContextInput {
            profile: Some(profile),
            name: Some(name.to_owned()),
            auth_state: None,
            local_context_state: None,
            fixture_state: None,
            network_state: None,
            runtime_dependency_state: None,
            workdir: None,
        });
        let bytes = serde_json::to_vec(&context).expect("runtime context serializes");
        tokio::fs::write(dir.join("runtime-context.json"), bytes)
            .await
            .expect("runtime context is written");
        dir
    }

    fn unique_test_dir(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cliare-{name}-{}-{nonce}", std::process::id()))
    }
}
