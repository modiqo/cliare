use std::path::PathBuf;

use clap::{Args, ValueEnum, ValueHint};
use serde::{Deserialize, Serialize};

use super::parse_positive_usize;

#[derive(Debug, Args)]
#[command(
    after_help = "Use after `cliare measure`. Produces one concise interpretation from scorecard.json, issues.json, and command-index.json.\n\nEach finding reports its assessment, meaning for agent/harness use, associated commands, and suggested remedy."
)]
pub struct SummaryArgs {
    /// Measurement artifact directory containing scorecard.json, command-index.json, and issues.json.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Context name to select when --out points at a context suite root.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,

    /// Representation to print to stdout.
    #[arg(long, value_enum, default_value_t = SummaryFormat::Markdown)]
    pub format: SummaryFormat,

    /// Maximum number of findings to include.
    #[arg(long, value_name = "N", default_value_t = 6, value_parser = parse_positive_usize)]
    pub max_findings: usize,

    /// Maximum command examples per finding.
    #[arg(long, value_name = "N", default_value_t = 5, value_parser = parse_positive_usize)]
    pub max_examples: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum SummaryFormat {
    Markdown,
    Json,
}

impl SummaryFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Json => "json",
        }
    }
}
