use std::path::PathBuf;
use std::time::Duration;

use clap::{Args, Parser, Subcommand, ValueEnum, ValueHint};
use serde::{Deserialize, Serialize};

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
    /// Generate a persona-specific outcome packet from measurement artifacts.
    Report(ReportArgs),
    /// Describe a CLIARE artifact directory for humans and agents.
    Describe(DescribeArgs),
    /// Install CLIARE artifact-review skills for coding agents.
    Skills(SkillsArgs),
    /// Print CLIARE implementation metadata.
    Metadata(MetadataArgs),
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
pub struct ReportArgs {
    /// Persona packet to generate.
    #[arg(value_enum)]
    pub persona: ReportPersona,

    /// Measurement artifact directory containing scorecard.json, command-index.json, shape.json, and evidence.jsonl.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Representation to print to stdout.
    #[arg(long, value_enum, default_value_t = ReportFormat::Markdown)]
    pub format: ReportFormat,

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
}

impl ReportFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Json => "json",
        }
    }
}

#[derive(Debug, Args)]
pub struct DescribeArgs {
    /// CLIARE measurement or benchmark artifact directory.
    #[arg(value_name = "FOLDER", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub folder: PathBuf,

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
}

#[derive(Debug, Clone, Args)]
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
            max_depth: args.max_depth,
            max_probes: args.max_probes,
            min_expected_value: args.min_expected_value,
            concurrency: args.concurrency,
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

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

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
        assert!(help.contains("metadata"));
        assert!(help.contains("--version"));
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
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Metadata(_) => {
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
            | Command::Metadata(_) => {
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
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Metadata(_) => {
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
            "--format",
            "json",
            "--write",
        ])
        .expect("valid report command");

        match cli.command {
            Command::Report(args) => {
                assert_eq!(args.persona, super::ReportPersona::Security);
                assert_eq!(args.out, std::path::PathBuf::from(".cliare-current"));
                assert_eq!(args.format, super::ReportFormat::Json);
                assert!(args.write);
            }
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Metadata(_) => {
                panic!("expected report command")
            }
        }
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
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Metadata(_) => {
                panic!("expected skills command")
            }
        }
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
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_) => {
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
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Metadata(_) => {
                panic!("expected measure command")
            }
        }
    }

    #[test]
    fn jobs_status_accepts_output_directory() {
        let cli = Cli::try_parse_from(["cliare", "jobs", "status", "--out", ".cliare-target"])
            .expect("valid jobs status command");

        match cli.command {
            Command::Jobs(args) => match args.command {
                super::JobsCommand::Status(args) => {
                    assert_eq!(args.out, std::path::PathBuf::from(".cliare-target"));
                }
            },
            Command::Measure(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Metadata(_) => {
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
            "--format",
            "json",
            "--write",
        ])
        .expect("valid describe command");

        match cli.command {
            Command::Describe(args) => {
                assert_eq!(args.folder, std::path::PathBuf::from(".cliare-current"));
                assert_eq!(args.format, super::DescribeFormat::Json);
                assert!(args.write);
            }
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Report(_)
            | Command::Skills(_)
            | Command::Metadata(_) => {
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
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Metadata(_) => {
                panic!("expected measure command")
            }
        }
    }
}
