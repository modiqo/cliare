use std::path::PathBuf;

use clap::{Args, ValueHint};

use super::parse_positive_usize;

#[derive(Debug, Args)]
pub struct BenchmarkArgs {
    /// Benchmark corpus manifest.
    #[arg(
        long,
        value_name = "FILE",
        default_value = "benchmarks/local-corpus.json",
        value_hint = ValueHint::FilePath
    )]
    pub manifest: PathBuf,

    /// Output directory for benchmark artifacts.
    #[arg(
        long,
        value_name = "DIR",
        default_value = ".cliare-bench",
        value_hint = ValueHint::DirPath
    )]
    pub out: PathBuf,

    /// Maximum benchmark targets to measure concurrently.
    #[arg(long, value_name = "N", value_parser = parse_positive_usize)]
    pub target_concurrency: Option<usize>,

    /// Ignore reusable measurement artifacts and run probes again.
    #[arg(long)]
    pub refresh: bool,
}
