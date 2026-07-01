use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum, ValueHint};

#[derive(Debug, Args)]
pub struct ContextArgs {
    #[command(subcommand)]
    pub command: ContextCommand,
}

#[derive(Debug, Subcommand)]
pub enum ContextCommand {
    /// Compare multiple context measurement artifact directories.
    Compare(ContextCompareArgs),
}

#[derive(Debug, Args)]
pub struct ContextCompareArgs {
    /// Context measurement directories to compare.
    #[arg(value_name = "CONTEXT_DIR", value_hint = ValueHint::DirPath, required = true)]
    pub context_dirs: Vec<PathBuf>,

    /// Output directory for context-suite.json and context-compare.md.
    #[arg(
        long,
        value_name = "DIR",
        default_value = ".cliare-context",
        value_hint = ValueHint::DirPath
    )]
    pub out: PathBuf,

    /// Representation to print to stdout.
    #[arg(long, value_enum, default_value_t = ContextCompareFormat::Markdown)]
    pub format: ContextCompareFormat,

    /// Write context-suite.json and context-compare.md to --out.
    #[arg(long)]
    pub write: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ContextCompareFormat {
    Markdown,
    Json,
}
