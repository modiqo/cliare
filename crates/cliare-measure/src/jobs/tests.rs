use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cli::{MeasureArgs, TraversalProfile};
use crate::sandbox::SandboxProfile;

use super::model::JobStatus;
use super::spawn::{ensure_no_active_job, spawn_detached_measure};
use super::status::{
    classify_job_status, job_artifact_dir, job_status, last_stream_line, parse_pointer,
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
    let path = std::env::temp_dir().join(format!("cliare-job-stream-test-{}", std::process::id()));
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
