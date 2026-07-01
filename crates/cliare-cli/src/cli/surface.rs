use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum, ValueHint};
use serde::{Deserialize, Serialize};

use super::parse_positive_usize;

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
