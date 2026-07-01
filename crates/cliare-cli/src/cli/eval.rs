use std::path::PathBuf;

use clap::{Args, Subcommand, ValueHint};

#[derive(Debug, Args)]
pub struct EvalArgs {
    #[command(subcommand)]
    pub command: EvalCommand,
}

#[derive(Debug, Subcommand)]
pub enum EvalCommand {
    /// Compare shape artifacts against fixture truth labels.
    ShapeQuality(ShapeQualityArgs),
}

#[derive(Debug, Args)]
pub struct ShapeQualityArgs {
    /// Inferred CLIARE shape artifact to evaluate.
    #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub shape: PathBuf,

    /// Fixture truth labels for the target CLI shape.
    #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub truth: PathBuf,

    /// Output directory for shape-quality artifacts.
    #[arg(long, value_name = "DIR", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,
}
