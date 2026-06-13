use std::path::PathBuf;
use std::time::Duration;

use clap::{Args, Parser, Subcommand, ValueHint};

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
    #[arg(long, value_name = "N", default_value_t = 2)]
    pub max_depth: usize,

    /// Maximum probes to execute for this run.
    #[arg(long, value_name = "N", default_value_t = 64)]
    pub max_probes: usize,
}

impl MeasureArgs {
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn clap_surface_exposes_measure_and_global_version() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();

        assert!(help.contains("Usage: cliare"));
        assert!(help.contains("measure"));
        assert!(help.contains("--version"));
    }
}
