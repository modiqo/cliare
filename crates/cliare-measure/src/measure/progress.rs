use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::AsyncWriteExt;

use crate::error::{CliareError, Result};
use crate::evidence::{ProbeIntent, ProcessCompleted, ProcessStatus};
use crate::fingerprint::TargetFingerprint;
use crate::process::ProbeSpec;

use super::profile::ProbeProfile;
use super::summary::MeasurementSummary;

static MEASURE_JOB_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub(super) struct ProgressLog {
    pub(super) job_id: String,
    pub(super) path: PathBuf,
    pub(super) file: File,
    pub(super) max_probes: usize,
}

impl ProgressLog {
    pub(super) async fn create(
        out_dir: &Path,
        target: &TargetFingerprint,
        profile: &ProbeProfile,
        concurrency_limit: usize,
        job_id: Option<String>,
    ) -> Result<Self> {
        let jobs_dir = out_dir.join("jobs");
        fs::create_dir_all(&jobs_dir)
            .await
            .map_err(|source| CliareError::CreateProgressDir {
                path: jobs_dir.clone(),
                source,
            })?;

        let job_id = match job_id {
            Some(job_id) => job_id,
            None => new_measure_job_id()?,
        };
        let path = jobs_dir.join(format!("{job_id}.log"));
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)
            .await
            .map_err(|source| CliareError::OpenProgressLog {
                path: path.clone(),
                source,
            })?;

        let mut log = Self {
            job_id,
            path,
            file,
            max_probes: profile.max_probes,
        };
        log.write_header(target, profile, concurrency_limit, out_dir)
            .await?;
        log.write_current_pointer(&jobs_dir).await?;
        Ok(log)
    }

    pub(super) fn job_id(&self) -> &str {
        &self.job_id
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn announce(&self) {
        use std::io::IsTerminal as _;

        let stdout = std::io::stdout();
        if !stdout.is_terminal() {
            return;
        }
        let mut stdout = stdout.lock();
        let _ = writeln!(stdout, "CLIARE measure job created");
        let _ = writeln!(stdout, "job_id: {}", self.job_id);
        let _ = writeln!(stdout, "progress log: {}", self.path.display());
        let _ = writeln!(stdout, "tail: tail -f {}", self.path.display());
        let _ = writeln!(stdout);
        let _ = stdout.flush();
    }

    async fn write_header(
        &mut self,
        target: &TargetFingerprint,
        profile: &ProbeProfile,
        concurrency_limit: usize,
        out_dir: &Path,
    ) -> Result<()> {
        let header = format!(
            "# CLIARE measure progress\n\
             job_id: {}\n\
             target: {}\n\
             resolved: {}\n\
             out: {}\n\
             profile: {}\n\
             max_depth: {}\n\
             max_probes: {}\n\
             min_expected_value: {}\n\
             concurrency_limit: {}\n\
             progress: probe-budget percentage while traversal is running; final completion is 100.0%.\n\
             progress_formula: shown_percent = min(completed / max_probes * 100, 99.0) until complete.\n\
             progress_example: if completed=529 and max_probes=5000, shown_percent = 529 / 5000 * 100 = 10.58%, logged as 10.6%.\n\
             tail: tail -f {}\n\n",
            self.job_id,
            target.requested.display(),
            target.resolved.display(),
            out_dir.display(),
            profile.traversal_profile.label(),
            profile.max_depth,
            profile.max_probes,
            profile.min_expected_value,
            concurrency_limit,
            self.path.display()
        );
        self.write_raw(header.as_bytes()).await
    }

    async fn write_current_pointer(&mut self, jobs_dir: &Path) -> Result<()> {
        let path = jobs_dir.join("current");
        let mut contents = format!(
            "job_id={}\nprogress_log={}\ntail=tail -f {}\n",
            self.job_id,
            self.path.display(),
            self.path.display()
        );
        if let Ok(existing) = fs::read_to_string(&path).await
            && pointer_job_id(&existing).as_deref() == Some(self.job_id.as_str())
        {
            for key in ["pid", "stdout_log", "stderr_log", "status_command"] {
                if let Some(value) = pointer_value(&existing, key) {
                    contents.push_str(key);
                    contents.push('=');
                    contents.push_str(&value);
                    contents.push('\n');
                }
            }
        }
        fs::write(&path, contents)
            .await
            .map_err(|source| CliareError::WriteProgressLog { path, source })
    }

    pub(super) async fn created(&mut self) -> Result<()> {
        self.log(0.0, "job_created").await
    }

    pub(super) async fn message(
        &mut self,
        completed: usize,
        message: impl AsRef<str>,
    ) -> Result<()> {
        self.log(progress_percent(completed, self.max_probes), message)
            .await
    }

    pub(super) async fn scheduled(
        &mut self,
        probe_id: &str,
        probe: &ProbeSpec,
        probes_scheduled: usize,
        probes_completed: usize,
    ) -> Result<()> {
        self.message(
            probes_completed,
            format!(
                "scheduled probe={} intent={} path={} argv_suffix={} scheduled={} completed={}",
                probe_id,
                intent_label(probe.intent),
                path_label(&probe.path),
                args_label(&probe.args),
                probes_scheduled,
                probes_completed
            ),
        )
        .await
    }

    pub(super) async fn round_started(
        &mut self,
        round: usize,
        inflight: usize,
        probes_scheduled: usize,
        probes_completed: usize,
    ) -> Result<()> {
        self.message(
            probes_completed,
            format!(
                "round_started round={} inflight={} scheduled={} completed={}",
                round, inflight, probes_scheduled, probes_completed
            ),
        )
        .await
    }

    pub(super) async fn completed(
        &mut self,
        probe_id: &str,
        probe: &ProbeSpec,
        completed: &ProcessCompleted,
        counters: ProgressCounters,
        planner_stats: crate::planner::PlannerStats,
    ) -> Result<()> {
        self.message(
            counters.probes_completed,
            format!(
                "completed probe={} intent={} path={} status={} duration_ms={} side_effects={} completed={} scheduled={} round={} frontier_remaining={} highest_pending_expected_value={}",
                probe_id,
                intent_label(probe.intent),
                path_label(&probe.path),
                status_label(&completed.status),
                completed.duration_ms,
                completed.side_effects.total,
                counters.probes_completed,
                counters.probes_scheduled,
                counters.round,
                planner_stats.frontier_remaining,
                planner_stats
                    .highest_pending_expected_value
                    .map_or_else(|| "none".to_owned(), |value| value.to_string())
            ),
        )
        .await
    }

    pub(super) async fn failed(&mut self, completed: usize, error: &CliareError) -> Result<()> {
        self.log(
            progress_percent(completed, self.max_probes),
            format!("failed error={error}"),
        )
        .await
    }

    pub(super) async fn finished(&mut self, summary: &MeasurementSummary) -> Result<()> {
        self.log(
            100.0,
            format!(
                "complete score={:.0} probes_completed={} traversal_complete={} stop_reason={} scorecard={} shape={} evidence={}",
                summary.score_total,
                summary.probes_completed,
                summary.traversal_complete,
                summary.traversal_stop_reason,
                summary.scorecard_path.display(),
                summary.shape_path.display(),
                summary.evidence_path.display()
            ),
        )
        .await
    }

    async fn log(&mut self, percent: f64, message: impl AsRef<str>) -> Result<()> {
        let line = format!(
            "[{}] {:>5.1}% {}\n",
            progress_timestamp()?,
            percent,
            message.as_ref()
        );
        self.write_raw(line.as_bytes()).await
    }

    async fn write_raw(&mut self, bytes: &[u8]) -> Result<()> {
        self.file
            .write_all(bytes)
            .await
            .map_err(|source| CliareError::WriteProgressLog {
                path: self.path.clone(),
                source,
            })?;
        self.file
            .flush()
            .await
            .map_err(|source| CliareError::WriteProgressLog {
                path: self.path.clone(),
                source,
            })
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ProgressCounters {
    pub(super) probes_scheduled: usize,
    pub(super) probes_completed: usize,
    pub(super) round: usize,
}

fn pointer_job_id(contents: &str) -> Option<String> {
    pointer_value(contents, "job_id")
}

fn pointer_value(contents: &str, key: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let (candidate, value) = line.split_once('=')?;
        (candidate.trim() == key).then(|| value.trim().to_owned())
    })
}

pub fn new_measure_job_id() -> Result<String> {
    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(CliareError::TimeFormat)?;
    let sanitized = timestamp
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned();
    let sequence = MEASURE_JOB_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(format!(
        "measure-{sanitized}-{}-{sequence}",
        std::process::id()
    ))
}

fn progress_timestamp() -> Result<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(CliareError::TimeFormat)
}

pub(super) fn progress_percent(completed: usize, max_probes: usize) -> f64 {
    if max_probes == 0 {
        return 0.0;
    }
    ((completed as f64 / max_probes as f64) * 100.0).min(99.0)
}

fn intent_label(intent: ProbeIntent) -> &'static str {
    match intent {
        ProbeIntent::Help => "help",
        ProbeIntent::Version => "version",
        ProbeIntent::InvalidCommand => "invalid_command",
        ProbeIntent::InvalidChild => "invalid_child",
        ProbeIntent::InvalidFlag => "invalid_flag",
        ProbeIntent::OutputJson => "output_json",
        ProbeIntent::OutputYaml => "output_yaml",
        ProbeIntent::OutputTable => "output_table",
        ProbeIntent::OutputPlain => "output_plain",
        ProbeIntent::OutputJsonHelp => "output_json_help",
        ProbeIntent::OutputYamlHelp => "output_yaml_help",
        ProbeIntent::OutputTableHelp => "output_table_help",
        ProbeIntent::OutputPlainHelp => "output_plain_help",
    }
}

fn status_label(status: &ProcessStatus) -> String {
    match status {
        ProcessStatus::Exited { code } => {
            format!(
                "exited:{}",
                code.map_or_else(|| "none".to_owned(), |code| code.to_string())
            )
        }
        ProcessStatus::TimedOut => "timed_out".to_owned(),
        ProcessStatus::SpawnFailed { error } => format!("spawn_failed:{error}"),
    }
}

fn path_label(path: &[String]) -> String {
    if path.is_empty() {
        "<root>".to_owned()
    } else {
        path.join(" ")
    }
}

fn args_label(args: &[String]) -> String {
    if args.is_empty() {
        "<none>".to_owned()
    } else {
        args.join(" ")
    }
}
