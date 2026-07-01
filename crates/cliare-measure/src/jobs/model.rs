use std::path::PathBuf;

#[derive(Debug)]
pub struct DetachedMeasureSummary {
    pub job_id: String,
    pub pid: u32,
    pub progress_log: PathBuf,
    pub stdout_log: PathBuf,
    pub stderr_log: PathBuf,
    pub status_command: String,
    pub watch_command: String,
}

impl DetachedMeasureSummary {
    pub fn terminal_summary(&self) -> String {
        format!(
            "CLIARE measure job started\n\
             job_id: {}\n\
             pid: {}\n\
             progress log: {}\n\
             stdout log: {}\n\
             stderr log: {}\n\
             status: {}\n\
             watch: {}\n",
            self.job_id,
            self.pid,
            self.progress_log.display(),
            self.stdout_log.display(),
            self.stderr_log.display(),
            self.status_command,
            self.watch_command
        )
    }
}

#[derive(Debug)]
pub struct JobsSummary {
    pub out: PathBuf,
    pub job_id: Option<String>,
    pub status: JobStatus,
    pub progress_log: Option<PathBuf>,
    pub stdout_log: Option<PathBuf>,
    pub stderr_log: Option<PathBuf>,
    pub last_progress: Option<String>,
    pub last_error: Option<String>,
}

impl JobsSummary {
    pub fn terminal_summary(&self) -> String {
        let mut lines = vec![
            "CLIARE job status".to_owned(),
            format!("out: {}", self.out.display()),
            format!("status: {}", self.status.label()),
        ];
        if let Some(job_id) = &self.job_id {
            lines.push(format!("job_id: {job_id}"));
        }
        if let Some(path) = &self.progress_log {
            lines.push(format!("progress log: {}", path.display()));
            lines.push(format!("watch: tail -f {}", path.display()));
        }
        if let Some(path) = &self.stdout_log {
            lines.push(format!("stdout log: {}", path.display()));
        }
        if let Some(path) = &self.stderr_log {
            lines.push(format!("stderr log: {}", path.display()));
        }
        if let Some(last_progress) = &self.last_progress {
            lines.push(format!("last progress: {last_progress}"));
        }
        if let Some(last_error) = &self.last_error {
            lines.push(format!("last error: {last_error}"));
        }
        format!("{}\n", lines.join("\n"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    NotStarted,
    Starting,
    Running,
    Complete,
    Failed,
}

impl JobStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Complete => "complete",
            Self::Failed => "failed",
        }
    }

    pub(super) fn is_active(self) -> bool {
        matches!(self, Self::Starting | Self::Running)
    }
}
