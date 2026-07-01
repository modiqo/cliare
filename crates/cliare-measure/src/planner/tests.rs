use super::{ConvergencePolicy, DeterministicPlanner, ProbePlanner};
use crate::claims::ClaimSet;
use crate::evidence::{ProbeIntent, ProcessCompleted, ProcessStatus};
use crate::observation::ShapeObservation;
use crate::process::OutputCapture;

#[test]
fn planner_schedules_confirmation_before_diagnostics() {
    let observations = vec![observation(
        "e_000003",
        ProbeIntent::Help,
        vec![],
        "Commands:\n  measure  Run probes\n",
        Some(0),
    )];
    let claims = ClaimSet::from_observations("cliare", &observations);
    let mut planner = DeterministicPlanner::new(2, "cliare".to_owned());

    planner.extend_from_claims(&claims);
    let first = planner.next().expect("first probe");

    assert_eq!(first.intent, ProbeIntent::Help);
    assert_eq!(first.args, ["measure", "--help"]);
}

#[test]
fn planner_schedules_diagnostics_for_confirmed_commands() {
    let observations = vec![observation(
        "e_000005",
        ProbeIntent::Help,
        vec!["measure".to_owned()],
        "Usage: cliare measure <TARGET>\n\nCommands:\n  child  Nested command\n\nOptions:\n  --out <DIR>  Output directory\n",
        Some(0),
    )];
    let claims = ClaimSet::from_observations("cliare", &observations);
    let mut planner = DeterministicPlanner::new(2, "cliare".to_owned());

    planner.extend_from_claims(&claims);
    let probes = std::iter::from_fn(|| planner.next())
        .map(|probe| probe.intent)
        .collect::<Vec<_>>();

    assert!(probes.contains(&ProbeIntent::InvalidChild));
    assert!(probes.contains(&ProbeIntent::InvalidFlag));
}

#[test]
fn planner_does_not_send_invalid_child_to_leaf_commands() {
    let observations = vec![observation(
        "e_000005",
        ProbeIntent::Help,
        vec!["measure".to_owned()],
        "Usage: cliare measure <TARGET>\n\nOptions:\n  --out <DIR>  Output directory\n",
        Some(0),
    )];
    let claims = ClaimSet::from_observations("cliare", &observations);
    let mut planner = DeterministicPlanner::new(2, "cliare".to_owned());

    planner.extend_from_claims(&claims);
    let probes = std::iter::from_fn(|| planner.next())
        .map(|probe| probe.intent)
        .collect::<Vec<_>>();

    assert!(!probes.contains(&ProbeIntent::InvalidChild));
    assert!(probes.contains(&ProbeIntent::InvalidFlag));
}

#[test]
fn planner_does_not_probe_output_modes_when_required_positionals_are_unknown() {
    let observations = vec![observation(
        "e_000005",
        ProbeIntent::Help,
        vec!["report".to_owned()],
        "Usage: cliare report <PERSONA> [OPTIONS]\n\nOptions:\n  --format <FORMAT>  Representation to print [possible values: markdown, json]\n",
        Some(0),
    )];
    let claims = ClaimSet::from_observations("cliare", &observations);
    let mut planner = DeterministicPlanner::new(2, "cliare".to_owned());

    assert_eq!(claims.output_contracts().count(), 1);

    planner.extend_from_claims(&claims);
    let probes = std::iter::from_fn(|| planner.next())
        .map(|probe| probe.intent)
        .collect::<Vec<_>>();

    assert!(!probes.contains(&ProbeIntent::OutputJson));
}

#[test]
fn planner_respects_deep_recursion_limit() {
    let observations = vec![
        observation(
            "e_000001",
            ProbeIntent::Help,
            vec!["alpha".to_owned(), "beta".to_owned(), "gamma".to_owned()],
            "Usage: tool alpha beta gamma <COMMAND>\n\nCommands:\n  delta  Continue\n",
            Some(0),
        ),
        observation(
            "e_000002",
            ProbeIntent::Help,
            vec![
                "alpha".to_owned(),
                "beta".to_owned(),
                "gamma".to_owned(),
                "delta".to_owned(),
            ],
            "Usage: tool alpha beta gamma delta <COMMAND>\n\nCommands:\n  epsilon  Continue\n",
            Some(0),
        ),
    ];
    let claims = ClaimSet::from_observations("tool", &observations);

    let mut shallow = DeterministicPlanner::new(3, "tool".to_owned());
    shallow.extend_from_claims(&claims);
    let shallow_probes = std::iter::from_fn(|| shallow.next())
        .map(|probe| probe.args)
        .collect::<Vec<_>>();

    let mut deep = DeterministicPlanner::new(5, "tool".to_owned());
    deep.extend_from_claims(&claims);
    let deep_probes = std::iter::from_fn(|| deep.next())
        .map(|probe| probe.args)
        .collect::<Vec<_>>();

    assert!(!shallow_probes.iter().any(|args| args
        == &[
            "alpha".to_owned(),
            "beta".to_owned(),
            "gamma".to_owned(),
            "delta".to_owned(),
            "epsilon".to_owned(),
            "--help".to_owned(),
        ]));
    assert!(deep_probes.iter().any(|args| args
        == &[
            "alpha".to_owned(),
            "beta".to_owned(),
            "gamma".to_owned(),
            "delta".to_owned(),
            "epsilon".to_owned(),
            "--help".to_owned(),
        ]));
}

#[test]
fn planner_skips_low_value_dynamic_probes_by_convergence_policy() {
    let observations = vec![observation(
        "e_000003",
        ProbeIntent::Help,
        vec![],
        "Commands:\n  maybe  Weak candidate\n",
        Some(0),
    )];
    let claims = ClaimSet::from_observations("tool", &observations);
    let mut planner =
        DeterministicPlanner::with_policy(2, ConvergencePolicy::new(1_000), "tool".to_owned());

    planner.extend_from_claims(&claims);

    assert!(planner.next().is_none());
    let stats = planner.stats();
    assert_eq!(stats.frontier_remaining, 0);
    assert_eq!(stats.candidates_skipped_by_convergence, 2);
    assert_eq!(stats.min_expected_value, 1_000);
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
            side_effects: crate::sandbox::SideEffectSummary::default(),
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
