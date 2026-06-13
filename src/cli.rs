use std::path::PathBuf;
use std::time::Duration;

use clap::{Args, Parser, Subcommand, ValueHint};

pub const DEFAULT_MAX_DEPTH: usize = 5;
pub const DEFAULT_MAX_PROBES: usize = 256;

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

    /// Maximum command-path depth to recursively confirm.
    #[arg(long, value_name = "N", default_value_t = DEFAULT_MAX_DEPTH)]
    pub max_depth: usize,

    /// Maximum probes to execute for this run.
    #[arg(long, value_name = "N", default_value_t = DEFAULT_MAX_PROBES)]
    pub max_probes: usize,
}

impl MeasureArgs {
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
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

    /// Maximum command-path depth to recursively confirm.
    #[arg(long, value_name = "N", default_value_t = DEFAULT_MAX_DEPTH)]
    pub max_depth: usize,

    /// Maximum probes to execute for this run.
    #[arg(long, value_name = "N", default_value_t = DEFAULT_MAX_PROBES)]
    pub max_probes: usize,
}

impl From<&GuardArgs> for MeasureArgs {
    fn from(args: &GuardArgs) -> Self {
        Self {
            target: args.target.clone(),
            out: args.out.clone(),
            timeout_ms: args.timeout_ms,
            output_limit_bytes: args.output_limit_bytes,
            max_depth: args.max_depth,
            max_probes: args.max_probes,
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{Cli, Command, DEFAULT_MAX_DEPTH, DEFAULT_MAX_PROBES};

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
                assert_eq!(args.max_depth, DEFAULT_MAX_DEPTH);
                assert_eq!(args.max_probes, DEFAULT_MAX_PROBES);
            }
            Command::Guard(_) => panic!("expected measure command"),
        }

        match guard.command {
            Command::Guard(args) => {
                assert_eq!(args.max_depth, DEFAULT_MAX_DEPTH);
                assert_eq!(args.max_probes, DEFAULT_MAX_PROBES);
            }
            Command::Measure(_) => panic!("expected guard command"),
        }
    }
}
