use std::path::{Path, PathBuf};

use cliare_cli::cli::{
    IssuesArgs, IssuesCommand, IssuesListArgs, IssuesListFormat, IssuesMarkArgs,
};
use cliare_context as context;
use cliare_core::error::{CliareError, Result};
use cliare_issues::issue_disposition::{IssueDispositions, disposition_path};

mod model;
mod packet;
mod render;

#[cfg(test)]
mod tests;

use model::IssueDispositionList;
use render::{render_list_human, render_list_markdown, render_mark_summary};

const ISSUES_LIST_SCHEMA_VERSION: &str = "cliare.issue-list.v2";
const ISSUE_COMMAND_SAMPLE_LIMIT: usize = 5;

#[derive(Debug, Clone)]
pub struct IssuesSummary {
    artifact_dir: PathBuf,
    dispositions_path: PathBuf,
    stdout: String,
}

impl IssuesSummary {
    pub fn terminal_summary(&self) -> &str {
        &self.stdout
    }

    pub fn artifact_dir(&self) -> &Path {
        &self.artifact_dir
    }

    pub fn dispositions_path(&self) -> &Path {
        &self.dispositions_path
    }
}

pub async fn issues(args: IssuesArgs) -> Result<IssuesSummary> {
    match args.command {
        IssuesCommand::Mark(args) => mark(args).await,
        IssuesCommand::List(args) => list(args).await,
    }
}

async fn mark(args: IssuesMarkArgs) -> Result<IssuesSummary> {
    let artifact_dir =
        context::resolve_measurement_dir(&args.out, args.context.as_deref(), "cliare issues mark")
            .await?;
    let issue_id = args.issue_id;
    let status = args.status;
    let mut dispositions = IssueDispositions::read_optional(&artifact_dir).await?;
    dispositions.mark(issue_id.clone(), status, args.reason);
    let dispositions_path = dispositions.write(&artifact_dir).await?;
    let stdout = render_mark_summary(&artifact_dir, &dispositions_path, &issue_id, status);

    Ok(IssuesSummary {
        artifact_dir,
        dispositions_path,
        stdout,
    })
}

async fn list(args: IssuesListArgs) -> Result<IssuesSummary> {
    let artifact_dir =
        context::resolve_measurement_dir(&args.out, args.context.as_deref(), "cliare issues list")
            .await?;
    let dispositions = IssueDispositions::read_optional(&artifact_dir).await?;
    let packet = IssueDispositionList::build(&artifact_dir, &dispositions).await?;
    let dispositions_path = disposition_path(&artifact_dir);
    let stdout = match args.format {
        IssuesListFormat::Human => render_list_human(&packet),
        IssuesListFormat::Markdown => render_list_markdown(&packet),
        IssuesListFormat::Json => {
            format!(
                "{}
",
                serde_json::to_string_pretty(&packet)
                    .map_err(CliareError::SerializeIssueDispositions)?
            )
        }
    };

    Ok(IssuesSummary {
        artifact_dir,
        dispositions_path,
        stdout,
    })
}
