use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::cli::{JobsArgs, JobsCommand, MeasureArgs};
use crate::context::{self, RuntimeContext, RuntimeContextInput};
use crate::error::{CliareError, Result};
use crate::fingerprint;

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

    fn is_active(self) -> bool {
        matches!(self, Self::Starting | Self::Running)
    }
}

pub fn spawn_detached_measure(args: MeasureArgs) -> Result<DetachedMeasureSummary> {
    let runtime_context = RuntimeContext::from_input(RuntimeContextInput {
        profile: args.context,
        name: args.context_name.clone(),
        auth_state: args.auth_state,
        local_context_state: args.local_context_state,
        fixture_state: args.fixture_state,
        network_state: args.network_state,
        runtime_dependency_state: args.runtime_dependency_state,
        workdir: args.context_workdir.clone(),
    });
    let artifact_dir = context::measurement_dir(&args.out, &runtime_context);
    let _resolved_target = fingerprint::preflight_target(&args.target)?;
    let jobs_dir = artifact_dir.join("jobs");
    fs::create_dir_all(&jobs_dir).map_err(|source| CliareError::CreateProgressDir {
        path: jobs_dir.clone(),
        source,
    })?;
    ensure_no_active_job(&artifact_dir)?;

    let job_id = crate::measure::new_measure_job_id()?;
    let progress_log = jobs_dir.join(format!("{job_id}.log"));
    let stdout_log = jobs_dir.join(format!("{job_id}.stdout.log"));
    let stderr_log = jobs_dir.join(format!("{job_id}.stderr.log"));
    create_initial_progress_log(&progress_log, &job_id, &artifact_dir)?;
    let stdout = open_job_stream(&stdout_log)?;
    let stderr = open_job_stream(&stderr_log)?;

    let executable = std::env::current_exe().map_err(CliareError::CurrentExecutable)?;
    let mut command = Command::new(executable);
    command
        .args(measure_worker_args(&args, &job_id))
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    configure_detached_process(&mut command);

    let child = command.spawn().map_err(CliareError::SpawnDetachedMeasure)?;
    write_current_pointer(
        &jobs_dir,
        &job_id,
        child.id(),
        &progress_log,
        &stdout_log,
        &stderr_log,
        &artifact_dir,
    )?;

    Ok(DetachedMeasureSummary {
        job_id,
        pid: child.id(),
        progress_log: progress_log.clone(),
        stdout_log,
        stderr_log,
        status_command: format!("cliare jobs status --out {}", shell_arg_path(&artifact_dir)),
        watch_command: format!("tail -f {}", shell_arg_path(&progress_log)),
    })
}

fn configure_detached_process(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;

        command.process_group(0);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;

        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
    }
}

pub async fn jobs(args: JobsArgs) -> Result<JobsSummary> {
    match args.command {
        JobsCommand::Status(args) => {
            let artifact_dir = job_artifact_dir(&args.out, args.context.as_deref());
            job_status(artifact_dir)
        }
    }
}

fn job_artifact_dir(root: &Path, context: Option<&str>) -> PathBuf {
    context.map_or_else(
        || root.to_path_buf(),
        |context| context::context_artifact_dir(root, context),
    )
}

fn open_job_stream(path: &Path) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|source| CliareError::OpenDetachedJobStream {
            path: path.to_path_buf(),
            source,
        })
}

fn ensure_no_active_job(out: &Path) -> Result<()> {
    let summary = job_status(out.to_path_buf())?;
    if !summary.status.is_active() {
        return Ok(());
    }

    let job_id = summary
        .job_id
        .as_deref()
        .map_or_else(String::new, |job_id| format!(" job_id={job_id}"));
    let status_command = format!("cliare jobs status --out {}", shell_arg_path(out));
    Err(CliareError::DetachedJobAlreadyActive {
        message: format!(
            "detached measure job already active for {}: status={}{}. Run `{}` or wait for it to finish before starting another detached measurement.",
            out.display(),
            summary.status.label(),
            job_id,
            status_command
        ),
    })
}

fn create_initial_progress_log(path: &Path, job_id: &str, out: &Path) -> Result<()> {
    let contents = format!(
        "# CLIARE measure progress\n\
         job_id: {job_id}\n\
         out: {}\n\
         progress: initializing detached measurement; worker will replace this header once target resolution succeeds.\n\n\
         [detached]   0.0% job_spawned\n",
        out.display()
    );
    fs::write(path, contents).map_err(|source| CliareError::WriteProgressLog {
        path: path.to_path_buf(),
        source,
    })
}

fn measure_worker_args(args: &MeasureArgs, job_id: &str) -> Vec<String> {
    let mut values = vec![
        "measure".to_owned(),
        args.target.display().to_string(),
        "--out".to_owned(),
        args.out.display().to_string(),
        "--timeout-ms".to_owned(),
        args.timeout_ms.to_string(),
        "--output-limit-bytes".to_owned(),
        args.output_limit_bytes.to_string(),
        "--profile".to_owned(),
        args.profile.label().to_owned(),
        "--execution-mode".to_owned(),
        args.execution_mode.label().to_owned(),
    ];
    push_optional(&mut values, "--max-depth", args.max_depth);
    push_optional(&mut values, "--max-probes", args.max_probes);
    push_optional(&mut values, "--min-expected-value", args.min_expected_value);
    push_optional(&mut values, "--concurrency", args.concurrency);
    push_optional_context_profile(&mut values, "--context", args.context);
    if let Some(name) = &args.context_name {
        values.push("--context-name".to_owned());
        values.push(name.to_owned());
    }
    push_optional_context_state(&mut values, "--auth-state", args.auth_state);
    push_optional_context_state(
        &mut values,
        "--local-context-state",
        args.local_context_state,
    );
    push_optional_context_state(&mut values, "--fixture-state", args.fixture_state);
    push_optional_context_state(&mut values, "--network-state", args.network_state);
    push_optional_context_state(
        &mut values,
        "--runtime-dependency-state",
        args.runtime_dependency_state,
    );
    if let Some(workdir) = &args.context_workdir {
        values.push("--context-workdir".to_owned());
        values.push(workdir.display().to_string());
    }
    if args.refresh {
        values.push("--refresh".to_owned());
    }
    values.push("--__cliare-detached-worker".to_owned());
    values.push("--__cliare-job-id".to_owned());
    values.push(job_id.to_owned());
    values
}

fn push_optional_context_profile(
    values: &mut Vec<String>,
    flag: &str,
    value: Option<crate::context::RuntimeContextProfile>,
) {
    if let Some(value) = value {
        values.push(flag.to_owned());
        values.push(value.cli_value().to_owned());
    }
}

fn push_optional_context_state(
    values: &mut Vec<String>,
    flag: &str,
    value: Option<crate::context::RuntimeContextState>,
) {
    if let Some(value) = value {
        values.push(flag.to_owned());
        values.push(value.cli_value().to_owned());
    }
}

fn push_optional<T>(values: &mut Vec<String>, flag: &str, value: Option<T>)
where
    T: ToString,
{
    if let Some(value) = value {
        values.push(flag.to_owned());
        values.push(value.to_string());
    }
}

fn write_current_pointer(
    jobs_dir: &Path,
    job_id: &str,
    pid: u32,
    progress_log: &Path,
    stdout_log: &Path,
    stderr_log: &Path,
    out_dir: &Path,
) -> Result<()> {
    let path = jobs_dir.join("current");
    let contents = format!(
        "job_id={job_id}\n\
         pid={pid}\n\
         progress_log={}\n\
         stdout_log={}\n\
         stderr_log={}\n\
         status=spawned\n\
         status_command=cliare jobs status --out {}\n\
         tail=tail -f {}\n",
        progress_log.display(),
        stdout_log.display(),
        stderr_log.display(),
        shell_arg_path(out_dir),
        shell_arg_path(progress_log)
    );
    fs::write(&path, contents).map_err(|source| CliareError::WriteProgressLog { path, source })
}

fn job_status(out: PathBuf) -> Result<JobsSummary> {
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

fn classify_job_status(last_progress: Option<&str>, last_error: Option<&str>) -> JobStatus {
    match last_progress {
        Some(line) if line.contains("failed error=") => JobStatus::Failed,
        Some(line) if line.contains("100.0% complete ") => JobStatus::Complete,
        Some(line) if line.contains("job_spawned") && last_error.is_some() => JobStatus::Failed,
        Some(_) => JobStatus::Running,
        None if last_error.is_some() => JobStatus::Failed,
        None => JobStatus::Starting,
    }
}

fn parse_pointer(contents: &str) -> BTreeMap<String, String> {
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

fn last_stream_line(path: &Path) -> Result<Option<String>> {
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

fn shell_arg_path(path: &Path) -> String {
    let value = path.display().to_string();
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':'))
    {
        value
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::cli::{MeasureArgs, TraversalProfile};
    use crate::sandbox::SandboxProfile;

    use super::{
        JobStatus, classify_job_status, ensure_no_active_job, job_artifact_dir, job_status,
        last_stream_line, parse_pointer, spawn_detached_measure,
    };

    #[test]
    fn parses_job_pointer_lines() {
        let pointer = parse_pointer("job_id=measure-1\nprogress_log=/tmp/job.log\n");

        assert_eq!(pointer.get("job_id").map(String::as_str), Some("measure-1"));
        assert_eq!(
            pointer.get("progress_log").map(String::as_str),
            Some("/tmp/job.log")
        );
    }

    #[test]
    fn job_status_labels_are_stable() {
        assert_eq!(JobStatus::Running.label(), "running");
        assert_eq!(JobStatus::Complete.label(), "complete");
    }

    #[test]
    fn status_classifier_does_not_complete_on_partial_progress() {
        let line = "[2026-06-15T18:25:30Z]   3.0% completed probe=p_000001";

        let status = classify_job_status(Some(line), None);

        assert_eq!(status, JobStatus::Running);
    }

    #[test]
    fn status_classifier_completes_only_on_final_progress() {
        let line = "[2026-06-15T18:25:30Z] 100.0% complete score=87";

        let status = classify_job_status(Some(line), None);

        assert_eq!(status, JobStatus::Complete);
    }

    #[test]
    fn status_classifier_marks_spawn_failure_from_stderr() {
        let line = "[detached]   0.0% job_spawned";

        let status = classify_job_status(Some(line), Some("Error: target executable not found"));

        assert_eq!(status, JobStatus::Failed);
    }

    #[test]
    fn job_status_reads_job_only_directory() {
        let out = temp_dir("job-status-running");
        let jobs = out.join("jobs");
        fs::create_dir_all(&jobs).expect("creates jobs directory");
        let progress = jobs.join("measure-1.log");
        let stdout = jobs.join("measure-1.stdout.log");
        let stderr = jobs.join("measure-1.stderr.log");
        fs::write(&progress, "[detached]   0.0% job_spawned\n").expect("writes progress log");
        fs::write(&stdout, "").expect("writes stdout log");
        fs::write(&stderr, "").expect("writes stderr log");
        fs::write(
            jobs.join("current"),
            format!(
                "job_id=measure-1\nprogress_log={}\nstdout_log={}\nstderr_log={}\n",
                progress.display(),
                stdout.display(),
                stderr.display()
            ),
        )
        .expect("writes current pointer");

        let summary = job_status(out.clone()).expect("reads job-only status");

        assert_eq!(summary.status, JobStatus::Running);
        assert_eq!(summary.job_id.as_deref(), Some("measure-1"));
        assert_eq!(
            summary.last_progress.as_deref(),
            Some("[detached]   0.0% job_spawned")
        );
        let _ = fs::remove_dir_all(out);
    }

    #[test]
    fn job_status_reports_failed_when_worker_errors_before_progress() {
        let out = temp_dir("job-status-worker-error");
        let jobs = out.join("jobs");
        fs::create_dir_all(&jobs).expect("creates jobs directory");
        let progress = jobs.join("measure-1.log");
        let stderr = jobs.join("measure-1.stderr.log");
        fs::write(&stderr, "Error: target executable was not found: rote\n")
            .expect("writes stderr log");
        fs::write(
            jobs.join("current"),
            format!(
                "job_id=measure-1\nprogress_log={}\nstderr_log={}\n",
                progress.display(),
                stderr.display()
            ),
        )
        .expect("writes current pointer");

        let summary = job_status(out.clone()).expect("reads failed status");

        assert_eq!(summary.status, JobStatus::Failed);
        assert_eq!(
            summary.last_error.as_deref(),
            Some("Error: target executable was not found: rote")
        );
        let _ = fs::remove_dir_all(out);
    }

    #[test]
    fn active_job_guard_rejects_running_job() {
        let out = temp_dir("job-status-active-guard");
        let jobs = out.join("jobs");
        fs::create_dir_all(&jobs).expect("creates jobs directory");
        let progress = jobs.join("measure-1.log");
        fs::write(&progress, "[detached]   0.0% job_spawned\n").expect("writes progress log");
        fs::write(
            jobs.join("current"),
            format!("job_id=measure-1\nprogress_log={}\n", progress.display()),
        )
        .expect("writes current pointer");

        let error = ensure_no_active_job(&out).expect_err("rejects active job");

        assert!(error.to_string().contains("already active"));
        assert!(error.to_string().contains("job_id=measure-1"));
        let _ = fs::remove_dir_all(out);
    }

    #[test]
    fn detached_measure_preflights_missing_target_before_creating_job_artifacts() {
        let out = temp_dir("detached-missing-target");
        let args = measure_args(
            PathBuf::from("cliare-missing-target-for-detached-preflight"),
            out.clone(),
        );

        let error = spawn_detached_measure(args).expect_err("rejects missing target");

        assert!(
            error
                .to_string()
                .contains("target executable was not found")
        );
        assert!(!out.exists());
    }

    #[test]
    fn job_artifact_dir_selects_named_context_without_finished_artifacts() {
        let root = PathBuf::from(".cliare");

        let artifact_dir = job_artifact_dir(&root, Some("Authenticated Context"));

        assert_eq!(
            artifact_dir,
            PathBuf::from(".cliare/contexts/authenticated-context")
        );
    }

    #[test]
    fn stream_tail_uses_last_non_empty_line() {
        let path =
            std::env::temp_dir().join(format!("cliare-job-stream-test-{}", std::process::id()));
        fs::write(&path, "first\n\nlast\n").expect("writes stream fixture");

        let line = last_stream_line(&path).expect("reads stream fixture");

        assert_eq!(line.as_deref(), Some("last"));
        let _ = fs::remove_file(path);
    }

    fn measure_args(target: PathBuf, out: PathBuf) -> MeasureArgs {
        MeasureArgs {
            target,
            out,
            timeout_ms: 5_000,
            output_limit_bytes: 1_048_576,
            profile: TraversalProfile::Standard,
            execution_mode: SandboxProfile::Isolated,
            max_depth: None,
            max_probes: None,
            min_expected_value: None,
            concurrency: None,
            snapshot_max_files: None,
            snapshot_max_directories: None,
            snapshot_max_hash_bytes: None,
            context: None,
            context_name: None,
            auth_state: None,
            local_context_state: None,
            fixture_state: None,
            network_state: None,
            runtime_dependency_state: None,
            context_workdir: None,
            refresh: true,
            detach: true,
            detached_worker: false,
            job_id: None,
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cliare-{name}-{}-{nonce}", std::process::id()))
    }
}
