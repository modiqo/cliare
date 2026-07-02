use std::collections::BTreeMap;

use super::formulas::total_score;
use super::model::{
    AgentNavigationCapability, AgentNavigationMetricStatus, DimensionScore, DimensionStatus,
    SandboxScoreContext, ScoreRunContext,
};
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
fn scorecard_reports_agent_navigation_evidence_metrics() {
    let observations = vec![
        observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Usage: cliare <COMMAND>\n\nCommands:\n  measure  Run probes\n",
            Some(0),
        ),
        observation(
            "e_000005",
            ProbeIntent::Help,
            vec!["measure".to_owned()],
            "Usage: cliare measure <TARGET>\n\nOptions:\n  --format <FORMAT>  Output format: json\n",
            Some(0),
        ),
        observation(
            "e_000007",
            ProbeIntent::OutputJson,
            vec!["measure".to_owned()],
            "{\"ok\":true}\n",
            Some(0),
        ),
        observation(
            "e_000009",
            ProbeIntent::InvalidFlag,
            vec!["measure".to_owned()],
            "error: unexpected flag\n\nFix:\n  cliare measure --help\n",
            Some(2),
        ),
    ];

    let scorecard = scorecard(target(), &observations, ScoreRunContext::default());
    let navigation = &scorecard.agent_navigation;

    assert_eq!(navigation.status, "experimental");
    assert!(
        navigation
            .limitations
            .iter()
            .any(|limitation| { limitation.contains("Agent navigation metrics are experimental") })
    );

    let canonical_help = &navigation.dimensions[&AgentNavigationCapability::CanonicalHelpCoverage];
    assert_eq!(canonical_help.score, Some(100.0));
    assert_eq!(canonical_help.numerator, 1);
    assert_eq!(canonical_help.denominator, 1);
    assert_eq!(canonical_help.status, AgentNavigationMetricStatus::Measured);
    assert_eq!(canonical_help.evidence, vec!["e_000005"]);

    let usage = &navigation.dimensions[&AgentNavigationCapability::UsageCoverage];
    assert_eq!(usage.score, Some(100.0));
    assert_eq!(usage.numerator, 1);
    assert_eq!(usage.denominator, 1);

    let subcommands = &navigation.dimensions[&AgentNavigationCapability::SubcommandTableClarity];
    assert_eq!(subcommands.score, Some(100.0));
    assert_eq!(subcommands.numerator, 1);
    assert_eq!(subcommands.denominator, 1);

    let output_contract =
        &navigation.dimensions[&AgentNavigationCapability::OutputContractParseCoverage];
    assert_eq!(output_contract.score, Some(100.0));
    assert_eq!(output_contract.numerator, 1);
    assert_eq!(output_contract.denominator, 1);

    let invalid_input = &navigation.dimensions[&AgentNavigationCapability::InvalidInputRecovery];
    assert_eq!(invalid_input.score, Some(100.0));
    assert_eq!(invalid_input.numerator, 2);
    assert_eq!(invalid_input.denominator, 2);
    assert_eq!(invalid_input.evidence, vec!["e_000009"]);

    let side_effect_safety =
        &navigation.dimensions[&AgentNavigationCapability::DiscoverySideEffectSafety];
    assert_eq!(side_effect_safety.score, Some(100.0));
    assert_eq!(side_effect_safety.numerator, 4);
    assert_eq!(side_effect_safety.denominator, 4);

    let examples = &navigation.dimensions[&AgentNavigationCapability::ExampleValidity];
    assert_eq!(examples.score, None);
    assert_eq!(examples.status, AgentNavigationMetricStatus::NotMeasured);

    let json = serde_json::to_value(&scorecard).expect("scorecard serializes");
    assert_eq!(
        json["agent_navigation"]["dimensions"]["canonical_help_coverage"]["score"],
        100.0
    );
    assert_eq!(
        json["agent_navigation"]["dimensions"]["example_validity"]["status"],
        "not_measured"
    );

    let report = report::render(&scorecard);
    assert!(report.contains("## Agent Navigation Evidence"));
    assert!(report.contains("canonical_help_coverage"));
    assert!(report.contains("output_contract_parse_coverage"));
    assert!(report.contains("example_validity"));
}

#[test]
fn alternate_help_invocation_is_not_canonical_help_evidence() {
    let observations = vec![
        observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Usage: cliare <COMMAND>\n\nCommands:\n  measure  Run probes\n",
            Some(0),
        ),
        observation_with_argv(
            "e_000005",
            ProbeIntent::Help,
            vec!["measure".to_owned()],
            "Usage: cliare measure <TARGET>\n\nOptions:\n  --format <FORMAT>  Output format: json\n",
            Some(0),
            vec!["cliare".to_owned(), "help".to_owned(), "measure".to_owned()],
        ),
    ];

    let scorecard = scorecard(target(), &observations, ScoreRunContext::default());
    let canonical_help =
        &scorecard.agent_navigation.dimensions[&AgentNavigationCapability::CanonicalHelpCoverage];
    let usage = &scorecard.agent_navigation.dimensions[&AgentNavigationCapability::UsageCoverage];

    assert_eq!(canonical_help.score, Some(0.0));
    assert_eq!(canonical_help.numerator, 0);
    assert_eq!(canonical_help.denominator, 1);
    assert_eq!(usage.score, Some(100.0));
}

#[test]
fn bare_help_like_output_is_navigation_evidence_but_not_canonical_help() {
    let observations = vec![
        observation(
            "e_000003",
            ProbeIntent::Help,
            vec![],
            "Usage: cliare <COMMAND>\n\nCommands:\n  measure  Run probes\n",
            Some(0),
        ),
        observation_with_argv(
            "e_000005",
            ProbeIntent::Help,
            vec!["measure".to_owned()],
            "Usage: cliare measure <TARGET>\n\nOptions:\n  --format <FORMAT>  Output format: json\n",
            Some(0),
            vec!["cliare".to_owned(), "measure".to_owned()],
        ),
    ];

    let scorecard = scorecard(target(), &observations, ScoreRunContext::default());
    let canonical_help =
        &scorecard.agent_navigation.dimensions[&AgentNavigationCapability::CanonicalHelpCoverage];
    let usage = &scorecard.agent_navigation.dimensions[&AgentNavigationCapability::UsageCoverage];
    let positionals = &scorecard.agent_navigation.dimensions
        [&AgentNavigationCapability::PositionalOperandCoverage];

    assert_eq!(canonical_help.score, Some(0.0));
    assert_eq!(canonical_help.numerator, 0);
    assert_eq!(canonical_help.denominator, 1);
    assert_eq!(usage.score, Some(100.0));
    assert_eq!(positionals.score, Some(100.0));
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
    let side_effect_safety = &scorecard.agent_navigation.dimensions
        [&AgentNavigationCapability::DiscoverySideEffectSafety];
    assert_eq!(side_effect_safety.score, None);
    assert_eq!(
        side_effect_safety.status,
        AgentNavigationMetricStatus::NotMeasured
    );
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
    observation_with_argv(
        evidence_id,
        intent,
        path.clone(),
        stdout,
        exit_code,
        argv_for(intent, &path),
    )
}

fn observation_with_argv(
    evidence_id: &str,
    intent: ProbeIntent,
    path: Vec<String>,
    stdout: &str,
    exit_code: Option<i32>,
    argv: Vec<String>,
) -> ShapeObservation {
    ShapeObservation {
        evidence_id: evidence_id.to_owned(),
        intent,
        path,
        process: ProcessCompleted {
            probe_id: "p_000001".to_owned(),
            argv,
            status: ProcessStatus::Exited { code: exit_code },
            duration_ms: 1,
            stdout: output(stdout),
            stderr: output(""),
            side_effects: SideEffectSummary::default(),
        },
    }
}

fn argv_for(intent: ProbeIntent, path: &[String]) -> Vec<String> {
    let mut argv = vec!["cliare".to_owned()];
    match intent {
        ProbeIntent::Help => {
            argv.extend(path.iter().cloned());
            argv.push("--help".to_owned());
        }
        ProbeIntent::Version => argv.push("--version".to_owned()),
        ProbeIntent::InvalidCommand => argv.push("__cliare_unknown_command__".to_owned()),
        ProbeIntent::InvalidChild => {
            argv.extend(path.iter().cloned());
            argv.push("__cliare_unknown_child__".to_owned());
        }
        ProbeIntent::InvalidFlag => {
            argv.extend(path.iter().cloned());
            argv.push("--__cliare_unknown_flag__".to_owned());
        }
        ProbeIntent::OutputJson | ProbeIntent::OutputJsonHelp => {
            argv.extend(path.iter().cloned());
            argv.push("--format".to_owned());
            argv.push("json".to_owned());
            if matches!(intent, ProbeIntent::OutputJsonHelp) {
                argv.push("--help".to_owned());
            }
        }
        ProbeIntent::OutputYaml | ProbeIntent::OutputYamlHelp => {
            argv.extend(path.iter().cloned());
            argv.push("--format".to_owned());
            argv.push("yaml".to_owned());
            if matches!(intent, ProbeIntent::OutputYamlHelp) {
                argv.push("--help".to_owned());
            }
        }
        ProbeIntent::OutputTable | ProbeIntent::OutputTableHelp => {
            argv.extend(path.iter().cloned());
            argv.push("--format".to_owned());
            argv.push("table".to_owned());
            if matches!(intent, ProbeIntent::OutputTableHelp) {
                argv.push("--help".to_owned());
            }
        }
        ProbeIntent::OutputPlain | ProbeIntent::OutputPlainHelp => {
            argv.extend(path.iter().cloned());
            argv.push("--format".to_owned());
            argv.push("plain".to_owned());
            if matches!(intent, ProbeIntent::OutputPlainHelp) {
                argv.push("--help".to_owned());
            }
        }
    }
    argv
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
