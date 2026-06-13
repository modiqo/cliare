use clap::Parser;
use miette::IntoDiagnostic;

use cliare::cli::{Cli, Command, MetadataArgs, MetadataFormat};

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
        Command::Metadata(args) => print_metadata(args),
    }
}

fn print_metadata(args: MetadataArgs) -> miette::Result<()> {
    match args.format {
        MetadataFormat::Json => {
            let value = serde_json::json!({
                "schema_version": "cliare.metadata.v1",
                "name": "cliare",
                "version": env!("CARGO_PKG_VERSION"),
                "formats": ["text", "json"],
                "commands": ["measure", "guard", "benchmark", "metadata"],
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&value).into_diagnostic()?
            );
        }
        MetadataFormat::Text => {
            if args.help {
                print!("{}", metadata_help());
            } else {
                println!("cliare {}", env!("CARGO_PKG_VERSION"));
            }
        }
    }
    Ok(())
}

fn metadata_help() -> &'static str {
    "Print CLIARE implementation metadata\n\nUsage: cliare metadata [OPTIONS]\n\nOptions:\n      --format <FORMAT>  Output format [default: text] [possible values: text, json]\n      --help             Print help. With --format json, emit a parseable metadata contract\n"
}
