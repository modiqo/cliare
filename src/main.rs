use clap::Parser;
use miette::IntoDiagnostic;

use cliare::cli::{Cli, Command};

#[tokio::main]
async fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Measure(args) => {
            let summary = cliare::measure::measure(args).await.into_diagnostic()?;
            print!("{}", summary.terminal_summary());
            Ok(())
        }
        Command::Benchmark(args) => {
            let summary = cliare::benchmark::benchmark(args).await.into_diagnostic()?;
            print!("{}", summary.terminal_summary());
            if summary.passed {
                Ok(())
            } else {
                Err(miette::miette!("benchmark failed calibration checks"))
            }
        }
        Command::Guard(args) => {
            let summary = cliare::guard::guard(args).await.into_diagnostic()?;
            print!("{}", summary.terminal_summary());
            if summary.passed {
                Ok(())
            } else if !summary.regression_passed {
                Err(miette::miette!(
                    "guard failed: score changed by {:+.1}, allowed drop is {:.1}",
                    summary.delta,
                    summary.allowed_drop
                ))
            } else {
                Err(miette::miette!("guard failed: policy checks failed"))
            }
        }
    }
}
