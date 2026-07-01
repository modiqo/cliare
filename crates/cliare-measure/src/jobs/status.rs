use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::{JobsArgs, JobsCommand};
use crate::context;
use crate::error::{CliareError, Result};

use super::model::{JobStatus, JobsSummary};

pub async fn jobs(args: JobsArgs) -> Result<JobsSummary> {
    match args.command {
        JobsCommand::Status(args) => {
            let artifact_dir = job_artifact_dir(&args.out, args.context.as_deref());
            job_status(artifact_dir)
        }
    }
}

pub(super) fn job_artifact_dir(root: &Path, context: Option<&str>) -> PathBuf {
    context.map_or_else(
        || root.to_path_buf(),
        |context| context::context_artifact_dir(root, context),
    )
}

pub(super) fn job_status(out: PathBuf) -> Result<JobsSummary> {
    let current_path = out.join("jobs/current");
    let current = match fs::read_to_string(&current_path) {
        Ok(current) => current,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(JobsSummary {
                out,
                job_id: None,
                status: JobStatus::NotStarted,
                progress_log: None,
                stdout_log: None,
                stderr_log: None,
                last_progress: None,
                last_error: None,
            });
        }
        Err(source) => {
            return Err(CliareError::ReadJobPointer {
                path: current_path,
                source,
            });
        }
    };
    let pointer = parse_pointer(&current);
    let progress_log = pointer.get("progress_log").map(PathBuf::from);
    let stdout_log = pointer.get("stdout_log").map(PathBuf::from);
    let stderr_log = pointer.get("stderr_log").map(PathBuf::from);
    let last_progress = match &progress_log {
        Some(path) => last_progress_line(path)?,
        None => None,
    };
    let last_error = match &stderr_log {
        Some(path) => last_stream_line(path)?,
        None => None,
    };
    let status = classify_job_status(last_progress.as_deref(), last_error.as_deref());

    Ok(JobsSummary {
        out,
        job_id: pointer.get("job_id").cloned(),
        status,
        progress_log,
        stdout_log,
        stderr_log,
        last_progress,
        last_error,
    })
}

pub(super) fn classify_job_status(
    last_progress: Option<&str>,
    last_error: Option<&str>,
) -> JobStatus {
    match last_progress {
        Some(line) if line.contains("failed error=") => JobStatus::Failed,
        Some(line) if line.contains("100.0% complete ") => JobStatus::Complete,
        Some(line) if line.contains("job_spawned") && last_error.is_some() => JobStatus::Failed,
        Some(_) => JobStatus::Running,
        None if last_error.is_some() => JobStatus::Failed,
        None => JobStatus::Starting,
    }
}

pub(super) fn parse_pointer(contents: &str) -> BTreeMap<String, String> {
    contents
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_owned(), value.trim().to_owned()))
        })
        .collect()
}

fn last_progress_line(path: &Path) -> Result<Option<String>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(CliareError::ReadJobProgressLog {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    Ok(text
        .lines()
        .rev()
        .find(|line| line.starts_with('['))
        .map(str::to_owned))
}

pub(super) fn last_stream_line(path: &Path) -> Result<Option<String>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(CliareError::ReadJobStream {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    Ok(text
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned))
}
