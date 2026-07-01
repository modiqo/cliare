use std::fs::{self, File, OpenOptions};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::cli::MeasureArgs;
use crate::context::{self, RuntimeContext, RuntimeContextInput};
use crate::error::{CliareError, Result};
use crate::fingerprint;

use super::model::DetachedMeasureSummary;
use super::status::job_status;
use super::util::shell_arg_path;

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

pub(super) fn ensure_no_active_job(out: &Path) -> Result<()> {
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
