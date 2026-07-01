use crate::artifact_guide;
use crate::artifacts::{CONTEXT_COMPARE_MD, CONTEXT_SUITE_JSON, MeasurementArtifactPaths};
use crate::ci;
use crate::claims::ClaimSet;
use crate::cli::MeasureArgs;
use crate::context::{self, RuntimeContext, RuntimeContextInput};
use crate::error::Result;
use crate::evidence::{EvidenceKind, EvidenceWriter, RunFinished, RunStarted};
use crate::fingerprint::fingerprint_target;
use crate::planner::{ConvergencePolicy, DeterministicPlanner, ProbePlanner};
use crate::process::TargetProcess;
use crate::report;
use crate::sandbox::Sandbox;
use crate::score::{self, ScoreRunContext};
use crate::shape;

use super::MEASUREMENT_CHECKPOINT_JSON;
use super::bootstrap::{bootstrap_probes, invalid_token_seed, target_binary_name};
use super::cache::{
    cached_summary, cleanup_abandoned_in_progress_files, remove_stale_cache_manifest,
    write_cache_manifest,
};
use super::checkpoint::{
    CheckpointWriter, TraversalResume, read_resume_checkpoint, remove_measurement_checkpoint,
};
use super::profile::{ProbeProfile, ResolvedProbeProfile};
use super::progress::ProgressLog;
use super::summary::{MeasurementFacts, MeasurementSummary};
use super::traversal::{TraversalContext, TraversalLimits, run_traversal};

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
    let snapshot_limits = args.snapshot_limits();
    let resolved_profile = ResolvedProbeProfile {
        max_depth,
        max_probes,
        min_expected_value,
        concurrency_limit,
        snapshot_limits,
    };
    let profile = ProbeProfile::from_args(
        &args,
        resolved_profile,
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
            summary.context_suite_path = Some(args.out.join(CONTEXT_SUITE_JSON));
            summary.context_compare_path = Some(args.out.join(CONTEXT_COMPARE_MD));
        }
        return Ok(summary);
    }

    remove_stale_cache_manifest(&artifact_dir).await?;
    let resume_checkpoint = if args.refresh {
        remove_measurement_checkpoint(&artifact_dir).await?;
        None
    } else {
        read_resume_checkpoint(&artifact_dir, &target, &profile).await?
    };
    if resume_checkpoint.is_none() {
        remove_measurement_checkpoint(&artifact_dir).await?;
        cleanup_abandoned_in_progress_files(&artifact_dir).await?;
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

    let sandbox = Sandbox::create_with_profile_and_limits(
        &artifact_dir,
        runtime_context.workdir.as_deref(),
        args.execution_mode,
        snapshot_limits,
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
    let mut evidence = if let Some(checkpoint) = &resume_checkpoint {
        progress
            .message(
                checkpoint.probes_completed,
                format!(
                    "resume_checkpoint_loaded probes_completed={} evidence={}",
                    checkpoint.probes_completed,
                    checkpoint.evidence_path.display()
                ),
            )
            .await?;
        EvidenceWriter::resume(
            &artifact_dir,
            checkpoint.evidence_path.clone(),
            checkpoint.next_event_id,
        )
        .await?
    } else {
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
        evidence
    };

    let binary_name = target_binary_name(&target);
    let mut planner = DeterministicPlanner::with_policy(
        max_depth,
        ConvergencePolicy::new(min_expected_value),
        invalid_token_seed(&binary_name),
    );
    let resume_state = resume_checkpoint.map(TraversalResume::from);
    if let Some(resume) = &resume_state {
        planner.mark_seen(resume.completed_probes());
    }
    planner.seed(bootstrap_probes(&target));
    if let Some(resume) = &resume_state {
        let observations = resume.observations();
        let claims = ClaimSet::from_observations(&binary_name, &observations);
        planner.extend_from_claims(&claims);
    }
    let process = TargetProcess::new(
        target.resolved.clone(),
        args.timeout(),
        args.output_limit_bytes,
    );
    let evidence_path = evidence.path().to_path_buf();
    let traversal = match run_traversal(TraversalContext {
        target: &target,
        sandbox: &sandbox,
        process: &process,
        evidence: &mut evidence,
        progress: &mut progress,
        planner: &mut planner,
        binary_name: &binary_name,
        checkpoint: CheckpointWriter {
            path: artifact_dir.join(MEASUREMENT_CHECKPOINT_JSON),
            target: target.clone(),
            profile: profile.clone(),
            evidence_path,
        },
        resume: resume_state,
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
    evidence.commit().await?;
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
                "score_artifacts_written score={:.0} scorecard={}",
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

    let paths = MeasurementArtifactPaths::from_dir(&artifact_dir);
    let mut summary = MeasurementSummary {
        target,
        job_id: Some(progress.job_id().to_owned()),
        job_log_path: Some(progress.path().to_path_buf()),
        evidence_path: paths.evidence,
        shape_path: paths.shape,
        command_index_json_path: paths.command_index_json,
        command_index_markdown_path: paths.command_index_markdown,
        scorecard_path: score_artifacts.scorecard_path,
        report_path: score_artifacts.report_path,
        ci_summary_path: ci_artifacts.summary_path,
        sarif_path: ci_artifacts.sarif_path,
        junit_path: ci_artifacts.junit_path,
        issues_markdown_path: persona_artifacts.issues_markdown_path,
        issues_json_path: persona_artifacts.issues_json_path,
        persona_report_count,
        readme_path: paths.readme,
        agent_skill_path: paths.agent_skill,
        facts: MeasurementFacts {
            probes_completed: traversal.probes_completed,
            sandbox_profile: score_artifacts.sandbox_profile.to_owned(),
            sandbox_root: score_artifacts.sandbox_root,
            sandbox_home: score_artifacts.sandbox_home,
            sandbox_workdir: score_artifacts.sandbox_workdir,
            sandbox_env_policy: score_artifacts.sandbox_env_policy.to_owned(),
            snapshot_max_files: score_artifacts.snapshot_max_files,
            snapshot_max_directories: score_artifacts.snapshot_max_directories,
            snapshot_max_hash_bytes: score_artifacts.snapshot_max_hash_bytes,
            hostile_binary_containment: score_artifacts.hostile_binary_containment,
            score_total: score_artifacts.total,
            score_maintainer_readiness: score_artifacts.maintainer_readiness,
            score_shape_confidence: score_artifacts.shape_confidence,
            score_measured_weight: score_artifacts.measured_weight,
            score_max_weight: score_artifacts.max_weight,
            score_model: score_artifacts.model.to_owned(),
            score_status: score_artifacts.status.to_owned(),
            findings: score_artifacts.findings,
            commands_precondition_blocked: score_artifacts.commands_precondition_blocked,
            help_text_probes: score_artifacts.help_text_probes,
            help_text_probes_with_shape: score_artifacts.help_text_probes_with_shape,
            help_text_probes_without_shape: score_artifacts.help_text_probes_without_shape,
            help_text_probes_not_recognized: score_artifacts.help_text_probes_not_recognized,
            parser_extraction_rate: score_artifacts.parser_extraction_rate,
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
            side_effect_scan_truncated: score_artifacts.side_effect_scan_truncated,
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
        },
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
    write_cache_manifest(&artifact_dir, &summary, profile, progress.job_id()).await?;
    progress
        .message(traversal.probes_completed, "measure_cache_written")
        .await?;
    remove_measurement_checkpoint(&artifact_dir).await?;
    if runtime_context.is_context_suite_measurement() {
        let _ = context::refresh_context_suite(&args.out).await?;
        summary.context_suite_path = Some(args.out.join(CONTEXT_SUITE_JSON));
        summary.context_compare_path = Some(args.out.join(CONTEXT_COMPARE_MD));
        progress
            .message(traversal.probes_completed, "context_suite_written")
            .await?;
    }
    progress.finished(&summary).await?;

    Ok(summary)
}
