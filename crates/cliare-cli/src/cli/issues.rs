use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum, ValueHint};

use cliare_issues::issue_disposition::IssueDispositionStatus;

#[derive(Debug, Args)]
#[command(
    after_help = "For the end-to-end maintainer workflow and parameter guide, run: cliare playbook maintainer"
)]
pub struct IssuesArgs {
    #[command(subcommand)]
    pub command: IssuesCommand,
}

#[derive(Debug, Subcommand)]
pub enum IssuesCommand {
    /// Record a maintainer disposition for an issue id.
    Mark(IssuesMarkArgs),
    /// List generated issues with maintainer dispositions.
    List(IssuesListArgs),
}

#[derive(Debug, Args)]
pub struct IssuesMarkArgs {
    /// Issue id to mark, such as issue.output_mode_unprobed.
    pub issue_id: String,

    /// Measurement artifact directory containing issues.json or issue-dispositions.json.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Context name to select when --out points at a context suite root.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,

    /// Maintainer disposition to record.
    #[arg(long, value_enum)]
    pub status: IssueDispositionStatus,

    /// Maintainer rationale for the disposition.
    #[arg(long)]
    pub reason: String,
}

#[derive(Debug, Args)]
pub struct IssuesListArgs {
    /// Measurement artifact directory containing issues.json or issue-dispositions.json.
    #[arg(long, value_name = "DIR", default_value = ".cliare", value_hint = ValueHint::DirPath)]
    pub out: PathBuf,

    /// Context name to select when --out points at a context suite root.
    #[arg(long, value_name = "NAME")]
    pub context: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = IssuesListFormat::Markdown)]
    pub format: IssuesListFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum IssuesListFormat {
    Human,
    Markdown,
    Json,
}
