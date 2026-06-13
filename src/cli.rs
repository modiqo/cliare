use std::path::PathBuf;
use std::time::Duration;

use clap::{Args, Parser, Subcommand, ValueEnum, ValueHint};
use serde::{Deserialize, Serialize};

pub const QUICK_MAX_DEPTH: usize = 3;
pub const QUICK_MAX_PROBES: usize = 64;
pub const STANDARD_MAX_DEPTH: usize = 5;
pub const STANDARD_MAX_PROBES: usize = 256;
pub const DEEP_MAX_DEPTH: usize = 8;
pub const DEEP_MAX_PROBES: usize = 1_000;

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
    /// Measure a target and fail when score regresses against a baseline.
    Guard(GuardArgs),
}

#[derive(Debug, Args)]
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

    /// Ignore reusable artifacts and run probes again.
    #[arg(long)]
    pub refresh: bool,
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
            refresh: args.refresh,
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

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{
        Cli, Command, DEEP_MAX_DEPTH, DEEP_MAX_PROBES, QUICK_MAX_DEPTH, QUICK_MAX_PROBES,
        STANDARD_MAX_DEPTH, STANDARD_MAX_PROBES, TraversalProfile,
    };

    #[test]
    fn clap_surface_exposes_measure_and_global_version() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();

        assert!(help.contains("Usage: cliare"));
        assert!(help.contains("measure"));
        assert!(help.contains("guard"));
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
            }
            Command::Guard(_) => panic!("expected measure command"),
        }

        match guard.command {
            Command::Guard(args) => {
                let measure_args = super::MeasureArgs::from(&args);
                assert_eq!(args.profile, TraversalProfile::Standard);
                assert_eq!(measure_args.resolved_max_depth(), STANDARD_MAX_DEPTH);
                assert_eq!(measure_args.resolved_max_probes(), STANDARD_MAX_PROBES);
            }
            Command::Measure(_) => panic!("expected guard command"),
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
        ])
        .expect("valid overrides");

        assert_budget(
            quick,
            TraversalProfile::Quick,
            QUICK_MAX_DEPTH,
            QUICK_MAX_PROBES,
        );
        assert_budget(
            deep,
            TraversalProfile::Deep,
            DEEP_MAX_DEPTH,
            DEEP_MAX_PROBES,
        );
        assert_budget(override_depth, TraversalProfile::Quick, 7, 128);
    }

    fn assert_budget(cli: Cli, profile: TraversalProfile, max_depth: usize, max_probes: usize) {
        match cli.command {
            Command::Measure(args) => {
                assert_eq!(args.profile, profile);
                assert_eq!(args.resolved_max_depth(), max_depth);
                assert_eq!(args.resolved_max_probes(), max_probes);
            }
            Command::Guard(_) => panic!("expected measure command"),
        }
    }
}
