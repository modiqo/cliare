use std::path::PathBuf;

use clap::{Args, ValueHint};

use cliare_context::{RuntimeContextProfile, RuntimeContextState};
use cliare_runtime::sandbox::SandboxProfile;

use super::{MeasureArgs, TraversalProfile, parse_positive_u64, parse_positive_usize};

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
