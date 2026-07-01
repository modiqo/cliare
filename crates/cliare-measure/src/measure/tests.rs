use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cli::{MeasureArgs, TraversalProfile};
use crate::evidence::ProbeIntent;
use crate::fingerprint::TargetFingerprint;
use crate::sandbox::SnapshotLimits;

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

#[tokio::test]
async fn fresh_measurement_removes_stale_cache_manifest() {
    let root = unique_test_dir("measure-stale-cache");
    fs::create_dir_all(&root).expect("creates cache test directory");
    let cache_path = root.join("measure-cache.json");
    fs::write(&cache_path, "{}").expect("writes stale cache");

    super::remove_stale_cache_manifest(&root)
        .await
        .expect("stale cache is removed");

    assert!(!cache_path.exists());
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn fresh_measurement_removes_abandoned_in_progress_evidence_logs() {
    let root = unique_test_dir("measure-in-progress-cleanup");
    fs::create_dir_all(&root).expect("creates cleanup test directory");
    let abandoned = root.join(format!(
        "{}dead-worker",
        crate::evidence::EVIDENCE_IN_PROGRESS_PREFIX
    ));
    let keep = root.join("evidence.jsonl");
    fs::write(&abandoned, "partial").expect("writes abandoned evidence");
    fs::write(&keep, "committed").expect("writes committed evidence");

    super::cleanup_abandoned_in_progress_files(&root)
        .await
        .expect("abandoned in-progress evidence is removed");

    assert!(!abandoned.exists());
    assert!(keep.exists());
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn artifact_digests_change_when_required_artifact_changes() {
    let root = unique_test_dir("measure-artifact-digests");
    fs::create_dir_all(&root).expect("creates digest test directory");
    for (index, name) in super::REQUIRED_MEASUREMENT_FILES.iter().enumerate() {
        fs::write(root.join(name), format!("artifact-{index}")).expect("writes required artifact");
    }

    let first = super::artifact_digests(&root)
        .await
        .expect("first artifact digests are computed");
    fs::write(root.join(super::REQUIRED_MEASUREMENT_FILES[0]), "changed")
        .expect("changes required artifact");
    let second = super::artifact_digests(&root)
        .await
        .expect("second artifact digests are computed");

    assert_eq!(first.len(), super::REQUIRED_MEASUREMENT_FILES.len());
    assert!(first.iter().all(|digest| !digest.sha256.is_empty()));
    assert_ne!(first, second);
    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn resume_checkpoint_requires_matching_profile_target_and_evidence_log() {
    let root = unique_test_dir("measure-checkpoint");
    fs::create_dir_all(&root).expect("creates checkpoint test directory");
    let target = TargetFingerprint {
        requested: "tool".into(),
        resolved: "/tmp/tool".into(),
        binary_sha256: "abc".to_owned(),
        size_bytes: 1,
    };
    let profile = super::ProbeProfile {
        traversal_profile: TraversalProfile::Quick,
        sandbox_profile: "isolated".to_owned(),
        runtime_context: crate::context::RuntimeContext::default(),
        timeout_ms: 1_000,
        output_limit_bytes: 1024,
        max_depth: 1,
        max_probes: 2,
        min_expected_value: 3,
        concurrency_limit: 1,
        snapshot_limits: SnapshotLimits::new(4, 5, 6),
    };
    let evidence_path = root.join(format!(
        "{}live-worker",
        crate::evidence::EVIDENCE_IN_PROGRESS_PREFIX
    ));
    fs::write(&evidence_path, "").expect("writes resumable evidence log");
    let checkpoint = super::MeasurementCheckpoint {
        schema_version: super::MEASUREMENT_CHECKPOINT_SCHEMA_VERSION.to_owned(),
        cliare_version: env!("CARGO_PKG_VERSION").to_owned(),
        engine: super::MEASUREMENT_ENGINE.to_owned(),
        target: target.clone(),
        profile: profile.clone(),
        evidence_path: evidence_path.clone(),
        next_event_id: 7,
        probes_scheduled: 0,
        probes_completed: 0,
        rounds: 0,
        completed: Vec::new(),
    };
    fs::write(
        root.join(super::MEASUREMENT_CHECKPOINT_JSON),
        serde_json::to_vec(&checkpoint).expect("serializes checkpoint"),
    )
    .expect("writes checkpoint");

    let loaded = super::read_resume_checkpoint(&root, &target, &profile)
        .await
        .expect("checkpoint read succeeds")
        .expect("matching checkpoint is accepted");
    assert_eq!(loaded.next_event_id, 7);

    let stale_profile = super::ProbeProfile {
        max_probes: 99,
        ..profile.clone()
    };
    assert!(
        super::read_resume_checkpoint(&root, &target, &stale_profile)
            .await
            .expect("stale checkpoint read succeeds")
            .is_none()
    );

    fs::remove_file(&evidence_path).expect("removes evidence log");
    assert!(
        super::read_resume_checkpoint(&root, &target, &profile)
            .await
            .expect("missing evidence read succeeds")
            .is_none()
    );
    let _ = fs::remove_dir_all(root);
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
        facts: super::MeasurementFacts {
            probes_completed: 7,
            sandbox_profile: "isolated".to_owned(),
            sandbox_root: PathBuf::from(".cliare/sandbox"),
            sandbox_home: PathBuf::from(".cliare/sandbox/home"),
            sandbox_workdir: PathBuf::from(".cliare/sandbox/cwd"),
            sandbox_env_policy: "cleared_with_allowlist".to_owned(),
            snapshot_max_files: 10_000,
            snapshot_max_directories: 2_000,
            snapshot_max_hash_bytes: 64 * 1024 * 1024,
            hostile_binary_containment: false,
            score_total: 82.4,
            score_measured_weight: 0.9,
            score_max_weight: 1.0,
            score_model: "cliare-score-v0".to_owned(),
            score_status: "experimental partial".to_owned(),
            findings: 2,
            commands_precondition_blocked: 1,
            help_text_probes: 3,
            help_text_probes_with_shape: 2,
            help_text_probes_without_shape: 1,
            help_text_probes_not_recognized: 0,
            parser_extraction_rate: 2.0 / 3.0,
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
            side_effect_scan_truncated: false,
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
        },
        cache_hit: false,
        runtime_context: crate::context::RuntimeContext::default(),
        suite_root_path: PathBuf::from(".cliare"),
        runtime_context_path: Some(PathBuf::from(".cliare/runtime-context.json")),
        context_suite_path: None,
        context_compare_path: None,
    };

    let text = summary.terminal_summary();

    assert!(text.contains("CLIARE measure complete"));
    assert!(text.contains("score: 82/100"));
    assert!(text.contains("cache: miss"));
    assert!(text.contains("job_id: measure-test"));
    assert!(text.contains("progress log: .cliare/jobs/measure-test.log"));
    assert!(text.contains("preconditions:"));
    assert!(text.contains("commands blocked: 1"));
    assert!(text.contains("probes blocked: 1"));
    assert!(text.contains("auth required: 1"));
    assert!(text.contains("local context required: 0"));
    assert!(text.contains("actionable recovery: 1 (100.0%)"));
    assert!(text.contains("extraction:"));
    assert!(text.contains("help-text probes: 3"));
    assert!(text.contains("with extracted shape: 2"));
    assert!(text.contains("without extracted shape: 1"));
    assert!(text.contains("not recognized as help-like: 0"));
    assert!(text.contains("parser extraction rate: 66.7%"));
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

    let bytes = serde_json::to_vec(&summary.facts).expect("serializes measurement facts");
    let decoded: super::MeasurementFacts =
        serde_json::from_slice(&bytes).expect("deserializes measurement facts");

    assert_eq!(decoded, summary.facts);
    assert_eq!(decoded.help_text_probes, 3);
    assert_eq!(decoded.help_text_probes_with_shape, 2);
    assert_eq!(decoded.help_text_probes_without_shape, 1);
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
    assert!(progress.contains(
        "progress_formula: shown_percent = min(completed / max_probes * 100, 99.0) until complete."
    ));
    assert!(progress.contains(
        "progress_example: if completed=529 and max_probes=5000, shown_percent = 529 / 5000 * 100 = 10.58%, logged as 10.6%."
    ));
    assert!(progress.contains("job_created"));
    assert!(progress.contains("scheduled probe=p_000001"));
    assert!(progress.contains("completed probe=p_000001"));
    assert!(progress.contains("persona_reports_written personas=7"));
    assert!(progress.contains("100.0% complete"));
    let current =
        fs::read_to_string(out_dir.join("jobs/current")).expect("reads current progress pointer");
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
