use clap::Parser;
use miette::IntoDiagnostic;

use cliare::cli::{Cli, Command};

#[tokio::main]
async fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Measure(args) => cliare::measure::measure(args).await.into_diagnostic(),
    }
}
