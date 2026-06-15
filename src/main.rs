use clap::Parser;
use miette::IntoDiagnostic;

use cliare::cli::{Cli, Command, MetadataArgs, MetadataFormat};

#[tokio::main]
async fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Measure(args) => {
            if args.detach && !args.detached_worker {
                let summary = cliare::jobs::spawn_detached_measure(args).into_diagnostic()?;
                print!("{}", summary.terminal_summary());
                return Ok(());
            }
            let summary = cliare::measure::measure(args).await.into_diagnostic()?;
            print!("{}", summary.terminal_summary());
            Ok(())
        }
        Command::Jobs(args) => {
            let summary = cliare::jobs::jobs(args).await.into_diagnostic()?;
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
        Command::Report(args) => {
            let summary = cliare::report::report(args).await.into_diagnostic()?;
            print!("{}", summary.terminal_summary());
            Ok(())
        }
        Command::Describe(args) => {
            let summary = cliare::describe::describe(args).await.into_diagnostic()?;
            print!("{}", summary.terminal_summary());
            Ok(())
        }
        Command::Skills(args) => {
            let summary = cliare::skills::skills(args).await.into_diagnostic()?;
            print!("{}", summary.terminal_summary());
            Ok(())
        }
        Command::Issues(args) => {
            let summary = cliare::issues::issues(args).await.into_diagnostic()?;
            print!("{}", summary.terminal_summary());
            Ok(())
        }
        Command::Playbook(args) => {
            let summary = cliare::playbook::playbook(args).into_diagnostic()?;
            print!("{}", summary.terminal_summary());
            Ok(())
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
        Command::Context(args) => {
            let summary = cliare::context::context(args).await.into_diagnostic()?;
            print!("{}", summary.terminal_summary());
            Ok(())
        }
        Command::Metadata(args) => print_metadata(args),
    }
}

fn print_metadata(args: MetadataArgs) -> miette::Result<()> {
    match args.format {
        MetadataFormat::Json => {
            let value = cliare::command_spec::metadata();
            println!(
                "{}",
                serde_json::to_string_pretty(&value).into_diagnostic()?
            );
        }
        MetadataFormat::Text => {
            if args.help {
                print!("{}", cliare::command_spec::metadata_help());
            } else {
                print!("{}", cliare::command_spec::metadata_text());
            }
        }
    }
    Ok(())
}
