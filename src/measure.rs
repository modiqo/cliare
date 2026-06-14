use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::task::JoinHandle;

use crate::artifact_guide::{self, ArtifactGuideSummary};
use crate::ci::{self, CiArtifactSummary};
use crate::claims::ClaimSet;
use crate::cli::MeasureArgs;
use crate::context::{self, RuntimeContext, RuntimeContextInput};
use crate::error::{CliareError, Result};
use crate::evidence::{
    EvidenceKind, EvidenceWriter, ProbeIntent, ProbeScheduled, ProcessCompleted, ProcessStatus,
    RunFinished, RunStarted,
};
use crate::fingerprint::{TargetFingerprint, fingerprint_target};
use crate::observation::ShapeObservation;
use crate::planner::{
    ConvergencePolicy, DeterministicPlanner, ProbePlanner, bootstrap_invalid_command_token,
    bootstrap_invalid_flag_token,
};
use crate::process::{ProbeSpec, TargetProcess};
use crate::report::{self, PersonaArtifactSummary};
use crate::sandbox::Sandbox;
use crate::score::{self, ScoreRunContext};
use crate::shape;

const MEASUREMENT_CACHE_SCHEMA_VERSION: &str = "cliare.measure-cache.v1";
const MEASUREMENT_ENGINE: &str = "cliare-measure-v0";
static MEASURE_JOB_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct MeasurementSummary {
    pub target: TargetFingerprint,
    pub job_id: Option<String>,
    pub job_log_path: Option<PathBuf>,
    pub probes_completed: usize,
    pub evidence_path: PathBuf,
    pub shape_path: PathBuf,
    pub command_index_json_path: PathBuf,
    pub command_index_markdown_path: PathBuf,
    pub scorecard_path: PathBuf,
    pub report_path: PathBuf,
    pub ci_summary_path: PathBuf,
    pub sarif_path: PathBuf,
    pub junit_path: PathBuf,
    pub issues_markdown_path: PathBuf,
    pub issues_json_path: PathBuf,
    pub persona_report_count: usize,
    pub readme_path: PathBuf,
    pub agent_skill_path: PathBuf,
    pub sandbox_profile: String,
    pub sandbox_root: PathBuf,
    pub sandbox_home: PathBuf,
    pub sandbox_workdir: PathBuf,
    pub sandbox_env_policy: String,
    pub score_total: f64,
    pub score_measured_weight: f64,
    pub score_max_weight: f64,
    pub score_model: String,
    pub score_status: String,
    pub findings: usize,
    pub commands_precondition_blocked: usize,
    pub output_contracts_discovered: usize,
    pub machine_readable_output_contracts: usize,
    pub output_mode_probes_completed: usize,
    pub output_mode_parse_successes: usize,
    pub output_mode_precondition_blocked: usize,
    pub precondition_blocked_probes: usize,
    pub auth_required_probes: usize,
    pub local_context_required_probes: usize,
    pub fixture_required_probes: usize,
    pub actionable_precondition_probes: usize,
    pub precondition_recovery_rate: f64,
    pub side_effect_files_created: usize,
    pub side_effect_files_modified: usize,
    pub side_effect_files_deleted: usize,
    pub side_effect_files_total: usize,
    pub side_effect_probe_count: usize,
    pub credential_like_side_effects: usize,
    pub observed_max_depth: usize,
    pub traversal_profile: String,
    pub max_depth: usize,
    pub max_probes: usize,
    pub min_expected_value: u16,
    pub concurrency_limit: usize,
    pub traversal_rounds: usize,
    pub probes_scheduled: usize,
    pub probes_cancelled: usize,
    pub frontier_remaining: usize,
    pub highest_pending_expected_value: Option<u16>,
    pub candidates_skipped_by_depth: usize,
    pub candidates_skipped_by_convergence: usize,
    pub probes_skipped_by_budget: usize,
    pub budget_exhausted: bool,
    pub traversal_stop_reason: String,
    pub traversal_complete: bool,
    pub cache_hit: bool,
    pub runtime_context: RuntimeContext,
    pub suite_root_path: PathBuf,
    pub runtime_context_path: Option<PathBuf>,
    pub context_suite_path: Option<PathBuf>,
    pub context_compare_path: Option<PathBuf>,
}

impl MeasurementSummary {
    pub fn set_ci_artifacts(&mut self, artifacts: CiArtifactSummary) {
        self.ci_summary_path = artifacts.summary_path;
        self.sarif_path = artifacts.sarif_path;
        self.junit_path = artifacts.junit_path;
    }

    pub fn set_artifact_guides(&mut self, guides: ArtifactGuideSummary) {
        self.readme_path = guides.readme_path;
        self.agent_skill_path = guides.agent_skill_path;
    }

    pub fn set_persona_artifacts(&mut self, artifacts: PersonaArtifactSummary) {
        let persona_report_count = artifacts.persona_count();
        self.issues_markdown_path = artifacts.issues_markdown_path;
        self.issues_json_path = artifacts.issues_json_path;
        self.persona_report_count = persona_report_count;
    }

    pub fn terminal_summary(&self) -> String {
        let mut lines = vec![
            "CLIARE measure complete".to_owned(),
            format!("target: {}", self.target.requested.display()),
            format!("resolved: {}", self.target.resolved.display()),
            format!(
                "score: {:.1}/100 ({}, measured {:.2}/{:.2}, model {})",
                self.score_total,
                self.score_status,
                self.score_measured_weight,
                self.score_max_weight,
                self.score_model
            ),
            format!("cache: {}", if self.cache_hit { "hit" } else { "miss" }),
        ];
        if let Some(job_id) = &self.job_id {
            lines.push(format!("job_id: {job_id}"));
        }
        if let Some(path) = &self.job_log_path {
            lines.push(format!("progress log: {}", path.display()));
        }
        lines.extend([
            format!("probes: {}", self.probes_completed),
            format!("findings: {}", self.findings),
            "preconditions:".to_owned(),
            format!("  commands blocked: {}", self.commands_precondition_blocked),
            format!("  probes blocked: {}", self.precondition_blocked_probes),
            format!("  auth required: {}", self.auth_required_probes),
            format!(
                "  local context required: {}",
                self.local_context_required_probes
            ),
            format!("  fixture required: {}", self.fixture_required_probes),
            format!(
                "  actionable recovery: {} ({:.1}%)",
                self.actionable_precondition_probes,
                self.precondition_recovery_rate * 100.0
            ),
            "output contracts:".to_owned(),
            format!("  discovered: {}", self.output_contracts_discovered),
            format!(
                "  machine-readable: {}",
                self.machine_readable_output_contracts
            ),
            format!("  probes completed: {}", self.output_mode_probes_completed),
            format!("  parse successes: {}", self.output_mode_parse_successes),
            format!("  blocked: {}", self.output_mode_precondition_blocked),
            "side effects:".to_owned(),
            format!("  file changes: {}", self.side_effect_files_total),
            format!("  probes with changes: {}", self.side_effect_probe_count),
            format!("  created: {}", self.side_effect_files_created),
            format!("  modified: {}", self.side_effect_files_modified),
            format!("  deleted: {}", self.side_effect_files_deleted),
            format!(
                "  credential-like paths: {}",
                self.credential_like_side_effects
            ),
            "runtime isolation:".to_owned(),
            format!("  sandbox profile: {}", self.sandbox_profile),
            format!("  env policy: {}", self.sandbox_env_policy),
            format!("  sandbox root: {}", self.sandbox_root.display()),
            format!("  sandbox home: {}", self.sandbox_home.display()),
            format!("  sandbox workdir: {}", self.sandbox_workdir.display()),
            "runtime context:".to_owned(),
            format!("  profile: {}", self.runtime_context.profile.label()),
            format!("  name: {}", self.runtime_context.name),
            format!("  auth: {}", self.runtime_context.auth_state.label()),
            format!(
                "  local context: {}",
                self.runtime_context.local_context_state.label()
            ),
            format!("  fixture: {}", self.runtime_context.fixture_state.label()),
            format!("  network: {}", self.runtime_context.network_state.label()),
            format!(
                "  runtime dependency: {}",
                self.runtime_context.runtime_dependency_state.label()
            ),
            format!("  cwd policy: {}", self.runtime_context.cwd_policy.label()),
            format!("  suite root: {}", self.suite_root_path.display()),
            "coverage pressure:".to_owned(),
            format!("  profile: {}", self.traversal_profile),
            format!(
                "  depth: observed {} / budget {}",
                self.observed_max_depth, self.max_depth
            ),
            format!(
                "  probes: completed {} / budget {}",
                self.probes_completed, self.max_probes
            ),
            format!("  min expected value: {}", self.min_expected_value),
            format!("  concurrency limit: {}", self.concurrency_limit),
            format!("  scheduler rounds: {}", self.traversal_rounds),
            format!("  probes scheduled: {}", self.probes_scheduled),
            format!("  probes cancelled: {}", self.probes_cancelled),
            format!("  frontier remaining: {}", self.frontier_remaining),
            format!(
                "  highest pending expected value: {}",
                self.highest_pending_expected_value
                    .map_or_else(|| "none".to_owned(), |value| value.to_string())
            ),
            format!("  skipped by depth: {}", self.candidates_skipped_by_depth),
            format!(
                "  skipped by convergence: {}",
                self.candidates_skipped_by_convergence
            ),
            format!(
                "  skipped by probe budget: {}",
                self.probes_skipped_by_budget
            ),
            format!("  budget exhausted: {}", self.budget_exhausted),
            format!("  stop reason: {}", self.traversal_stop_reason),
            format!("  traversal complete: {}", self.traversal_complete),
            "artifacts:".to_owned(),
            format!("  evidence: {}", self.evidence_path.display()),
            format!("  shape: {}", self.shape_path.display()),
            format!(
                "  command index: {}",
                self.command_index_json_path.display()
            ),
            format!(
                "  command index report: {}",
                self.command_index_markdown_path.display()
            ),
            format!("  scorecard: {}", self.scorecard_path.display()),
            format!("  report: {}", self.report_path.display()),
            format!("  ci summary: {}", self.ci_summary_path.display()),
            format!("  sarif: {}", self.sarif_path.display()),
            format!("  junit: {}", self.junit_path.display()),
            format!("  issues: {}", self.issues_json_path.display()),
            format!("  issue report: {}", self.issues_markdown_path.display()),
            format!(
                "  persona reports: {} markdown/json pairs",
                self.persona_report_count
            ),
            format!("  readme: {}", self.readme_path.display()),
            format!("  agent guide: {}", self.agent_skill_path.display()),
        ]);
        if let Some(path) = &self.runtime_context_path {
            lines.push(format!("  runtime context: {}", path.display()));
        }
        if let Some(path) = &self.context_suite_path {
            lines.push(format!("  context suite: {}", path.display()));
        }
        if let Some(path) = &self.context_compare_path {
            lines.push(format!("  context comparison: {}", path.display()));
        }

        format!("{}\n", lines.join("\n"))
    }
}

pub async fn measure(args: MeasureArgs) -> Result<MeasurementSummary> {
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
    let target = fingerprint_target(&args.target).await?;
    let max_depth = args.resolved_max_depth();
    let max_probes = args.resolved_max_probes();
    let min_expected_value = args.resolved_min_expected_value();
    let concurrency_limit = args.resolved_concurrency();
    let profile = ProbeProfile::from_args(
        &args,
        max_depth,
        max_probes,
        min_expected_value,
        concurrency_limit,
        args.execution_mode.label(),
        runtime_context.clone(),
    );

    if !args.refresh
        && let Some(mut summary) = cached_summary(&artifact_dir, &target, &profile).await?
    {
        summary.suite_root_path = args.out.clone();
        let runtime_context_path =
            context::write_runtime_context(&artifact_dir, &runtime_context).await?;
        summary.runtime_context_path = Some(runtime_context_path);
        let persona_artifacts = report::write_all_persona_reports(&artifact_dir).await?;
        summary.set_persona_artifacts(persona_artifacts);
        let guides = artifact_guide::write_measurement_guides(&artifact_dir).await?;
        summary.set_artifact_guides(guides);
        if runtime_context.is_context_suite_measurement() {
            let _ = context::refresh_context_suite(&args.out).await?;
            summary.context_suite_path = Some(args.out.join("context-suite.json"));
            summary.context_compare_path = Some(args.out.join("context-compare.md"));
        }
        return Ok(summary);
    }

    let mut progress = ProgressLog::create(
        &artifact_dir,
        &target,
        &profile,
        concurrency_limit,
        args.job_id.clone(),
    )
    .await?;
    if !args.detached_worker {
        progress.announce();
    }
    progress.created().await?;
    progress
        .message(
            0,
            format!(
                "target_fingerprinted resolved={} sha256={} size_bytes={}",
                target.resolved.display(),
                target.binary_sha256,
                target.size_bytes
            ),
        )
        .await?;
    progress
        .message(
            0,
            format!(
                "cache_miss refresh={} profile={} max_depth={} max_probes={} concurrency={}",
                args.refresh,
                args.profile.label(),
                max_depth,
                max_probes,
                concurrency_limit
            ),
        )
        .await?;

    let runtime_context_path =
        context::write_runtime_context(&artifact_dir, &runtime_context).await?;
    progress
        .message(
            0,
            format!(
                "runtime_context_written profile={} name={} path={}",
                runtime_context.profile.label(),
                runtime_context.name,
                runtime_context_path.display()
            ),
        )
        .await?;

    let sandbox = Sandbox::create_with_profile(
        &artifact_dir,
        runtime_context.workdir.as_deref(),
        args.execution_mode,
    )
    .await?;
    progress
        .message(
            0,
            format!(
                "sandbox_created root={} profile={}",
                sandbox.metadata().root.display(),
                sandbox.metadata().profile.label()
            ),
        )
        .await?;
    let mut evidence = EvidenceWriter::create(&artifact_dir).await?;
    progress.message(0, "evidence_log_created").await?;

    evidence
        .append(EvidenceKind::RunStarted(RunStarted {
            target: target.clone(),
            artifact_dir: artifact_dir.clone(),
            runtime_context: runtime_context.clone(),
            sandbox: sandbox.metadata().clone(),
        }))
        .await?;
    progress.message(0, "run_started_evidence_written").await?;

    let binary_name = target_binary_name(&target);
    let mut planner = DeterministicPlanner::with_policy(
        max_depth,
        ConvergencePolicy::new(min_expected_value),
        invalid_token_seed(&binary_name),
    );
    planner.seed(bootstrap_probes(&target));
    let process = TargetProcess::new(
        target.resolved.clone(),
        args.timeout(),
        args.output_limit_bytes,
    );
    let traversal = match run_traversal(TraversalContext {
        target: &target,
        sandbox: &sandbox,
        process: &process,
        evidence: &mut evidence,
        progress: &mut progress,
        planner: &mut planner,
        binary_name: &binary_name,
        limits: TraversalLimits {
            max_probes,
            concurrency_limit,
        },
    })
    .await
    {
        Ok(traversal) => traversal,
        Err(error) => {
            let _ = progress.failed(0, &error).await;
            return Err(error);
        }
    };
    progress
        .message(
            traversal.probes_completed,
            format!(
                "traversal_finished probes_completed={} probes_scheduled={} rounds={}",
                traversal.probes_completed, traversal.probes_scheduled, traversal.rounds
            ),
        )
        .await?;

    evidence
        .append(EvidenceKind::RunFinished(RunFinished {
            probes_completed: traversal.probes_completed,
        }))
        .await?;
    progress
        .message(traversal.probes_completed, "run_finished_evidence_written")
        .await?;

    shape::write_shape(&artifact_dir, target.clone(), &traversal.observations).await?;
    progress
        .message(traversal.probes_completed, "shape_artifacts_written")
        .await?;
    let planner_stats = planner.stats();
    let run_context = ScoreRunContext {
        max_depth: planner_stats.max_depth,
        max_probes,
        min_expected_value: planner_stats.min_expected_value,
        concurrency_limit,
        traversal_rounds: traversal.rounds,
        probes_scheduled: traversal.probes_scheduled,
        probes_cancelled: traversal.probes_cancelled,
        traversal_profile: args.profile.label(),
        frontier_remaining: planner_stats.frontier_remaining,
        highest_pending_expected_value: planner_stats.highest_pending_expected_value,
        candidates_skipped_by_depth: planner_stats.candidates_skipped_by_depth,
        candidates_skipped_by_convergence: planner_stats.candidates_skipped_by_convergence,
        sandbox: score::SandboxScoreContext::from(sandbox.metadata()),
        runtime_context: runtime_context.clone(),
    };
    let score_artifacts = score::write_score_artifacts(
        &artifact_dir,
        target.clone(),
        &traversal.observations,
        run_context,
    )
    .await?;
    progress
        .message(
            traversal.probes_completed,
            format!(
                "score_artifacts_written score={:.1} scorecard={}",
                score_artifacts.total,
                score_artifacts.scorecard_path.display()
            ),
        )
        .await?;
    let ci_artifacts = ci::write_ci_artifacts(&artifact_dir, None).await?;
    progress
        .message(traversal.probes_completed, "ci_artifacts_written")
        .await?;
    let persona_artifacts = report::write_all_persona_reports(&artifact_dir).await?;
    progress
        .message(
            traversal.probes_completed,
            format!(
                "persona_reports_written personas={} issues={}",
                persona_artifacts.persona_count(),
                persona_artifacts.issues_json_path.display()
            ),
        )
        .await?;
    let persona_report_count = persona_artifacts.persona_count();

    let mut summary = MeasurementSummary {
        target,
        job_id: Some(progress.job_id().to_owned()),
        job_log_path: Some(progress.path().to_path_buf()),
        probes_completed: traversal.probes_completed,
        evidence_path: artifact_dir.join("evidence.jsonl"),
        shape_path: artifact_dir.join("shape.json"),
        command_index_json_path: artifact_dir.join("command-index.json"),
        command_index_markdown_path: artifact_dir.join("command-index.md"),
        scorecard_path: score_artifacts.scorecard_path,
        report_path: score_artifacts.report_path,
        ci_summary_path: ci_artifacts.summary_path,
        sarif_path: ci_artifacts.sarif_path,
        junit_path: ci_artifacts.junit_path,
        issues_markdown_path: persona_artifacts.issues_markdown_path,
        issues_json_path: persona_artifacts.issues_json_path,
        persona_report_count,
        readme_path: artifact_dir.join("README.md"),
        agent_skill_path: artifact_dir.join("AGENT_SKILL.md"),
        sandbox_profile: score_artifacts.sandbox_profile.to_owned(),
        sandbox_root: score_artifacts.sandbox_root,
        sandbox_home: score_artifacts.sandbox_home,
        sandbox_workdir: score_artifacts.sandbox_workdir,
        sandbox_env_policy: score_artifacts.sandbox_env_policy.to_owned(),
        score_total: score_artifacts.total,
        score_measured_weight: score_artifacts.measured_weight,
        score_max_weight: score_artifacts.max_weight,
        score_model: score_artifacts.model.to_owned(),
        score_status: score_artifacts.status.to_owned(),
        findings: score_artifacts.findings,
        commands_precondition_blocked: score_artifacts.commands_precondition_blocked,
        output_contracts_discovered: score_artifacts.output_contracts_discovered,
        machine_readable_output_contracts: score_artifacts.machine_readable_output_contracts,
        output_mode_probes_completed: score_artifacts.output_mode_probes_completed,
        output_mode_parse_successes: score_artifacts.output_mode_parse_successes,
        output_mode_precondition_blocked: score_artifacts.output_mode_precondition_blocked,
        precondition_blocked_probes: score_artifacts.precondition_blocked_probes,
        auth_required_probes: score_artifacts.auth_required_probes,
        local_context_required_probes: score_artifacts.local_context_required_probes,
        fixture_required_probes: score_artifacts.fixture_required_probes,
        actionable_precondition_probes: score_artifacts.actionable_precondition_probes,
        precondition_recovery_rate: score_artifacts.precondition_recovery_rate,
        side_effect_files_created: score_artifacts.side_effect_files_created,
        side_effect_files_modified: score_artifacts.side_effect_files_modified,
        side_effect_files_deleted: score_artifacts.side_effect_files_deleted,
        side_effect_files_total: score_artifacts.side_effect_files_total,
        side_effect_probe_count: score_artifacts.side_effect_probe_count,
        credential_like_side_effects: score_artifacts.credential_like_side_effects,
        observed_max_depth: score_artifacts.observed_max_depth,
        traversal_profile: score_artifacts.traversal_profile.to_owned(),
        max_depth: score_artifacts.max_depth,
        max_probes: score_artifacts.max_probes,
        min_expected_value: score_artifacts.min_expected_value,
        concurrency_limit: score_artifacts.concurrency_limit,
        traversal_rounds: score_artifacts.traversal_rounds,
        probes_scheduled: score_artifacts.probes_scheduled,
        probes_cancelled: score_artifacts.probes_cancelled,
        frontier_remaining: score_artifacts.frontier_remaining,
        highest_pending_expected_value: score_artifacts.highest_pending_expected_value,
        candidates_skipped_by_depth: score_artifacts.candidates_skipped_by_depth,
        candidates_skipped_by_convergence: score_artifacts.candidates_skipped_by_convergence,
        probes_skipped_by_budget: score_artifacts.probes_skipped_by_budget,
        budget_exhausted: score_artifacts.budget_exhausted,
        traversal_stop_reason: score_artifacts.traversal_stop_reason.to_owned(),
        traversal_complete: score_artifacts.traversal_complete,
        cache_hit: false,
        runtime_context: runtime_context.clone(),
        suite_root_path: args.out.clone(),
        runtime_context_path: Some(runtime_context_path),
        context_suite_path: None,
        context_compare_path: None,
    };
    let guides = artifact_guide::write_measurement_guides(&artifact_dir).await?;
    summary.set_artifact_guides(guides);
    progress
        .message(traversal.probes_completed, "artifact_guides_written")
        .await?;
    write_cache_manifest(&artifact_dir, &summary, profile).await?;
    progress
        .message(traversal.probes_completed, "measure_cache_written")
        .await?;
    if runtime_context.is_context_suite_measurement() {
        let _ = context::refresh_context_suite(&args.out).await?;
        summary.context_suite_path = Some(args.out.join("context-suite.json"));
        summary.context_compare_path = Some(args.out.join("context-compare.md"));
        progress
            .message(traversal.probes_completed, "context_suite_written")
            .await?;
    }
    progress.finished(&summary).await?;

    Ok(summary)
}

#[derive(Debug)]
struct TraversalRun {
    observations: Vec<ShapeObservation>,
    probes_scheduled: usize,
    probes_completed: usize,
    probes_cancelled: usize,
    rounds: usize,
}

#[derive(Debug)]
struct ScheduledProbe {
    probe_id: String,
    probe: ProbeSpec,
    handle: JoinHandle<Result<crate::process::ProbeOutcome>>,
}

struct TraversalContext<'a> {
    target: &'a TargetFingerprint,
    sandbox: &'a Sandbox,
    process: &'a TargetProcess,
    evidence: &'a mut EvidenceWriter,
    progress: &'a mut ProgressLog,
    planner: &'a mut DeterministicPlanner,
    binary_name: &'a str,
    limits: TraversalLimits,
}

#[derive(Debug, Clone, Copy)]
struct TraversalLimits {
    max_probes: usize,
    concurrency_limit: usize,
}

#[derive(Debug)]
struct ProgressLog {
    job_id: String,
    path: PathBuf,
    file: File,
    max_probes: usize,
}

impl ProgressLog {
    async fn create(
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

    fn job_id(&self) -> &str {
        &self.job_id
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn announce(&self) {
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

    async fn created(&mut self) -> Result<()> {
        self.log(0.0, "job_created").await
    }

    async fn message(&mut self, completed: usize, message: impl AsRef<str>) -> Result<()> {
        self.log(progress_percent(completed, self.max_probes), message)
            .await
    }

    async fn scheduled(
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

    async fn round_started(
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

    async fn completed(
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

    async fn failed(&mut self, completed: usize, error: &CliareError) -> Result<()> {
        self.log(
            progress_percent(completed, self.max_probes),
            format!("failed error={error}"),
        )
        .await
    }

    async fn finished(&mut self, summary: &MeasurementSummary) -> Result<()> {
        self.log(
            100.0,
            format!(
                "complete score={:.1} probes_completed={} traversal_complete={} stop_reason={} scorecard={} shape={} evidence={}",
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
struct ProgressCounters {
    probes_scheduled: usize,
    probes_completed: usize,
    round: usize,
}

async fn run_traversal(context: TraversalContext<'_>) -> Result<TraversalRun> {
    let mut observations = Vec::new();
    let mut probes_scheduled = 0_usize;
    let mut probes_completed = 0_usize;
    let mut rounds = 0_usize;

    loop {
        let mut round = Vec::new();
        while round.len() < context.limits.concurrency_limit
            && probes_scheduled < context.limits.max_probes
        {
            let Some(probe) = context.planner.next() else {
                break;
            };
            probes_scheduled += 1;
            let probe_id = format!("p_{:06}", probes_scheduled);
            let execution = context.sandbox.execution_for_probe(&probe_id).await?;

            context
                .evidence
                .append(EvidenceKind::ProbeScheduled(ProbeScheduled {
                    probe_id: probe_id.clone(),
                    argv: probe.argv(&context.target.resolved),
                    path: probe.path.clone(),
                    intent: probe.intent,
                    sandbox: context.sandbox.probe_evidence_for(&execution),
                }))
                .await?;
            context
                .progress
                .scheduled(&probe_id, &probe, probes_scheduled, probes_completed)
                .await?;

            let process = context.process.clone();
            let task_probe = probe.clone();
            let handle = tokio::spawn(async move { process.run(&task_probe, execution).await });
            round.push(ScheduledProbe {
                probe_id,
                probe,
                handle,
            });
        }

        if round.is_empty() {
            break;
        }
        rounds += 1;
        context
            .progress
            .round_started(rounds, round.len(), probes_scheduled, probes_completed)
            .await?;

        let mut round_error = None;
        for scheduled in round {
            let outcome = match scheduled.handle.await {
                Ok(Ok(outcome)) => outcome,
                Ok(Err(error)) => {
                    round_error.get_or_insert(error);
                    continue;
                }
                Err(error) => {
                    round_error.get_or_insert(CliareError::Join(error));
                    continue;
                }
            };
            probes_completed += 1;
            let probe_id = scheduled.probe_id.clone();
            let probe = scheduled.probe.clone();
            let completed = ProcessCompleted::from_outcome(probe_id.clone(), outcome);
            let event_id = context
                .evidence
                .append(EvidenceKind::ProcessCompleted(completed.clone()))
                .await?;

            observations.push(ShapeObservation {
                evidence_id: event_id,
                intent: probe.intent,
                path: probe.path.clone(),
                process: completed.clone(),
            });

            let claims = ClaimSet::from_observations(context.binary_name, &observations);
            context.planner.extend_from_claims(&claims);
            context
                .progress
                .completed(
                    &probe_id,
                    &probe,
                    &completed,
                    ProgressCounters {
                        probes_scheduled,
                        probes_completed,
                        round: rounds,
                    },
                    context.planner.stats(),
                )
                .await?;
        }

        if let Some(error) = round_error {
            let _ = context.progress.failed(probes_completed, &error).await;
            return Err(error);
        }
    }

    Ok(TraversalRun {
        observations,
        probes_scheduled,
        probes_completed,
        probes_cancelled: probes_scheduled.saturating_sub(probes_completed),
        rounds,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct ProbeProfile {
    traversal_profile: crate::cli::TraversalProfile,
    sandbox_profile: String,
    #[serde(default)]
    runtime_context: RuntimeContext,
    timeout_ms: u64,
    output_limit_bytes: usize,
    max_depth: usize,
    max_probes: usize,
    min_expected_value: u16,
    #[serde(default)]
    concurrency_limit: usize,
}

impl ProbeProfile {
    fn from_args(
        args: &MeasureArgs,
        max_depth: usize,
        max_probes: usize,
        min_expected_value: u16,
        concurrency_limit: usize,
        sandbox_profile: &str,
        runtime_context: RuntimeContext,
    ) -> Self {
        Self {
            traversal_profile: args.profile,
            sandbox_profile: sandbox_profile.to_owned(),
            runtime_context,
            timeout_ms: args.timeout_ms,
            output_limit_bytes: args.output_limit_bytes,
            max_depth,
            max_probes,
            min_expected_value,
            concurrency_limit,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct MeasurementCacheManifest {
    schema_version: String,
    cliare_version: String,
    engine: String,
    target: TargetFingerprint,
    profile: ProbeProfile,
    summary: CachedMeasurementSummary,
}

#[derive(Debug, Deserialize, Serialize)]
struct CachedMeasurementSummary {
    probes_completed: usize,
    score_total: f64,
    score_measured_weight: f64,
    score_max_weight: f64,
    score_model: String,
    score_status: String,
    sandbox_profile: String,
    sandbox_root: PathBuf,
    sandbox_home: PathBuf,
    sandbox_workdir: PathBuf,
    sandbox_env_policy: String,
    findings: usize,
    #[serde(default)]
    commands_precondition_blocked: usize,
    output_contracts_discovered: usize,
    machine_readable_output_contracts: usize,
    output_mode_probes_completed: usize,
    output_mode_parse_successes: usize,
    #[serde(default)]
    output_mode_precondition_blocked: usize,
    #[serde(default)]
    precondition_blocked_probes: usize,
    #[serde(default)]
    auth_required_probes: usize,
    #[serde(default)]
    local_context_required_probes: usize,
    #[serde(default)]
    fixture_required_probes: usize,
    #[serde(default)]
    actionable_precondition_probes: usize,
    #[serde(default)]
    precondition_recovery_rate: f64,
    side_effect_files_created: usize,
    side_effect_files_modified: usize,
    side_effect_files_deleted: usize,
    side_effect_files_total: usize,
    side_effect_probe_count: usize,
    credential_like_side_effects: usize,
    observed_max_depth: usize,
    traversal_profile: String,
    max_depth: usize,
    max_probes: usize,
    min_expected_value: u16,
    #[serde(default)]
    concurrency_limit: usize,
    #[serde(default)]
    traversal_rounds: usize,
    #[serde(default)]
    probes_scheduled: usize,
    #[serde(default)]
    probes_cancelled: usize,
    frontier_remaining: usize,
    highest_pending_expected_value: Option<u16>,
    candidates_skipped_by_depth: usize,
    candidates_skipped_by_convergence: usize,
    probes_skipped_by_budget: usize,
    budget_exhausted: bool,
    traversal_stop_reason: String,
    traversal_complete: bool,
}

async fn cached_summary(
    out_dir: &std::path::Path,
    target: &TargetFingerprint,
    profile: &ProbeProfile,
) -> Result<Option<MeasurementSummary>> {
    let path = out_dir.join("measure-cache.json");
    let bytes = match fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(CliareError::ReadMeasurementCache { path, source });
        }
    };
    let manifest: MeasurementCacheManifest =
        serde_json::from_slice(&bytes).map_err(|source| CliareError::ParseMeasurementCache {
            path: path.clone(),
            source,
        })?;

    if !manifest.matches(target, profile) || !artifacts_exist(out_dir).await? {
        return Ok(None);
    }

    Ok(Some(manifest.into_summary(out_dir)))
}

impl MeasurementCacheManifest {
    fn matches(&self, target: &TargetFingerprint, profile: &ProbeProfile) -> bool {
        self.schema_version == MEASUREMENT_CACHE_SCHEMA_VERSION
            && self.cliare_version == env!("CARGO_PKG_VERSION")
            && self.engine == MEASUREMENT_ENGINE
            && &self.target == target
            && &self.profile == profile
    }

    fn into_summary(self, out_dir: &std::path::Path) -> MeasurementSummary {
        MeasurementSummary {
            target: self.target,
            job_id: None,
            job_log_path: None,
            probes_completed: self.summary.probes_completed,
            evidence_path: out_dir.join("evidence.jsonl"),
            shape_path: out_dir.join("shape.json"),
            command_index_json_path: out_dir.join("command-index.json"),
            command_index_markdown_path: out_dir.join("command-index.md"),
            scorecard_path: out_dir.join("scorecard.json"),
            report_path: out_dir.join("report.md"),
            ci_summary_path: out_dir.join("summary.md"),
            sarif_path: out_dir.join("findings.sarif"),
            junit_path: out_dir.join("junit.xml"),
            issues_markdown_path: out_dir.join("issues.md"),
            issues_json_path: out_dir.join("issues.json"),
            persona_report_count: report::Persona::all().len(),
            readme_path: out_dir.join("README.md"),
            agent_skill_path: out_dir.join("AGENT_SKILL.md"),
            score_total: self.summary.score_total,
            score_measured_weight: self.summary.score_measured_weight,
            score_max_weight: self.summary.score_max_weight,
            score_model: self.summary.score_model,
            score_status: self.summary.score_status,
            sandbox_profile: self.summary.sandbox_profile,
            sandbox_root: self.summary.sandbox_root,
            sandbox_home: self.summary.sandbox_home,
            sandbox_workdir: self.summary.sandbox_workdir,
            sandbox_env_policy: self.summary.sandbox_env_policy,
            findings: self.summary.findings,
            commands_precondition_blocked: self.summary.commands_precondition_blocked,
            output_contracts_discovered: self.summary.output_contracts_discovered,
            machine_readable_output_contracts: self.summary.machine_readable_output_contracts,
            output_mode_probes_completed: self.summary.output_mode_probes_completed,
            output_mode_parse_successes: self.summary.output_mode_parse_successes,
            output_mode_precondition_blocked: self.summary.output_mode_precondition_blocked,
            precondition_blocked_probes: self.summary.precondition_blocked_probes,
            auth_required_probes: self.summary.auth_required_probes,
            local_context_required_probes: self.summary.local_context_required_probes,
            fixture_required_probes: self.summary.fixture_required_probes,
            actionable_precondition_probes: self.summary.actionable_precondition_probes,
            precondition_recovery_rate: self.summary.precondition_recovery_rate,
            side_effect_files_created: self.summary.side_effect_files_created,
            side_effect_files_modified: self.summary.side_effect_files_modified,
            side_effect_files_deleted: self.summary.side_effect_files_deleted,
            side_effect_files_total: self.summary.side_effect_files_total,
            side_effect_probe_count: self.summary.side_effect_probe_count,
            credential_like_side_effects: self.summary.credential_like_side_effects,
            observed_max_depth: self.summary.observed_max_depth,
            traversal_profile: self.summary.traversal_profile,
            max_depth: self.summary.max_depth,
            max_probes: self.summary.max_probes,
            min_expected_value: self.summary.min_expected_value,
            concurrency_limit: self.summary.concurrency_limit,
            traversal_rounds: self.summary.traversal_rounds,
            probes_scheduled: self.summary.probes_scheduled,
            probes_cancelled: self.summary.probes_cancelled,
            frontier_remaining: self.summary.frontier_remaining,
            highest_pending_expected_value: self.summary.highest_pending_expected_value,
            candidates_skipped_by_depth: self.summary.candidates_skipped_by_depth,
            candidates_skipped_by_convergence: self.summary.candidates_skipped_by_convergence,
            probes_skipped_by_budget: self.summary.probes_skipped_by_budget,
            budget_exhausted: self.summary.budget_exhausted,
            traversal_stop_reason: self.summary.traversal_stop_reason,
            traversal_complete: self.summary.traversal_complete,
            cache_hit: true,
            runtime_context: self.profile.runtime_context,
            suite_root_path: out_dir.to_path_buf(),
            runtime_context_path: Some(out_dir.join("runtime-context.json")),
            context_suite_path: None,
            context_compare_path: None,
        }
    }
}

async fn artifacts_exist(out_dir: &std::path::Path) -> Result<bool> {
    for name in [
        "evidence.jsonl",
        "shape.json",
        "command-index.json",
        "command-index.md",
        "scorecard.json",
        "report.md",
        "summary.md",
        "findings.sarif",
        "junit.xml",
    ] {
        let path = out_dir.join(name);
        match fs::metadata(&path).await {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => return Ok(false),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(source) => {
                return Err(CliareError::ReadMeasurementCache { path, source });
            }
        }
    }
    Ok(true)
}

async fn write_cache_manifest(
    out_dir: &std::path::Path,
    summary: &MeasurementSummary,
    profile: ProbeProfile,
) -> Result<()> {
    let path = out_dir.join("measure-cache.json");
    let manifest = MeasurementCacheManifest {
        schema_version: MEASUREMENT_CACHE_SCHEMA_VERSION.to_owned(),
        cliare_version: env!("CARGO_PKG_VERSION").to_owned(),
        engine: MEASUREMENT_ENGINE.to_owned(),
        target: summary.target.clone(),
        profile,
        summary: CachedMeasurementSummary {
            probes_completed: summary.probes_completed,
            score_total: summary.score_total,
            score_measured_weight: summary.score_measured_weight,
            score_max_weight: summary.score_max_weight,
            score_model: summary.score_model.to_owned(),
            score_status: summary.score_status.to_owned(),
            sandbox_profile: summary.sandbox_profile.to_owned(),
            sandbox_root: summary.sandbox_root.clone(),
            sandbox_home: summary.sandbox_home.clone(),
            sandbox_workdir: summary.sandbox_workdir.clone(),
            sandbox_env_policy: summary.sandbox_env_policy.to_owned(),
            findings: summary.findings,
            commands_precondition_blocked: summary.commands_precondition_blocked,
            output_contracts_discovered: summary.output_contracts_discovered,
            machine_readable_output_contracts: summary.machine_readable_output_contracts,
            output_mode_probes_completed: summary.output_mode_probes_completed,
            output_mode_parse_successes: summary.output_mode_parse_successes,
            output_mode_precondition_blocked: summary.output_mode_precondition_blocked,
            precondition_blocked_probes: summary.precondition_blocked_probes,
            auth_required_probes: summary.auth_required_probes,
            local_context_required_probes: summary.local_context_required_probes,
            fixture_required_probes: summary.fixture_required_probes,
            actionable_precondition_probes: summary.actionable_precondition_probes,
            precondition_recovery_rate: summary.precondition_recovery_rate,
            side_effect_files_created: summary.side_effect_files_created,
            side_effect_files_modified: summary.side_effect_files_modified,
            side_effect_files_deleted: summary.side_effect_files_deleted,
            side_effect_files_total: summary.side_effect_files_total,
            side_effect_probe_count: summary.side_effect_probe_count,
            credential_like_side_effects: summary.credential_like_side_effects,
            observed_max_depth: summary.observed_max_depth,
            traversal_profile: summary.traversal_profile.to_owned(),
            max_depth: summary.max_depth,
            max_probes: summary.max_probes,
            min_expected_value: summary.min_expected_value,
            concurrency_limit: summary.concurrency_limit,
            traversal_rounds: summary.traversal_rounds,
            probes_scheduled: summary.probes_scheduled,
            probes_cancelled: summary.probes_cancelled,
            frontier_remaining: summary.frontier_remaining,
            highest_pending_expected_value: summary.highest_pending_expected_value,
            candidates_skipped_by_depth: summary.candidates_skipped_by_depth,
            candidates_skipped_by_convergence: summary.candidates_skipped_by_convergence,
            probes_skipped_by_budget: summary.probes_skipped_by_budget,
            budget_exhausted: summary.budget_exhausted,
            traversal_stop_reason: summary.traversal_stop_reason.to_owned(),
            traversal_complete: summary.traversal_complete,
        },
    };
    let bytes =
        serde_json::to_vec_pretty(&manifest).map_err(CliareError::SerializeMeasurementCache)?;
    fs::write(&path, bytes)
        .await
        .map_err(|source| CliareError::WriteMeasurementCache { path, source })
}

fn bootstrap_probes(target: &TargetFingerprint) -> Vec<ProbeSpec> {
    let target_name = target
        .resolved
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target");
    let invalid_command = bootstrap_invalid_command_token(target_name);
    let invalid_flag = bootstrap_invalid_flag_token(target_name);

    vec![
        ProbeSpec::new(["--help"], ProbeIntent::Help),
        ProbeSpec::new(["-h"], ProbeIntent::Help),
        ProbeSpec::new(["help"], ProbeIntent::Help),
        ProbeSpec::new(["--version"], ProbeIntent::Version),
        ProbeSpec::new(["version"], ProbeIntent::Version),
        ProbeSpec::from_vec(vec![invalid_command], ProbeIntent::InvalidCommand),
        ProbeSpec::from_vec(vec![invalid_flag], ProbeIntent::InvalidFlag),
    ]
}

fn target_binary_name(target: &TargetFingerprint) -> String {
    target
        .resolved
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("target")
        .to_owned()
}

fn invalid_token_seed(binary_name: &str) -> String {
    binary_name.replace('-', "_")
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

fn progress_percent(completed: usize, max_probes: usize) -> f64 {
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::cli::{MeasureArgs, TraversalProfile};
    use crate::evidence::ProbeIntent;
    use crate::fingerprint::TargetFingerprint;

    #[test]
    fn bootstrap_contains_only_generic_safe_probes() {
        let probes = super::bootstrap_probes(&crate::fingerprint::TargetFingerprint {
            requested: "tool".into(),
            resolved: "/tmp/tool".into(),
            binary_sha256: "abc".to_owned(),
            size_bytes: 1,
        });

        assert!(probes.iter().any(|probe| probe.args == ["--help"]));
        assert!(probes.iter().any(|probe| probe.args == ["help"]));
        assert!(
            probes
                .iter()
                .any(|probe| matches!(probe.intent, ProbeIntent::InvalidCommand))
        );
    }

    #[test]
    fn invalid_token_seed_is_shell_token_friendly() {
        assert_eq!(super::invalid_token_seed("my-tool"), "my_tool");
    }

    #[test]
    fn progress_percent_is_probe_budget_bounded_until_finish() {
        assert_eq!(super::progress_percent(0, 5000), 0.0);
        assert_eq!(super::progress_percent(2500, 5000), 50.0);
        assert_eq!(super::progress_percent(5000, 5000), 99.0);
        assert_eq!(super::progress_percent(1, 0), 0.0);
    }

    #[test]
    fn measure_job_ids_are_unique_inside_one_process() {
        let first = super::new_measure_job_id().expect("first job id");
        let second = super::new_measure_job_id().expect("second job id");

        assert_ne!(first, second);
        assert!(first.starts_with("measure-"));
        assert!(second.starts_with("measure-"));
    }

    #[test]
    fn terminal_summary_lists_score_and_artifacts() {
        let summary = super::MeasurementSummary {
            target: TargetFingerprint {
                requested: "tool".into(),
                resolved: "/tmp/tool".into(),
                binary_sha256: "abc".to_owned(),
                size_bytes: 1,
            },
            job_id: Some("measure-test".to_owned()),
            job_log_path: Some(PathBuf::from(".cliare/jobs/measure-test.log")),
            probes_completed: 7,
            evidence_path: PathBuf::from(".cliare/evidence.jsonl"),
            shape_path: PathBuf::from(".cliare/shape.json"),
            command_index_json_path: PathBuf::from(".cliare/command-index.json"),
            command_index_markdown_path: PathBuf::from(".cliare/command-index.md"),
            scorecard_path: PathBuf::from(".cliare/scorecard.json"),
            report_path: PathBuf::from(".cliare/report.md"),
            ci_summary_path: PathBuf::from(".cliare/summary.md"),
            sarif_path: PathBuf::from(".cliare/findings.sarif"),
            junit_path: PathBuf::from(".cliare/junit.xml"),
            issues_markdown_path: PathBuf::from(".cliare/issues.md"),
            issues_json_path: PathBuf::from(".cliare/issues.json"),
            persona_report_count: 7,
            readme_path: PathBuf::from(".cliare/README.md"),
            agent_skill_path: PathBuf::from(".cliare/AGENT_SKILL.md"),
            sandbox_profile: "isolated".to_owned(),
            sandbox_root: PathBuf::from(".cliare/sandbox"),
            sandbox_home: PathBuf::from(".cliare/sandbox/home"),
            sandbox_workdir: PathBuf::from(".cliare/sandbox/cwd"),
            sandbox_env_policy: "cleared_with_allowlist".to_owned(),
            score_total: 82.4,
            score_measured_weight: 0.9,
            score_max_weight: 1.0,
            score_model: "cliare-score-v0".to_owned(),
            score_status: "experimental partial".to_owned(),
            findings: 2,
            commands_precondition_blocked: 1,
            output_contracts_discovered: 1,
            machine_readable_output_contracts: 1,
            output_mode_probes_completed: 1,
            output_mode_parse_successes: 1,
            output_mode_precondition_blocked: 0,
            precondition_blocked_probes: 1,
            auth_required_probes: 1,
            local_context_required_probes: 0,
            fixture_required_probes: 0,
            actionable_precondition_probes: 1,
            precondition_recovery_rate: 1.0,
            side_effect_files_created: 0,
            side_effect_files_modified: 0,
            side_effect_files_deleted: 0,
            side_effect_files_total: 0,
            side_effect_probe_count: 0,
            credential_like_side_effects: 0,
            observed_max_depth: 1,
            traversal_profile: "standard".to_owned(),
            max_depth: 5,
            max_probes: 256,
            min_expected_value: 150,
            concurrency_limit: 4,
            traversal_rounds: 2,
            probes_scheduled: 7,
            probes_cancelled: 0,
            frontier_remaining: 0,
            highest_pending_expected_value: None,
            candidates_skipped_by_depth: 0,
            candidates_skipped_by_convergence: 0,
            probes_skipped_by_budget: 0,
            budget_exhausted: false,
            traversal_stop_reason: "converged".to_owned(),
            traversal_complete: true,
            cache_hit: false,
            runtime_context: crate::context::RuntimeContext::default(),
            suite_root_path: PathBuf::from(".cliare"),
            runtime_context_path: Some(PathBuf::from(".cliare/runtime-context.json")),
            context_suite_path: None,
            context_compare_path: None,
        };

        let text = summary.terminal_summary();

        assert!(text.contains("CLIARE measure complete"));
        assert!(text.contains("score: 82.4/100"));
        assert!(text.contains("cache: miss"));
        assert!(text.contains("job_id: measure-test"));
        assert!(text.contains("progress log: .cliare/jobs/measure-test.log"));
        assert!(text.contains("preconditions:"));
        assert!(text.contains("commands blocked: 1"));
        assert!(text.contains("probes blocked: 1"));
        assert!(text.contains("auth required: 1"));
        assert!(text.contains("local context required: 0"));
        assert!(text.contains("actionable recovery: 1 (100.0%)"));
        assert!(text.contains("output contracts:"));
        assert!(text.contains("machine-readable: 1"));
        assert!(text.contains("blocked: 0"));
        assert!(text.contains("side effects:"));
        assert!(text.contains("file changes: 0"));
        assert!(text.contains("sandbox profile: isolated"));
        assert!(text.contains("env policy: cleared_with_allowlist"));
        assert!(text.contains("runtime context:"));
        assert!(text.contains("profile: single"));
        assert!(text.contains("suite root: .cliare"));
        assert!(text.contains("depth: observed 1 / budget 5"));
        assert!(text.contains("min expected value: 150"));
        assert!(text.contains("concurrency limit: 4"));
        assert!(text.contains("scheduler rounds: 2"));
        assert!(text.contains("probes scheduled: 7"));
        assert!(text.contains("probes cancelled: 0"));
        assert!(text.contains("stop reason: converged"));
        assert!(text.contains("  scorecard: .cliare/scorecard.json"));
        assert!(text.contains("  command index: .cliare/command-index.json"));
        assert!(text.contains("  command index report: .cliare/command-index.md"));
        assert!(text.contains("  report: .cliare/report.md"));
        assert!(text.contains("  ci summary: .cliare/summary.md"));
        assert!(text.contains("  sarif: .cliare/findings.sarif"));
        assert!(text.contains("  junit: .cliare/junit.xml"));
        assert!(text.contains("  issues: .cliare/issues.json"));
        assert!(text.contains("  issue report: .cliare/issues.md"));
        assert!(text.contains("  persona reports: 7 markdown/json pairs"));
        assert!(text.contains("  readme: .cliare/README.md"));
        assert!(text.contains("  agent guide: .cliare/AGENT_SKILL.md"));
        assert!(text.contains("  runtime context: .cliare/runtime-context.json"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn measure_runs_probes_inside_isolated_sandbox() {
        use std::os::unix::fs::PermissionsExt;

        let root = unique_test_dir("sandbox-measure");
        let bin_dir = root.join("bin");
        let out_dir = root.join("out");
        fs::create_dir_all(&bin_dir).expect("creates fixture bin dir");

        let target = bin_dir.join("writes-home");
        fs::write(
            &target,
            r#"#!/bin/sh
case "$HOME" in
  */sandbox/probes/*/home) ;;
  *) echo "unexpected HOME: $HOME" >&2; exit 99 ;;
esac
case "$PWD" in
  */sandbox/probes/*/cwd) ;;
  *) echo "unexpected PWD: $PWD" >&2; exit 98 ;;
esac
printf ok > "$HOME/home-marker"
printf ok > "$PWD/cwd-marker"
cat <<'EOF'
Usage: writes-home [COMMAND]

Commands:
  alpha  Sample command

Options:
  --help  Print help
EOF
"#,
        )
        .expect("writes fixture cli");
        let mut permissions = fs::metadata(&target)
            .expect("reads fixture metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&target, permissions).expect("marks fixture executable");

        let summary = super::measure(MeasureArgs {
            target,
            out: out_dir.clone(),
            timeout_ms: 5_000,
            output_limit_bytes: 16 * 1024,
            profile: TraversalProfile::Quick,
            execution_mode: crate::sandbox::SandboxProfile::Isolated,
            max_depth: Some(1),
            max_probes: Some(1),
            min_expected_value: Some(1),
            concurrency: None,
            context: None,
            context_name: None,
            auth_state: None,
            local_context_state: None,
            fixture_state: None,
            network_state: None,
            runtime_dependency_state: None,
            context_workdir: None,
            refresh: true,
            detach: false,
            detached_worker: false,
            job_id: None,
        })
        .await
        .expect("measurement succeeds");

        assert_eq!(summary.sandbox_profile, "isolated");
        assert_eq!(summary.sandbox_env_policy, "cleared_with_allowlist");
        assert!(
            summary
                .job_id
                .as_ref()
                .is_some_and(|id| id.starts_with("measure-"))
        );
        let job_log_path = summary
            .job_log_path
            .as_ref()
            .expect("fresh measurement exposes progress log");
        assert!(job_log_path.is_file());
        let progress = fs::read_to_string(job_log_path).expect("reads progress log");
        assert!(progress.contains("job_created"));
        assert!(progress.contains("scheduled probe=p_000001"));
        assert!(progress.contains("completed probe=p_000001"));
        assert!(progress.contains("persona_reports_written personas=7"));
        assert!(progress.contains("100.0% complete"));
        let current = fs::read_to_string(out_dir.join("jobs/current"))
            .expect("reads current progress pointer");
        assert!(current.contains("job_id=measure-"));
        assert!(current.contains("tail=tail -f"));
        assert!(out_dir.join("issues.json").is_file());
        assert!(out_dir.join("issues.md").is_file());
        for persona in crate::report::Persona::all() {
            assert!(
                out_dir
                    .join(format!("persona-{}.json", persona.label()))
                    .is_file()
            );
            assert!(
                out_dir
                    .join(format!("persona-{}.md", persona.label()))
                    .is_file()
            );
        }
        assert_eq!(
            summary.persona_report_count,
            crate::report::Persona::all().len()
        );
        assert!(
            out_dir
                .join("sandbox/probes/p_000001/home/home-marker")
                .is_file()
        );
        assert!(
            out_dir
                .join("sandbox/probes/p_000001/cwd/cwd-marker")
                .is_file()
        );
        assert!(!root.join("home-marker").exists());

        let _ = fs::remove_dir_all(root);
    }

    fn unique_test_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cliare-{name}-{}-{nonce}", std::process::id()))
    }
}
