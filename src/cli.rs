use std::path::PathBuf;
use std::time::Duration;

use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum, ValueHint};
use serde::{Deserialize, Serialize};

use crate::context::{ContextArgs, RuntimeContextProfile, RuntimeContextState};
use crate::issue_disposition::IssueDispositionStatus;
use crate::sandbox::{
    DEFAULT_SNAPSHOT_MAX_DIRS, DEFAULT_SNAPSHOT_MAX_FILES, DEFAULT_SNAPSHOT_MAX_HASH_BYTES,
    SandboxProfile, SnapshotLimits,
};

pub const QUICK_MAX_DEPTH: usize = 3;
pub const QUICK_MAX_PROBES: usize = 64;
pub const QUICK_MIN_EXPECTED_VALUE: u16 = 300;
pub const QUICK_CONCURRENCY: usize = 2;
pub const STANDARD_MAX_DEPTH: usize = 5;
pub const STANDARD_MAX_PROBES: usize = 256;
pub const STANDARD_MIN_EXPECTED_VALUE: u16 = 150;
pub const STANDARD_CONCURRENCY: usize = 4;
pub const DEEP_MAX_DEPTH: usize = 8;
pub const DEEP_MAX_PROBES: usize = 1_000;
pub const DEEP_MIN_EXPECTED_VALUE: u16 = 50;
pub const DEEP_CONCURRENCY: usize = 8;

#[derive(Debug, Parser)]
#[command(name = "cliare")]
#[command(version)]
#[command(about = "Measure CLI agent readiness from runtime evidence")]
#[command(
    long_about = "CLIARE measures command-line interfaces by probing runtime behavior, recording evidence, and producing agent-readiness artifacts."
)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run safe bootstrap probes and write an evidence log.
    Measure(MeasureArgs),
    /// Inspect detached CLIARE jobs.
    Jobs(JobsArgs),
    /// Measure a target and fail when score regresses against a baseline.
    Guard(GuardArgs),
    /// Run a benchmark corpus and produce calibration reports.
    Benchmark(BenchmarkArgs),
    /// Compare measurements across runtime contexts.
    Context(ContextArgs),
    /// Generate a persona-specific outcome packet from measurement artifacts.
    Report(ReportArgs),
    /// Describe a CLIARE artifact directory for humans and agents.
    Describe(DescribeArgs),
    /// Install CLIARE artifact-review skills for coding agents.
    Skills(SkillsArgs),
    /// Review, mark, and list evidence-backed CLIARE issues.
    Issues(IssuesArgs),
    /// Query the measured command surface for harness routing.
    Surface(SurfaceArgs),
    /// Print role-specific operational playbooks.
    Playbook(PlaybookArgs),
    /// Print CLIARE implementation metadata.
    Metadata(MetadataArgs),
}

#[derive(Debug, Args)]
#[command(
    long_about = "Print a role-specific operational playbook. Maintainers get the measure, view, act, disposition, remeasure, CI, and agent-surface publishing loop. Harness and security teams get focused execution loops over the same CLIARE artifacts.",
    after_help = "Available playbooks:
  maintainer  Measure, inspect, fix or disposition, remeasure, gate in CI, and publish the agent surface.
  harness     Consume the command index, harness packet, and generated skill to route agents through the CLI deliberately.
  security    Review safety, credential-like side effects, host/auth exposure, and policy evidence before approving agent use.

Maintainer workflow:
  1. Measure: cliare measure <target-cli> --out .cliare/<target-cli> --profile quick|standard|deep --refresh
  2. View: cliare report maintainer --out .cliare/<target-cli> --format markdown
  3. Act or disposition: fix the CLI, or use cliare issues mark <issue-id> --status intentional|needs-fixture
  4. Remeasure: cliare measure <target-cli> --out .cliare/<target-cli> --profile deep --refresh
  5. Gate: cliare guard <target-cli> --baseline .cliare-baseline/<target-cli>/scorecard.json --out .cliare/<target-cli> --profile deep
  6. Publish: cliare describe .cliare/<target-cli> --write && cliare report harness --out .cliare/<target-cli> --write

Measure profiles used by generated commands: `quick` is the small local smoke pass, `standard` is the normal maintainer loop, and `deep` is the broader release-quality pass for CI baselines, releases, and agent-surface publishing.

Advanced traversal knobs:
  --max-depth controls recursive command-path depth.
  --max-probes controls total runtime probes.
  --concurrency controls simultaneous probes.
  --execution-mode host measures authenticated or host-specific behavior.

Do not pass --profile to `cliare playbook`; pass it to `cliare measure` or `cliare guard`.
`.cliare/<target-cli>` is a project-scoped artifact directory, relative to the directory where you run CLIARE.
Run `cliare playbook maintainer --target <target-cli>`, `cliare playbook harness --target <target-cli>`, or `cliare playbook security --target <target-cli>` to print the full command-by-command guide."
)]
pub struct PlaybookArgs {
    /// Playbook role to print.
    #[arg(value_enum)]
    pub role: PlaybookRole,

    /// Target CLI name or path to use in generated commands.
    #[arg(long, value_name = "TARGET", value_hint = ValueHint::CommandName)]
    pub target: Option<String>,

    /// Measurement artifact directory to use in generated commands.
    #[arg(
        long,
        value_name = "DIR",
        default_value = ".cliare/<target-cli>",
        value_hint = ValueHint::DirPath
    )]
    pub out: PathBuf,

    /// Context name to use in generated report, issue, and describe commands.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = PlaybookFormat::Human)]
    pub format: PlaybookFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PlaybookRole {
    Maintainer,
    Harness,
    Security,
}

impl PlaybookRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Maintainer => "maintainer",
            Self::Harness => "harness",
            Self::Security => "security",
        }
    }
}

impl std::fmt::Display for PlaybookRole {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PlaybookFormat {
    Human,
    Markdown,
    Json,
}

#[derive(Debug, Args)]
#[command(
    after_help = "For the end-to-end maintainer workflow and parameter guide, run: cliare playbook maintainer"
)]
pub struct IssuesArgs {
    #[command(subcommand)]
    pub command: IssuesCommand,
}

#[derive(Debug, Subcommand)]
pub enum IssuesCommand {
    /// Record a maintainer disposition for an issue id.
    Mark(IssuesMarkArgs),
    /// List generated issues with maintainer dispositions.
    List(IssuesListArgs),
}

#[derive(Debug, Args)]
pub struct IssuesMarkArgs {
    /// Issue id to mark, such as issue.output_mode_unprobed.
    pub issue_id: String,

    /// Measurement artifact directory containing issues.json or issue-dispositions.json.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Context name to select when --out points at a context suite root.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,

    /// Maintainer disposition to record.
    #[arg(long, value_enum)]
    pub status: IssueDispositionStatus,

    /// Maintainer rationale for the disposition.
    #[arg(long)]
    pub reason: String,
}

#[derive(Debug, Args)]
pub struct IssuesListArgs {
    /// Measurement artifact directory containing issues.json or issue-dispositions.json.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Context name to select when --out points at a context suite root.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = IssuesListFormat::Markdown)]
    pub format: IssuesListFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum IssuesListFormat {
    Human,
    Markdown,
    Json,
}

#[derive(Debug, Args)]
#[command(
    after_help = "Harnesses should use `surface query` or `surface explain` instead of parsing command-index.json directly."
)]
pub struct SurfaceArgs {
    #[command(subcommand)]
    pub command: SurfaceCommand,
}

#[derive(Debug, Subcommand)]
pub enum SurfaceCommand {
    /// Rank commands that match a harness intent.
    Query(SurfaceQueryArgs),
    /// Explain one measured command path for harness routing.
    Explain(SurfaceExplainArgs),
    /// List measured commands with optional readiness filtering.
    List(SurfaceListArgs),
}

#[derive(Debug, Args)]
pub struct SurfaceQueryArgs {
    /// Harness intent to match, such as "check job status".
    #[arg(value_name = "INTENT")]
    pub intent: String,

    /// Measurement artifact directory containing command-index.json.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Context name to select when --out points at a context suite root.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,

    /// Only return commands with this output capability.
    #[arg(long, value_enum)]
    pub require_output: Option<SurfaceOutputRequirement>,

    /// Maximum ranked matches to return.
    #[arg(long, value_name = "N", default_value_t = 5, value_parser = parse_positive_usize)]
    pub limit: usize,

    /// Representation to print to stdout.
    #[arg(long, value_enum, default_value_t = SurfaceFormat::Json)]
    pub format: SurfaceFormat,
}

#[derive(Debug, Args)]
pub struct SurfaceExplainArgs {
    /// Command path to explain, such as "jobs status".
    #[arg(value_name = "COMMAND", num_args = 1..)]
    pub command: Vec<String>,

    /// Measurement artifact directory containing command-index.json.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Context name to select when --out points at a context suite root.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,

    /// Add this output capability to the synthesized argv template when available.
    #[arg(long, value_enum)]
    pub require_output: Option<SurfaceOutputRequirement>,

    /// Representation to print to stdout.
    #[arg(long, value_enum, default_value_t = SurfaceFormat::Json)]
    pub format: SurfaceFormat,
}

#[derive(Debug, Args)]
pub struct SurfaceListArgs {
    /// Measurement artifact directory containing command-index.json.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Context name to select when --out points at a context suite root.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,

    /// Only list commands with this readiness state.
    #[arg(long, value_enum)]
    pub state: Option<SurfaceReadiness>,

    /// Only list commands with this output capability.
    #[arg(long, value_enum)]
    pub require_output: Option<SurfaceOutputRequirement>,

    /// Maximum commands to return.
    #[arg(long, value_name = "N", default_value_t = 50, value_parser = parse_positive_usize)]
    pub limit: usize,

    /// Representation to print to stdout.
    #[arg(long, value_enum, default_value_t = SurfaceFormat::Json)]
    pub format: SurfaceFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceReadiness {
    Ready,
    Conditional,
    NeedsFixture,
    Blocked,
    Candidate,
}

impl SurfaceReadiness {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Conditional => "conditional",
            Self::NeedsFixture => "needs_fixture",
            Self::Blocked => "blocked",
            Self::Candidate => "candidate",
        }
    }
}

impl std::fmt::Display for SurfaceReadiness {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SurfaceOutputRequirement {
    Json,
    Yaml,
    MachineReadable,
}

impl SurfaceOutputRequirement {
    pub fn label(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::MachineReadable => "machine-readable",
        }
    }
}

impl std::fmt::Display for SurfaceOutputRequirement {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Debug, Args)]
pub struct SkillsArgs {
    #[command(subcommand)]
    pub command: SkillsCommand,
}

#[derive(Debug, Subcommand)]
pub enum SkillsCommand {
    /// List installable CLIARE agent skill targets.
    List(SkillsListArgs),
    /// Install CLIARE skills into a local agent configuration directory.
    Install(SkillsInstallArgs),
}

#[derive(Debug, Args)]
pub struct SkillsListArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = SkillsListFormat::Text)]
    pub format: SkillsListFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SkillsListFormat {
    Text,
    Json,
}

#[derive(Debug, Args)]
pub struct SkillsInstallArgs {
    /// Agent integration to install.
    #[arg(long, value_enum, default_value_t = SkillAgent::All)]
    pub agent: SkillAgent,

    /// Install into user-level or project-level agent directories.
    #[arg(long, value_enum, default_value_t = SkillInstallScope::User)]
    pub scope: SkillInstallScope,

    /// User home directory override for user-scope installs.
    #[arg(long, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub home: Option<PathBuf>,

    /// Project directory override for project-scope installs.
    #[arg(long, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub project_dir: Option<PathBuf>,

    /// Show planned writes without changing files.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SkillAgent {
    All,
    Claude,
    Codex,
    Cursor,
}

impl SkillAgent {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
        }
    }
}

impl std::fmt::Display for SkillAgent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SkillInstallScope {
    User,
    Project,
}

impl SkillInstallScope {
    pub fn label(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
        }
    }
}

impl std::fmt::Display for SkillInstallScope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Debug, Args)]
#[command(disable_help_flag = true)]
pub struct MetadataArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = MetadataFormat::Text)]
    pub format: MetadataFormat,

    /// Print help. With --format json, emit a parseable metadata contract.
    #[arg(long)]
    pub help: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum MetadataFormat {
    Text,
    Json,
}

impl MetadataFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
        }
    }
}

#[derive(Debug, Args)]
pub struct BenchmarkArgs {
    /// Benchmark corpus manifest.
    #[arg(
        long,
        value_name = "FILE",
        default_value = "benchmarks/local-corpus.json",
        value_hint = ValueHint::FilePath
    )]
    pub manifest: PathBuf,

    /// Output directory for benchmark artifacts.
    #[arg(
        long,
        value_name = "DIR",
        default_value = ".cliare-bench",
        value_hint = ValueHint::DirPath
    )]
    pub out: PathBuf,

    /// Maximum benchmark targets to measure concurrently.
    #[arg(long, value_name = "N", value_parser = parse_positive_usize)]
    pub target_concurrency: Option<usize>,

    /// Ignore reusable measurement artifacts and run probes again.
    #[arg(long)]
    pub refresh: bool,
}

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("report_filter")
        .args(["area", "issue"])
        .multiple(false)
))]
#[command(
    after_help = "For the end-to-end maintainer workflow and parameter guide, run: cliare playbook maintainer"
)]
pub struct ReportArgs {
    /// Persona packet to generate.
    #[arg(value_enum)]
    pub persona: ReportPersona,

    /// Measurement artifact directory containing scorecard.json, command-index.json, shape.json, and evidence.jsonl.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Context name to select when --out points at a context suite root.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,

    /// Representation to print to stdout.
    #[arg(long, value_enum, default_value_t = ReportFormat::Markdown)]
    pub format: ReportFormat,

    /// Limit output to one agent-readiness area.
    #[arg(long, value_enum)]
    pub area: Option<ReportArea>,

    /// Limit output to one issue id, such as issue.output_mode_unprobed.
    #[arg(long, value_name = "ID")]
    pub issue: Option<String>,

    /// Include attached evidence entries in focused report output.
    #[arg(long)]
    pub with_evidence: bool,

    /// Write persona-<persona>.json and persona-<persona>.md into the artifact directory.
    #[arg(long)]
    pub write: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ReportPersona {
    Maintainer,
    Harness,
    Platform,
    Security,
    Oss,
    Devrel,
    Research,
}

impl ReportPersona {
    pub fn label(self) -> &'static str {
        match self {
            Self::Maintainer => "maintainer",
            Self::Harness => "harness",
            Self::Platform => "platform",
            Self::Security => "security",
            Self::Oss => "oss",
            Self::Devrel => "devrel",
            Self::Research => "research",
        }
    }
}

impl std::fmt::Display for ReportPersona {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ReportFormat {
    Markdown,
    Json,
    Bundle,
}

impl ReportFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Json => "json",
            Self::Bundle => "bundle",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ReportArea {
    OutputContracts,
    Preconditions,
    CommandDiscovery,
    HelpCoverage,
    Compatibility,
    Diagnostics,
    Execution,
    Safety,
    Coverage,
    Policy,
    Publishing,
    Calibration,
}

impl ReportArea {
    pub fn label(self) -> &'static str {
        match self {
            Self::OutputContracts => "output-contracts",
            Self::Preconditions => "preconditions",
            Self::CommandDiscovery => "command-discovery",
            Self::HelpCoverage => "help-coverage",
            Self::Compatibility => "compatibility",
            Self::Diagnostics => "diagnostics",
            Self::Execution => "execution",
            Self::Safety => "safety",
            Self::Coverage => "coverage",
            Self::Policy => "policy",
            Self::Publishing => "publishing",
            Self::Calibration => "calibration",
        }
    }
}

impl std::fmt::Display for ReportArea {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

#[derive(Debug, Args)]
pub struct DescribeArgs {
    /// CLIARE measurement or benchmark artifact directory.
    #[arg(value_name = "FOLDER", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub folder: PathBuf,

    /// Context name to describe when FOLDER points at a context suite root.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,

    /// Representation to print to stdout.
    #[arg(long, value_enum, default_value_t = DescribeFormat::Markdown)]
    pub format: DescribeFormat,

    /// Write artifact-map.json and artifact-map.md into the artifact directory.
    #[arg(long)]
    pub write: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum DescribeFormat {
    Markdown,
    Json,
}

impl DescribeFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Json => "json",
        }
    }
}

#[derive(Debug, Args)]
pub struct JobsArgs {
    #[command(subcommand)]
    pub command: JobsCommand,
}

#[derive(Debug, Subcommand)]
pub enum JobsCommand {
    /// Print the latest detached or foreground measurement progress state.
    Status(JobsStatusArgs),
}

#[derive(Debug, Args)]
pub struct JobsStatusArgs {
    /// Measurement artifact directory containing jobs/current.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Context name to select when --out points at a context suite root.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,
}

#[derive(Debug, Clone, Args)]
#[command(
    after_help = "For profile selection, probe budgets, and the maintainer workflow, run: cliare playbook maintainer"
)]
pub struct MeasureArgs {
    /// Path or PATH-resolved command name for the target CLI.
    #[arg(value_name = "TARGET", value_hint = ValueHint::CommandName)]
    pub target: PathBuf,

    /// Output directory for CLIARE artifacts.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Per-probe timeout in milliseconds.
    #[arg(long, value_name = "MS", default_value_t = 5_000)]
    pub timeout_ms: u64,

    /// Maximum stdout bytes and stderr bytes retained per probe.
    #[arg(long, value_name = "BYTES", default_value_t = 1_048_576)]
    pub output_limit_bytes: usize,

    /// Traversal budget preset.
    #[arg(long, value_enum, default_value_t = TraversalProfile::Standard)]
    pub profile: TraversalProfile,

    /// Probe execution environment.
    #[arg(long, value_enum, default_value_t = SandboxProfile::Isolated)]
    pub execution_mode: SandboxProfile,

    /// Maximum command-path depth to recursively confirm.
    #[arg(long, value_name = "N")]
    pub max_depth: Option<usize>,

    /// Maximum probes to execute for this run.
    #[arg(long, value_name = "N")]
    pub max_probes: Option<usize>,

    /// Minimum expected value for dynamically scheduled probes.
    #[arg(long, value_name = "N")]
    pub min_expected_value: Option<u16>,

    /// Maximum probes to run concurrently.
    #[arg(long, value_name = "N", value_parser = parse_positive_usize)]
    pub concurrency: Option<usize>,

    /// Maximum files scanned per side-effect snapshot.
    #[arg(long, value_name = "N", value_parser = parse_positive_usize)]
    pub snapshot_max_files: Option<usize>,

    /// Maximum directories scanned per side-effect snapshot.
    #[arg(long, value_name = "N", value_parser = parse_positive_usize)]
    pub snapshot_max_directories: Option<usize>,

    /// Maximum bytes hashed per side-effect snapshot.
    #[arg(long, value_name = "BYTES", value_parser = parse_positive_u64)]
    pub snapshot_max_hash_bytes: Option<u64>,

    /// Runtime context profile. When set, --out is a suite root and artifacts are written under contexts/<context>.
    #[arg(long, value_enum)]
    pub context: Option<RuntimeContextProfile>,

    /// Override the context folder/display name.
    #[arg(long, value_name = "NAME")]
    pub context_name: Option<String>,

    /// Declared authentication state for this runtime context.
    #[arg(long, value_enum)]
    pub auth_state: Option<RuntimeContextState>,

    /// Declared local workspace/project/repository context state.
    #[arg(long, value_enum)]
    pub local_context_state: Option<RuntimeContextState>,

    /// Declared fixture-data state for this runtime context.
    #[arg(long, value_enum)]
    pub fixture_state: Option<RuntimeContextState>,

    /// Declared network state for this runtime context.
    #[arg(long, value_enum)]
    pub network_state: Option<RuntimeContextState>,

    /// Declared local runtime dependency state for this runtime context.
    #[arg(long, value_enum)]
    pub runtime_dependency_state: Option<RuntimeContextState>,

    /// Working directory that supplies the local context under test.
    #[arg(long, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub context_workdir: Option<PathBuf>,

    /// Ignore reusable artifacts and run probes again.
    #[arg(long)]
    pub refresh: bool,

    /// Start the measurement in the background and return immediately with a job id.
    #[arg(long)]
    pub detach: bool,

    /// Internal worker mode used by `measure --detach`.
    #[arg(long = "__cliare-detached-worker", hide = true)]
    pub detached_worker: bool,

    /// Internal job id supplied by `measure --detach`.
    #[arg(long = "__cliare-job-id", hide = true)]
    pub job_id: Option<String>,
}

impl MeasureArgs {
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }

    pub fn resolved_max_depth(&self) -> usize {
        self.max_depth
            .unwrap_or_else(|| self.profile.default_max_depth())
    }

    pub fn resolved_max_probes(&self) -> usize {
        self.max_probes
            .unwrap_or_else(|| self.profile.default_max_probes())
    }

    pub fn resolved_min_expected_value(&self) -> u16 {
        self.min_expected_value
            .unwrap_or_else(|| self.profile.default_min_expected_value())
    }

    pub fn resolved_concurrency(&self) -> usize {
        self.concurrency
            .unwrap_or_else(|| self.profile.default_concurrency())
    }

    pub fn snapshot_limits(&self) -> SnapshotLimits {
        SnapshotLimits::new(
            self.snapshot_max_files
                .unwrap_or(DEFAULT_SNAPSHOT_MAX_FILES),
            self.snapshot_max_directories
                .unwrap_or(DEFAULT_SNAPSHOT_MAX_DIRS),
            self.snapshot_max_hash_bytes
                .unwrap_or(DEFAULT_SNAPSHOT_MAX_HASH_BYTES),
        )
    }
}

#[derive(Debug, Args)]
pub struct GuardArgs {
    /// Path or PATH-resolved command name for the target CLI.
    #[arg(value_name = "TARGET", value_hint = ValueHint::CommandName)]
    pub target: PathBuf,

    /// Baseline scorecard to compare against.
    #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub baseline: PathBuf,

    /// Output directory for CLIARE artifacts.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Allowed score drop before guard fails.
    #[arg(long, value_name = "POINTS", default_value_t = 0.0)]
    pub allowed_drop: f64,

    /// Policy file with score thresholds and side-effect rules.
    #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub policy: Option<PathBuf>,

    /// Per-probe timeout in milliseconds.
    #[arg(long, value_name = "MS", default_value_t = 5_000)]
    pub timeout_ms: u64,

    /// Maximum stdout bytes and stderr bytes retained per probe.
    #[arg(long, value_name = "BYTES", default_value_t = 1_048_576)]
    pub output_limit_bytes: usize,

    /// Traversal budget preset.
    #[arg(long, value_enum, default_value_t = TraversalProfile::Standard)]
    pub profile: TraversalProfile,

    /// Probe execution environment.
    #[arg(long, value_enum, default_value_t = SandboxProfile::Isolated)]
    pub execution_mode: SandboxProfile,

    /// Maximum command-path depth to recursively confirm.
    #[arg(long, value_name = "N")]
    pub max_depth: Option<usize>,

    /// Maximum probes to execute for this run.
    #[arg(long, value_name = "N")]
    pub max_probes: Option<usize>,

    /// Minimum expected value for dynamically scheduled probes.
    #[arg(long, value_name = "N")]
    pub min_expected_value: Option<u16>,

    /// Maximum probes to run concurrently.
    #[arg(long, value_name = "N", value_parser = parse_positive_usize)]
    pub concurrency: Option<usize>,

    /// Maximum files scanned per side-effect snapshot.
    #[arg(long, value_name = "N", value_parser = parse_positive_usize)]
    pub snapshot_max_files: Option<usize>,

    /// Maximum directories scanned per side-effect snapshot.
    #[arg(long, value_name = "N", value_parser = parse_positive_usize)]
    pub snapshot_max_directories: Option<usize>,

    /// Maximum bytes hashed per side-effect snapshot.
    #[arg(long, value_name = "BYTES", value_parser = parse_positive_u64)]
    pub snapshot_max_hash_bytes: Option<u64>,

    /// Runtime context profile. When set, --out is a suite root and artifacts are written under contexts/<context>.
    #[arg(long, value_enum)]
    pub context: Option<RuntimeContextProfile>,

    /// Override the context folder/display name.
    #[arg(long, value_name = "NAME")]
    pub context_name: Option<String>,

    /// Declared authentication state for this runtime context.
    #[arg(long, value_enum)]
    pub auth_state: Option<RuntimeContextState>,

    /// Declared local workspace/project/repository context state.
    #[arg(long, value_enum)]
    pub local_context_state: Option<RuntimeContextState>,

    /// Declared fixture-data state for this runtime context.
    #[arg(long, value_enum)]
    pub fixture_state: Option<RuntimeContextState>,

    /// Declared network state for this runtime context.
    #[arg(long, value_enum)]
    pub network_state: Option<RuntimeContextState>,

    /// Declared local runtime dependency state for this runtime context.
    #[arg(long, value_enum)]
    pub runtime_dependency_state: Option<RuntimeContextState>,

    /// Working directory that supplies the local context under test.
    #[arg(long, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub context_workdir: Option<PathBuf>,

    /// Ignore reusable artifacts and run probes again.
    #[arg(long)]
    pub refresh: bool,
}

impl From<&GuardArgs> for MeasureArgs {
    fn from(args: &GuardArgs) -> Self {
        Self {
            target: args.target.clone(),
            out: args.out.clone(),
            timeout_ms: args.timeout_ms,
            output_limit_bytes: args.output_limit_bytes,
            profile: args.profile,
            execution_mode: args.execution_mode,
            max_depth: args.max_depth,
            max_probes: args.max_probes,
            min_expected_value: args.min_expected_value,
            concurrency: args.concurrency,
            snapshot_max_files: args.snapshot_max_files,
            snapshot_max_directories: args.snapshot_max_directories,
            snapshot_max_hash_bytes: args.snapshot_max_hash_bytes,
            context: args.context,
            context_name: args.context_name.clone(),
            auth_state: args.auth_state,
            local_context_state: args.local_context_state,
            fixture_state: args.fixture_state,
            network_state: args.network_state,
            runtime_dependency_state: args.runtime_dependency_state,
            context_workdir: args.context_workdir.clone(),
            refresh: args.refresh,
            detach: false,
            detached_worker: false,
            job_id: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum TraversalProfile {
    Quick,
    Standard,
    Deep,
}

impl TraversalProfile {
    pub fn default_max_depth(self) -> usize {
        match self {
            Self::Quick => QUICK_MAX_DEPTH,
            Self::Standard => STANDARD_MAX_DEPTH,
            Self::Deep => DEEP_MAX_DEPTH,
        }
    }

    pub fn default_max_probes(self) -> usize {
        match self {
            Self::Quick => QUICK_MAX_PROBES,
            Self::Standard => STANDARD_MAX_PROBES,
            Self::Deep => DEEP_MAX_PROBES,
        }
    }

    pub fn default_min_expected_value(self) -> u16 {
        match self {
            Self::Quick => QUICK_MIN_EXPECTED_VALUE,
            Self::Standard => STANDARD_MIN_EXPECTED_VALUE,
            Self::Deep => DEEP_MIN_EXPECTED_VALUE,
        }
    }

    pub fn default_concurrency(self) -> usize {
        match self {
            Self::Quick => QUICK_CONCURRENCY,
            Self::Standard => STANDARD_CONCURRENCY,
            Self::Deep => DEEP_CONCURRENCY,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Standard => "standard",
            Self::Deep => "deep",
        }
    }
}

impl std::fmt::Display for TraversalProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}

fn parse_positive_usize(raw: &str) -> std::result::Result<usize, String> {
    let value = raw
        .parse::<usize>()
        .map_err(|source| format!("expected positive integer: {source}"))?;
    if value == 0 {
        Err("expected positive integer greater than zero".to_owned())
    } else {
        Ok(value)
    }
}

fn parse_positive_u64(raw: &str) -> std::result::Result<u64, String> {
    let value = raw
        .parse::<u64>()
        .map_err(|source| format!("expected positive integer: {source}"))?;
    if value == 0 {
        Err("expected positive integer greater than zero".to_owned())
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use crate::sandbox::SnapshotLimits;

    use super::{
        Cli, Command, DEEP_CONCURRENCY, DEEP_MAX_DEPTH, DEEP_MAX_PROBES, DEEP_MIN_EXPECTED_VALUE,
        QUICK_CONCURRENCY, QUICK_MAX_DEPTH, QUICK_MAX_PROBES, QUICK_MIN_EXPECTED_VALUE,
        STANDARD_CONCURRENCY, STANDARD_MAX_DEPTH, STANDARD_MAX_PROBES, STANDARD_MIN_EXPECTED_VALUE,
        TraversalProfile,
    };

    #[test]
    fn clap_surface_exposes_measure_and_global_version() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();

        assert!(help.contains("Usage: cliare"));
        assert!(help.contains("measure"));
        assert!(help.contains("jobs"));
        assert!(help.contains("guard"));
        assert!(help.contains("benchmark"));
        assert!(help.contains("report"));
        assert!(help.contains("describe"));
        assert!(help.contains("skills"));
        assert!(help.contains("issues"));
        assert!(help.contains("surface"));
        assert!(help.contains("playbook"));
        assert!(help.contains("context"));
        assert!(help.contains("metadata"));
        assert!(help.contains("--version"));
    }

    #[test]
    fn measure_accepts_configurable_snapshot_limits() {
        let cli = Cli::try_parse_from([
            "cliare",
            "measure",
            "target",
            "--snapshot-max-files",
            "12",
            "--snapshot-max-directories",
            "13",
            "--snapshot-max-hash-bytes",
            "14",
        ])
        .expect("valid measure command");

        match cli.command {
            Command::Measure(args) => {
                assert_eq!(args.snapshot_limits(), SnapshotLimits::new(12, 13, 14));
            }
            Command::Guard(_)
            | Command::Jobs(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected measure command")
            }
        }
    }

    #[test]
    fn measure_and_guard_share_deep_recursion_defaults() {
        let measure = Cli::try_parse_from(["cliare", "measure", "target"]).expect("valid measure");
        let guard =
            Cli::try_parse_from(["cliare", "guard", "target", "--baseline", "scorecard.json"])
                .expect("valid guard");

        match measure.command {
            Command::Measure(args) => {
                assert_eq!(args.profile, TraversalProfile::Standard);
                assert_eq!(args.resolved_max_depth(), STANDARD_MAX_DEPTH);
                assert_eq!(args.resolved_max_probes(), STANDARD_MAX_PROBES);
                assert_eq!(
                    args.resolved_min_expected_value(),
                    STANDARD_MIN_EXPECTED_VALUE
                );
                assert_eq!(args.resolved_concurrency(), STANDARD_CONCURRENCY);
            }
            Command::Guard(_)
            | Command::Jobs(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected measure command")
            }
        }

        match guard.command {
            Command::Guard(args) => {
                let measure_args = super::MeasureArgs::from(&args);
                assert_eq!(args.profile, TraversalProfile::Standard);
                assert_eq!(measure_args.resolved_max_depth(), STANDARD_MAX_DEPTH);
                assert_eq!(measure_args.resolved_max_probes(), STANDARD_MAX_PROBES);
                assert_eq!(
                    measure_args.resolved_min_expected_value(),
                    STANDARD_MIN_EXPECTED_VALUE
                );
                assert_eq!(measure_args.resolved_concurrency(), STANDARD_CONCURRENCY);
            }
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Benchmark(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Context(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected guard command")
            }
        }
    }

    #[test]
    fn benchmark_uses_local_corpus_defaults() {
        let cli = Cli::try_parse_from(["cliare", "benchmark"]).expect("valid benchmark");

        match cli.command {
            Command::Benchmark(args) => {
                assert_eq!(
                    args.manifest,
                    std::path::PathBuf::from("benchmarks/local-corpus.json")
                );
                assert_eq!(args.out, std::path::PathBuf::from(".cliare-bench"));
                assert_eq!(args.target_concurrency, None);
                assert!(!args.refresh);
            }
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Guard(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected benchmark command")
            }
        }
    }

    #[test]
    fn report_accepts_persona_and_output_options() {
        let cli = Cli::try_parse_from([
            "cliare",
            "report",
            "security",
            "--out",
            ".cliare-current",
            "--context",
            "clean",
            "--format",
            "json",
            "--write",
        ])
        .expect("valid report command");

        match cli.command {
            Command::Report(args) => {
                assert_eq!(args.persona, super::ReportPersona::Security);
                assert_eq!(args.out, std::path::PathBuf::from(".cliare-current"));
                assert_eq!(args.context.as_deref(), Some("clean"));
                assert_eq!(args.format, super::ReportFormat::Json);
                assert!(args.write);
            }
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected report command")
            }
        }
    }

    #[test]
    fn report_accepts_typed_drilldown_options() {
        let cli = Cli::try_parse_from([
            "cliare",
            "report",
            "maintainer",
            "--area",
            "output-contracts",
            "--with-evidence",
            "--format",
            "bundle",
        ])
        .expect("valid focused report command");

        match cli.command {
            Command::Report(args) => {
                assert_eq!(args.persona, super::ReportPersona::Maintainer);
                assert_eq!(args.area, Some(super::ReportArea::OutputContracts));
                assert_eq!(args.issue, None);
                assert!(args.with_evidence);
                assert_eq!(args.format, super::ReportFormat::Bundle);
            }
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected report command")
            }
        }
    }

    #[test]
    fn report_rejects_area_and_issue_together() {
        let error = Cli::try_parse_from([
            "cliare",
            "report",
            "maintainer",
            "--area",
            "output-contracts",
            "--issue",
            "issue.output_mode_unprobed",
        ])
        .expect_err("area and issue filters are mutually exclusive");

        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn skills_install_accepts_agent_scope_and_dry_run() {
        let cli = Cli::try_parse_from([
            "cliare",
            "skills",
            "install",
            "--agent",
            "claude",
            "--scope",
            "project",
            "--project-dir",
            ".",
            "--dry-run",
        ])
        .expect("valid skills install command");

        match cli.command {
            Command::Skills(args) => match args.command {
                super::SkillsCommand::Install(args) => {
                    assert_eq!(args.agent, super::SkillAgent::Claude);
                    assert_eq!(args.scope, super::SkillInstallScope::Project);
                    assert_eq!(args.project_dir, Some(std::path::PathBuf::from(".")));
                    assert!(args.dry_run);
                }
                super::SkillsCommand::List(_) => panic!("expected install command"),
            },
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected skills command")
            }
        }
    }

    #[test]
    fn issues_mark_accepts_status_and_reason() {
        let cli = Cli::try_parse_from([
            "cliare",
            "issues",
            "mark",
            "issue.alternate_help_form_unavailable",
            "--out",
            ".cliare-current",
            "--context",
            "authenticated",
            "--status",
            "intentional",
            "--reason",
            "direct help is canonical",
        ])
        .expect("valid issues mark command");

        match cli.command {
            Command::Issues(args) => match args.command {
                super::IssuesCommand::Mark(args) => {
                    assert_eq!(args.issue_id, "issue.alternate_help_form_unavailable");
                    assert_eq!(args.out, std::path::PathBuf::from(".cliare-current"));
                    assert_eq!(args.context.as_deref(), Some("authenticated"));
                    assert_eq!(
                        args.status,
                        crate::issue_disposition::IssueDispositionStatus::Intentional
                    );
                    assert_eq!(args.reason, "direct help is canonical");
                }
                super::IssuesCommand::List(_) => panic!("expected mark command"),
            },
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Playbook(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected issues command")
            }
        }
    }

    #[test]
    fn surface_query_accepts_intent_filters_and_format() {
        let cli = Cli::try_parse_from([
            "cliare",
            "surface",
            "query",
            "check job status",
            "--out",
            ".cliare-current",
            "--context",
            "clean",
            "--require-output",
            "json",
            "--limit",
            "3",
            "--format",
            "human",
        ])
        .expect("valid surface query command");

        match cli.command {
            Command::Surface(args) => match args.command {
                super::SurfaceCommand::Query(args) => {
                    assert_eq!(args.intent, "check job status");
                    assert_eq!(args.out, std::path::PathBuf::from(".cliare-current"));
                    assert_eq!(args.context.as_deref(), Some("clean"));
                    assert_eq!(
                        args.require_output,
                        Some(super::SurfaceOutputRequirement::Json)
                    );
                    assert_eq!(args.limit, 3);
                    assert_eq!(args.format, super::SurfaceFormat::Human);
                }
                super::SurfaceCommand::Explain(_) | super::SurfaceCommand::List(_) => {
                    panic!("expected query command")
                }
            },
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_) => {
                panic!("expected surface command")
            }
        }
    }

    #[test]
    fn surface_explain_accepts_command_path_and_output_request() {
        let cli = Cli::try_parse_from([
            "cliare",
            "surface",
            "explain",
            "jobs",
            "status",
            "--require-output",
            "machine-readable",
        ])
        .expect("valid surface explain command");

        match cli.command {
            Command::Surface(args) => match args.command {
                super::SurfaceCommand::Explain(args) => {
                    assert_eq!(args.command, vec!["jobs".to_owned(), "status".to_owned()]);
                    assert_eq!(
                        args.require_output,
                        Some(super::SurfaceOutputRequirement::MachineReadable)
                    );
                    assert_eq!(args.format, super::SurfaceFormat::Json);
                }
                super::SurfaceCommand::Query(_) | super::SurfaceCommand::List(_) => {
                    panic!("expected explain command")
                }
            },
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_) => {
                panic!("expected surface command")
            }
        }
    }

    #[test]
    fn surface_list_accepts_readiness_filter() {
        let cli = Cli::try_parse_from([
            "cliare", "surface", "list", "--state", "ready", "--limit", "12",
        ])
        .expect("valid surface list command");

        match cli.command {
            Command::Surface(args) => match args.command {
                super::SurfaceCommand::List(args) => {
                    assert_eq!(args.state, Some(super::SurfaceReadiness::Ready));
                    assert_eq!(args.limit, 12);
                    assert_eq!(args.format, super::SurfaceFormat::Json);
                }
                super::SurfaceCommand::Query(_) | super::SurfaceCommand::Explain(_) => {
                    panic!("expected list command")
                }
            },
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_) => {
                panic!("expected surface command")
            }
        }
    }

    #[test]
    fn playbook_maintainer_accepts_target_context_and_format() {
        let cli = Cli::try_parse_from([
            "cliare",
            "playbook",
            "maintainer",
            "--target",
            "rote",
            "--out",
            ".cliare-context",
            "--context",
            "authenticated",
            "--format",
            "json",
        ])
        .expect("valid playbook command");

        match cli.command {
            Command::Playbook(args) => {
                assert_eq!(args.role, super::PlaybookRole::Maintainer);
                assert_eq!(args.target.as_deref(), Some("rote"));
                assert_eq!(args.out, std::path::PathBuf::from(".cliare-context"));
                assert_eq!(args.context.as_deref(), Some("authenticated"));
                assert_eq!(args.format, super::PlaybookFormat::Json);
            }
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected playbook command")
            }
        }
    }

    #[test]
    fn playbook_accepts_harness_and_security_roles() {
        for (role, expected) in [
            ("harness", super::PlaybookRole::Harness),
            ("security", super::PlaybookRole::Security),
        ] {
            let cli =
                Cli::try_parse_from(["cliare", "playbook", role]).expect("valid playbook role");

            match cli.command {
                Command::Playbook(args) => {
                    assert_eq!(args.role, expected);
                }
                Command::Measure(_)
                | Command::Jobs(_)
                | Command::Guard(_)
                | Command::Benchmark(_)
                | Command::Context(_)
                | Command::Report(_)
                | Command::Describe(_)
                | Command::Skills(_)
                | Command::Issues(_)
                | Command::Metadata(_)
                | Command::Surface(_) => {
                    panic!("expected playbook command")
                }
            }
        }
    }

    #[test]
    fn playbook_help_includes_maintainer_workflow_and_profiles() {
        let mut command = Cli::command();
        let playbook = command
            .find_subcommand_mut("playbook")
            .expect("playbook command exists");
        let help = playbook.render_long_help().to_string();

        assert!(help.contains("Maintainer workflow"));
        assert!(help.contains("Available playbooks"));
        assert!(help.contains("harness"));
        assert!(help.contains("security"));
        assert!(help.contains(".cliare/<target-cli>"));
        assert!(help.contains("human"));
        assert!(help.contains("--profile quick|standard|deep"));
        assert!(help.contains("Measure profiles used by generated commands"));
        assert!(help.contains("Do not pass --profile to `cliare playbook`"));
        assert!(help.contains("quick"));
        assert!(help.contains("standard"));
        assert!(help.contains("deep"));
        assert!(help.contains("cliare report maintainer"));
        assert!(help.contains("cliare guard"));
        assert!(help.contains("cliare report harness"));
    }

    #[test]
    fn metadata_exposes_parseable_output_mode() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();

        assert!(help.contains("metadata"));

        let cli = Cli::try_parse_from(["cliare", "metadata", "--format", "json", "--help"])
            .expect("valid metadata command");

        match cli.command {
            Command::Metadata(args) => {
                assert_eq!(args.format, super::MetadataFormat::Json);
                assert!(args.help);
            }
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Surface(_) => {
                panic!("expected metadata command")
            }
            Command::Playbook(_) => {
                panic!("expected metadata command")
            }
        }
    }

    #[test]
    fn measure_accepts_detached_job_mode() {
        let cli = Cli::try_parse_from([
            "cliare",
            "measure",
            "target",
            "--out",
            ".cliare-target",
            "--detach",
        ])
        .expect("valid detached measure command");

        match cli.command {
            Command::Measure(args) => {
                assert_eq!(args.out, std::path::PathBuf::from(".cliare-target"));
                assert!(args.detach);
                assert!(!args.detached_worker);
                assert_eq!(args.job_id, None);
            }
            Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected measure command")
            }
        }
    }

    #[test]
    fn measure_accepts_host_execution_mode() {
        let cli = Cli::try_parse_from(["cliare", "measure", "target", "--execution-mode", "host"])
            .expect("valid host execution mode");

        match cli.command {
            Command::Measure(args) => {
                assert_eq!(args.execution_mode, crate::sandbox::SandboxProfile::Host);
            }
            Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected measure command")
            }
        }
    }

    #[test]
    fn jobs_status_accepts_output_directory() {
        let cli = Cli::try_parse_from([
            "cliare",
            "jobs",
            "status",
            "--out",
            ".cliare-target",
            "--context",
            "clean",
        ])
        .expect("valid jobs status command");

        match cli.command {
            Command::Jobs(args) => match args.command {
                super::JobsCommand::Status(args) => {
                    assert_eq!(args.out, std::path::PathBuf::from(".cliare-target"));
                    assert_eq!(args.context.as_deref(), Some("clean"));
                }
            },
            Command::Measure(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected jobs command")
            }
        }
    }

    #[test]
    fn describe_accepts_folder_format_and_write_options() {
        let cli = Cli::try_parse_from([
            "cliare",
            "describe",
            ".cliare-current",
            "--context",
            "clean",
            "--format",
            "json",
            "--write",
        ])
        .expect("valid describe command");

        match cli.command {
            Command::Describe(args) => {
                assert_eq!(args.folder, std::path::PathBuf::from(".cliare-current"));
                assert_eq!(args.context.as_deref(), Some("clean"));
                assert_eq!(args.format, super::DescribeFormat::Json);
                assert!(args.write);
            }
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected describe command")
            }
        }
    }

    #[test]
    fn traversal_profiles_resolve_budget_presets_and_overrides() {
        let quick = Cli::try_parse_from(["cliare", "measure", "target", "--profile", "quick"])
            .expect("valid quick profile");
        let deep = Cli::try_parse_from(["cliare", "measure", "target", "--profile", "deep"])
            .expect("valid deep profile");
        let override_depth = Cli::try_parse_from([
            "cliare",
            "measure",
            "target",
            "--profile",
            "quick",
            "--max-depth",
            "7",
            "--max-probes",
            "128",
            "--min-expected-value",
            "90",
            "--concurrency",
            "11",
        ])
        .expect("valid overrides");

        assert_budget(
            quick,
            TraversalProfile::Quick,
            QUICK_MAX_DEPTH,
            QUICK_MAX_PROBES,
            QUICK_MIN_EXPECTED_VALUE,
            QUICK_CONCURRENCY,
        );
        assert_budget(
            deep,
            TraversalProfile::Deep,
            DEEP_MAX_DEPTH,
            DEEP_MAX_PROBES,
            DEEP_MIN_EXPECTED_VALUE,
            DEEP_CONCURRENCY,
        );
        assert_budget(override_depth, TraversalProfile::Quick, 7, 128, 90, 11);
    }

    fn assert_budget(
        cli: Cli,
        profile: TraversalProfile,
        max_depth: usize,
        max_probes: usize,
        min_expected_value: u16,
        concurrency: usize,
    ) {
        match cli.command {
            Command::Measure(args) => {
                assert_eq!(args.profile, profile);
                assert_eq!(args.resolved_max_depth(), max_depth);
                assert_eq!(args.resolved_max_probes(), max_probes);
                assert_eq!(args.resolved_min_expected_value(), min_expected_value);
                assert_eq!(args.resolved_concurrency(), concurrency);
            }
            Command::Guard(_)
            | Command::Jobs(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Playbook(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected measure command")
            }
        }
    }
}
