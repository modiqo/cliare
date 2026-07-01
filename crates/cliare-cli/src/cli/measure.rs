use std::path::PathBuf;
use std::time::Duration;

use clap::{Args, ValueHint};

use cliare_context::{RuntimeContextProfile, RuntimeContextState};
use cliare_runtime::sandbox::{
    DEFAULT_SNAPSHOT_MAX_DIRS, DEFAULT_SNAPSHOT_MAX_FILES, DEFAULT_SNAPSHOT_MAX_HASH_BYTES,
    SandboxProfile, SnapshotLimits,
};

use super::{TraversalProfile, parse_positive_u64, parse_positive_usize};

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
