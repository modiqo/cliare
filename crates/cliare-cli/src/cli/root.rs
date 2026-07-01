use clap::{Parser, Subcommand};

use cliare_context::ContextArgs;

use super::{
    BenchmarkArgs, DescribeArgs, GuardArgs, IssuesArgs, JobsArgs, MeasureArgs, MetadataArgs,
    PlaybookArgs, ReportArgs, SkillsArgs, SurfaceArgs,
};

#[derive(Debug, Parser)]
#[command(name = "cliare")]
#[command(version)]
#[command(about = "Measure CLI agent readiness from runtime evidence")]
#[command(
    long_about = "CLIARE measures command-line interfaces by probing runtime behavior, recording evidence, and producing agent-readiness artifacts."
)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run safe bootstrap probes and write an evidence log.
    Measure(MeasureArgs),
    /// Inspect detached CLIARE jobs.
    Jobs(JobsArgs),
    /// Measure a target and fail when score regresses against a baseline.
    Guard(GuardArgs),
    /// Run a benchmark corpus and produce calibration reports.
    Benchmark(BenchmarkArgs),
    /// Compare measurements across runtime contexts.
    Context(ContextArgs),
    /// Generate a persona-specific outcome packet from measurement artifacts.
    Report(ReportArgs),
    /// Describe a CLIARE artifact directory for humans and agents.
    Describe(DescribeArgs),
    /// Install CLIARE artifact-review skills for coding agents.
    Skills(SkillsArgs),
    /// Review, mark, and list evidence-backed CLIARE issues.
    Issues(IssuesArgs),
    /// Query the measured command surface for harness routing.
    Surface(SurfaceArgs),
    /// Print role-specific operational playbooks.
    Playbook(PlaybookArgs),
    /// Print CLIARE implementation metadata.
    Metadata(MetadataArgs),
}
