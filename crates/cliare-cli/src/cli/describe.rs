use std::path::PathBuf;

use clap::{Args, ValueEnum, ValueHint};
use serde::{Deserialize, Serialize};

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
