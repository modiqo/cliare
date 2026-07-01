use std::path::PathBuf;

use clap::{Args, Subcommand, ValueHint};

#[derive(Debug, Args)]
pub struct JobsArgs {
    #[command(subcommand)]
    pub command: JobsCommand,
}

#[derive(Debug, Subcommand)]
pub enum JobsCommand {
    /// Print the latest detached or foreground measurement progress state.
    Status(JobsStatusArgs),
}

#[derive(Debug, Args)]
pub struct JobsStatusArgs {
    /// Measurement artifact directory containing jobs/current.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Context name to select when --out points at a context suite root.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,
}
