use std::collections::BTreeMap;

use super::formulas::total_score;
use super::model::{DimensionScore, DimensionStatus, SandboxScoreContext, ScoreRunContext};
use super::{Dimension, report, scorecard};
use cliare_context::RuntimeContext;
use cliare_evidence::{ProbeIntent, ProcessCompleted, ProcessStatus};
use cliare_inference::score_model::ScoreModelSpec;
use cliare_runtime::fingerprint::TargetFingerprint;
use cliare_runtime::process::OutputCapture;
use cliare_runtime::sandbox::{SideEffectSummary, SnapshotLimits};
use cliare_shape::observation::ShapeObservation;

#[test]
fn runtime_confirmation_improves_discovery_score() {
    let target = target();
    let weak = vec![observation(
        "e_000003",
        ProbeIntent::Help,
        vec![],
        "Commands:\n  measure  Run probes\n",
        Some(0),
    )];
    let strong = vec![
        observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Commands:\n  measure  Run probes\n",
            Some(0),
        ),
        observation(
            "e_000005",
            ProbeIntent::Help,
            vec!["measure".to_owned()],
            "Usage: cliare measure <TARGET>\n\nOptions:\n  --out <DIR>  Output directory\n",
            Some(0),
        ),
    ];

    let weak_score = scorecard(target.clone(), &weak, ScoreRunContext::default());
    let strong_score = scorecard(target, &strong, ScoreRunContext::default());

    assert!(
        dimension_score(&strong_score, Dimension::Discovery)
            > dimension_score(&weak_score, Dimension::Discovery)
    );
    assert!(strong_score.score.shape_confidence > weak_score.score.shape_confidence);
    assert_eq!(
        strong_score.score.maintainer_readiness,
        strong_score.score.total
    );
}

#[test]
fn invalid_flag_rejection_improves_recovery_score() {
    let target = target();
    let observations = vec![
        observation(
            "e_000003",
            ProbeIntent::Help,
            vec!["measure".to_owned()],
            "Usage: cliare measure <TARGET>\n\nOptions:\n  --out <DIR>  Output directory\n",
            Some(0),
        ),
        observation(
            "e_000005",
            ProbeIntent::InvalidFlag,
            vec!["measure".to_owned()],
            "error: unexpected argument",
            Some(2),
        ),
    ];

    let scorecard = scorecard(target, &observations, ScoreRunContext::default());

    assert_eq!(dimension_score(&scorecard, Dimension::Recovery), 100.0);
}

#[test]
fn invalid_probe_finding_uses_invalid_rejection_rate_not_total_recovery_score() {
    let observations = vec![
        observation(
            "e_000003",
            ProbeIntent::InvalidFlag,
            vec!["measure".to_owned()],
            "error: unexpected argument",
            Some(2),
        ),
        observation(
            "e_000005",
            ProbeIntent::Help,
            vec!["project".to_owned()],
            "owner is required when not running interactively",
            Some(1),
        ),
    ];

    let scorecard = scorecard(target(), &observations, ScoreRunContext::default());

    assert_eq!(scorecard.coverage.precondition_blocked_probes, 1);
    assert_eq!(scorecard.coverage.actionable_precondition_probes, 0);
    assert_eq!(dimension_score(&scorecard, Dimension::Recovery), 70.0);
    assert!(
        scorecard
            .findings
            .iter()
            .all(|finding| { finding.id != "finding.recovery.invalid_probe_acceptance" })
    );
    assert!(
        scorecard
            .findings
            .iter()
            .any(|finding| { finding.id == "finding.precondition.runtime_blocked" })
    );
}

#[test]
fn invalid_probe_finding_reports_accepted_invalid_probes() {
    let observations = vec![observation(
        "e_000003",
        ProbeIntent::InvalidFlag,
        vec!["measure".to_owned()],
        "ignored unknown flag",
        Some(0),
    )];

    let scorecard = scorecard(target(), &observations, ScoreRunContext::default());

    assert!(scorecard.findings.iter().any(|finding| {
        finding.id == "finding.recovery.invalid_probe_acceptance"
            && finding.detail == "0 of 1 invalid probes rejected with nonzero exit status."
    }));
}

#[test]
fn auth_blocked_probes_are_reported_as_preconditions_not_recovery_success() {
    let observations = vec![
        observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Commands:\n  model  Track AI model identity\n",
            Some(0),
        ),
        observation(
            "e_000005",
            ProbeIntent::Help,
            vec!["model".to_owned()],
            "error: rote requires login\n\nrun rote login",
            Some(77),
        ),
        observation(
            "e_000007",
            ProbeIntent::InvalidFlag,
            vec!["model".to_owned()],
            "error: rote requires login\n\nrun rote login",
            Some(77),
        ),
    ];

    let scorecard = scorecard(target(), &observations, ScoreRunContext::default());

    assert_eq!(scorecard.coverage.commands_runtime_confirmed, 0);
    assert_eq!(scorecard.coverage.commands_precondition_blocked, 1);
    assert_eq!(scorecard.coverage.precondition_blocked_probes, 2);
    assert_eq!(scorecard.coverage.auth_required_probes, 2);
    assert_eq!(dimension_score(&scorecard, Dimension::Recovery), 0.0);
    assert!(scorecard.findings.iter().any(|finding| {
        finding.id == "finding.precondition.runtime_blocked"
            && finding.title == "Some probes were blocked by runtime preconditions"
    }));

    let report = report::render(&scorecard);
    assert!(report.contains("- Commands precondition-blocked: `1`"));
    assert!(report.contains("- Precondition-blocked probes: `2`"));
    assert!(report.contains("- Auth-required probes: `2`"));
}

#[test]
fn actionable_precondition_diagnostics_improve_recovery_score() {
    let observations = vec![observation(
        "e_000005",
        ProbeIntent::Help,
        vec!["stats".to_owned()],
        "error: not in a workspace directory\n\nFix:\n  cliare init demo\n  cd workspaces/demo\n\nhint: or list existing: 'cliare ls'\n",
        Some(1),
    )];

    let scorecard = scorecard(target(), &observations, ScoreRunContext::default());

    assert_eq!(scorecard.coverage.precondition_blocked_probes, 1);
    assert_eq!(scorecard.coverage.local_context_required_probes, 1);
    assert_eq!(scorecard.coverage.actionable_precondition_probes, 1);
    assert_eq!(scorecard.coverage.precondition_recovery_rate, 1.0);
    assert_eq!(dimension_score(&scorecard, Dimension::Recovery), 100.0);

    let report = report::render(&scorecard);
    assert!(report.contains("- Local-context-required probes: `1`"));
    assert!(report.contains("- Actionable precondition diagnostics: `1`"));
    assert!(report.contains("- Precondition recovery rate: `100.0%`"));
}

#[test]
fn fixture_required_output_probes_are_not_output_parse_failures() {
    let observations = vec![
        observation(
            "e_000003",
            ProbeIntent::Help,
            vec!["project".to_owned(), "item-list".to_owned()],
            "List items\n\nUSAGE\n  acmectl project item-list [<number>] [flags]\n\nFLAGS\n  --format string  Output format: {json}\n",
            Some(0),
        ),
        observation(
            "e_000005",
            ProbeIntent::OutputJson,
            vec!["project".to_owned(), "item-list".to_owned()],
            "owner is required when not running interactively",
            Some(1),
        ),
    ];

    let scorecard = scorecard(target(), &observations, ScoreRunContext::default());

    assert_eq!(scorecard.coverage.precondition_blocked_probes, 1);
    assert_eq!(scorecard.coverage.fixture_required_probes, 1);
    assert_eq!(scorecard.coverage.output_mode_precondition_blocked, 1);
    assert!(
        scorecard
            .findings
            .iter()
            .all(|finding| { finding.id != "finding.output.unparseable_mode" })
    );

    let report = report::render(&scorecard);
    assert!(report.contains("- Fixture-required probes: `1`"));
}

#[test]
fn report_renders_scorecard_summary_and_unmeasured_dimensions() {
    let scorecard = scorecard(
        target(),
        &[observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Commands:\n  measure  Run probes\n",
            Some(0),
        )],
        ScoreRunContext {
            traversal_profile: "standard",
            max_depth: 5,
            max_probes: 256,
            min_expected_value: 150,
            concurrency_limit: 4,
            traversal_rounds: 1,
            probes_scheduled: 1,
            probes_cancelled: 0,
            frontier_remaining: 0,
            highest_pending_expected_value: None,
            candidates_skipped_by_depth: 0,
            candidates_skipped_by_convergence: 0,
            sandbox: test_sandbox(),
            runtime_context: RuntimeContext::default(),
        },
    );

    let report = report::render(&scorecard);

    assert!(report.contains("# CLIARE Report"));
    assert!(report.contains("- Maintainer readiness:"));
    assert!(report.contains("- Harness shape confidence:"));
    assert!(report.contains("| output | 0 | 0.05 | measured |"));
    assert!(report.contains("experimental partial"));
    assert!(report.contains("- Output contracts discovered: `0`"));
    assert!(report.contains("- Help-text probes: `1`"));
    assert!(report.contains("- Help-text probes with extracted shape: `1`"));
    assert!(report.contains("- Parser extraction rate: `100.0%`"));
    assert!(report.contains("## Runtime Context"));
    assert!(report.contains("- Profile: `single`"));
    assert!(report.contains("- Traversal profile: `standard`"));
    assert!(report.contains("- Depth budget: `5`"));
    assert!(report.contains("- Minimum expected probe value: `150`"));
    assert!(report.contains("- Concurrency limit: `4`"));
    assert!(report.contains("- Scheduler rounds: `1`"));
    assert!(report.contains("- Probes scheduled: `1`"));
    assert!(report.contains("- Probes cancelled: `0`"));
    assert!(report.contains("- Sandbox profile: `isolated`"));
    assert!(report.contains("- Environment policy: `cleared_with_allowlist`"));
    assert!(report.contains("- Budget exhausted: `false`"));
    assert!(report.contains("- Traversal stop reason: `converged`"));
    assert!(report.contains("- Traversal complete: `true`"));
}

#[test]
fn scorecard_reports_budget_pressure_without_lowering_score() {
    let observations = vec![
        observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Commands:\n  alpha  First level\n",
            Some(0),
        ),
        observation(
            "e_000004",
            ProbeIntent::Help,
            vec!["alpha".to_owned()],
            "Commands:\n  beta  Second level\n",
            Some(0),
        ),
    ];

    let scorecard = scorecard(
        target(),
        &observations,
        ScoreRunContext {
            traversal_profile: "quick",
            max_depth: 1,
            max_probes: 2,
            min_expected_value: 300,
            concurrency_limit: 2,
            traversal_rounds: 1,
            probes_scheduled: 2,
            probes_cancelled: 0,
            frontier_remaining: 3,
            highest_pending_expected_value: Some(400),
            candidates_skipped_by_depth: 1,
            candidates_skipped_by_convergence: 2,
            ..ScoreRunContext::default()
        },
    );

    assert_eq!(scorecard.coverage.observed_max_depth, 2);
    assert_eq!(scorecard.coverage.traversal_profile, "quick");
    assert_eq!(scorecard.coverage.max_depth, 1);
    assert_eq!(scorecard.coverage.max_probes, 2);
    assert_eq!(scorecard.coverage.min_expected_value, 300);
    assert_eq!(scorecard.coverage.concurrency_limit, 2);
    assert_eq!(scorecard.coverage.traversal_rounds, 1);
    assert_eq!(scorecard.coverage.probes_scheduled, 2);
    assert_eq!(scorecard.coverage.probes_cancelled, 0);
    assert_eq!(scorecard.coverage.frontier_remaining, 3);
    assert_eq!(scorecard.coverage.highest_pending_expected_value, Some(400));
    assert_eq!(scorecard.coverage.candidates_skipped_by_depth, 1);
    assert_eq!(scorecard.coverage.candidates_skipped_by_convergence, 2);
    assert_eq!(scorecard.coverage.probes_skipped_by_budget, 3);
    assert!(scorecard.coverage.budget_exhausted);
    assert_eq!(
        scorecard.coverage.traversal_stop_reason,
        super::TraversalStopReason::ProbeBudgetExhausted
    );
    assert!(!scorecard.coverage.traversal_complete);
}

#[test]
fn scorecard_classifies_depth_budget_stop_before_convergence() {
    let scorecard = scorecard(
        target(),
        &[observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Commands:\n  alpha  First level\n",
            Some(0),
        )],
        ScoreRunContext {
            traversal_profile: "quick",
            max_depth: 1,
            max_probes: 64,
            min_expected_value: 300,
            frontier_remaining: 0,
            highest_pending_expected_value: None,
            candidates_skipped_by_depth: 2,
            candidates_skipped_by_convergence: 0,
            ..ScoreRunContext::default()
        },
    );

    assert_eq!(
        scorecard.coverage.traversal_stop_reason,
        super::TraversalStopReason::DepthBudgetExhausted
    );
    assert!(!scorecard.coverage.traversal_complete);
}

#[test]
fn scorecard_classifies_empty_frontier_without_claims() {
    let scorecard = scorecard(
        target(),
        &[],
        ScoreRunContext {
            traversal_profile: "standard",
            max_depth: 5,
            max_probes: 256,
            min_expected_value: 150,
            frontier_remaining: 0,
            highest_pending_expected_value: None,
            candidates_skipped_by_depth: 0,
            candidates_skipped_by_convergence: 0,
            ..ScoreRunContext::default()
        },
    );

    assert_eq!(
        scorecard.coverage.traversal_stop_reason,
        super::TraversalStopReason::FrontierExhausted
    );
    assert!(scorecard.coverage.traversal_complete);
}

#[test]
fn partial_total_is_normalized_by_declared_weight() {
    let mut subscores = BTreeMap::new();
    subscores.insert(
        Dimension::Discovery,
        DimensionScore {
            score: Some(100.0),
            weight: 0.35,
            status: DimensionStatus::Measured,
            rationale: "measured".to_owned(),
        },
    );
    subscores.insert(
        Dimension::Grammar,
        DimensionScore {
            score: None,
            weight: 0.65,
            status: DimensionStatus::NotMeasured,
            rationale: "not measured".to_owned(),
        },
    );

    let score = total_score(&subscores, ScoreModelSpec::bundled());

    assert_eq!(score.total, 35.0);
    assert_eq!(score.maintainer_readiness, 35.0);
    assert_eq!(score.shape_confidence, 35.0);
    assert!(score.total <= 100.0);
    assert_eq!(score.measured_weight, 0.35);
    assert_eq!(score.max_weight, 1.0);
}

#[test]
fn host_mode_marks_safety_unmeasured() {
    let scorecard = scorecard(
        target(),
        &[observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Commands:\n  inspect  Inspect state\n",
            Some(0),
        )],
        ScoreRunContext {
            sandbox: SandboxScoreContext {
                profile: "host",
                root: "/tmp/project".into(),
                home: "/Users/example".into(),
                workdir: "/tmp/project".into(),
                env_policy: "inherited",
                snapshot_limits: SnapshotLimits::default(),
                hostile_binary_containment: false,
            },
            ..ScoreRunContext::default()
        },
    );

    let safety = &scorecard.subscores[&Dimension::Safety];
    assert_eq!(safety.score, None);
    assert!(matches!(safety.status, DimensionStatus::NotMeasured));
    assert!(
        scorecard
            .findings
            .iter()
            .any(|finding| { finding.id == "finding.safety.side_effects_unobserved_in_host_mode" })
    );
}

#[test]
fn scorecard_embeds_bundled_model_provenance() {
    let scorecard = scorecard(target(), &[], ScoreRunContext::default());

    assert_eq!(scorecard.model.name, "cliare-score-v0");
    assert_eq!(scorecard.model.sha256.len(), 64);
    assert_eq!(scorecard.model.normalization, "declared_weight");
    assert_eq!(scorecard.score.model, scorecard.model.name);
}

#[test]
fn extraction_limited_help_is_reported_as_measurement_ambiguity() {
    let odd_layout = "\
AYUDA
  publicar    publicar servicio
  borrar      borrar servicio
";
    let scorecard = scorecard(
        target(),
        &[
            observation("e_000003", ProbeIntent::Help, vec![], odd_layout, Some(0)),
            observation(
                "e_000004",
                ProbeIntent::Help,
                vec!["publicar".to_owned()],
                odd_layout,
                Some(0),
            ),
        ],
        ScoreRunContext::default(),
    );

    assert_eq!(scorecard.coverage.help_text_probes, 2);
    assert_eq!(scorecard.coverage.help_text_probes_with_shape, 0);
    assert_eq!(scorecard.coverage.help_text_probes_without_shape, 2);
    assert_eq!(scorecard.coverage.parser_extraction_rate, 0.0);
    assert!(scorecard.findings.iter().any(|finding| {
        finding.id == "finding.discovery.extraction_limited"
            && finding.title == "Help text was not converted into reliable command shape"
    }));
}

fn dimension_score(scorecard: &super::Scorecard, dimension: Dimension) -> f64 {
    scorecard.subscores[&dimension]
        .score
        .expect("dimension is measured")
}

fn target() -> TargetFingerprint {
    TargetFingerprint {
        requested: "cliare".into(),
        resolved: "/tmp/cliare".into(),
        binary_sha256: "abc".to_owned(),
        size_bytes: 1,
    }
}

fn test_sandbox() -> SandboxScoreContext {
    SandboxScoreContext {
        profile: "isolated",
        root: "/tmp/cliare/sandbox".into(),
        home: "/tmp/cliare/sandbox/home".into(),
        workdir: "/tmp/cliare/sandbox/cwd".into(),
        env_policy: "cleared_with_allowlist",
        snapshot_limits: SnapshotLimits::default(),
        hostile_binary_containment: false,
    }
}

fn observation(
    evidence_id: &str,
    intent: ProbeIntent,
    path: Vec<String>,
    stdout: &str,
    exit_code: Option<i32>,
) -> ShapeObservation {
    ShapeObservation {
        evidence_id: evidence_id.to_owned(),
        intent,
        path,
        process: ProcessCompleted {
            probe_id: "p_000001".to_owned(),
            argv: vec!["cliare".to_owned(), "--help".to_owned()],
            status: ProcessStatus::Exited { code: exit_code },
            duration_ms: 1,
            stdout: output(stdout),
            stderr: output(""),
            side_effects: SideEffectSummary::default(),
        },
    }
}

fn output(text: &str) -> OutputCapture {
    OutputCapture {
        sha256: "unused".to_owned(),
        bytes: text.len(),
        retained_bytes: text.len(),
        truncated: false,
        text: Some(text.to_owned()),
    }
}
