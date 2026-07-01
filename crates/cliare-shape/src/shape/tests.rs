use super::infer_shape;
use crate::observation::ShapeObservation;
use cliare_core::probe_intent::ProbeIntent;
use cliare_core::process_status::ProcessStatus;
use cliare_evidence::ProcessCompleted;
use cliare_inference::precondition::PreconditionKind;
use cliare_runtime::fingerprint::TargetFingerprint;
use cliare_runtime::process::OutputCapture;

#[test]
fn generic_layout_candidates_are_low_confidence_until_confirmed() {
    let target = target();
    let root = observation(
        "e_000003",
        ProbeIntent::Help,
        vec![],
        "Commands:\n  measure  Run probes\n\nOptions:\n  -h, --help     Print help\n",
        Some(0),
    );

    let shape = infer_shape(target, &[root]);

    let measure = shape
        .commands
        .iter()
        .find(|command| command.path == ["measure"])
        .expect("measure candidate exists");
    assert!(!measure.runtime_confirmed);
    assert!(measure.confidence < 0.80);
    assert!(shape.flags.iter().any(|flag| flag.name == "--help"));
    assert!(shape.gaps.iter().any(|gap| gap.command_path == ["measure"]));
}

#[test]
fn runtime_help_confirmation_raises_command_confidence() {
    let target = target();
    let root = observation(
        "e_000003",
        ProbeIntent::Help,
        vec![],
        "Commands:\n  measure  Run probes\n",
        Some(0),
    );
    let measure_help = observation(
        "e_000005",
        ProbeIntent::Help,
        vec!["measure".to_owned()],
        "Usage: cliare measure <TARGET>\n\nOptions:\n  --out <DIR>  Output directory\n",
        Some(0),
    );

    let shape = infer_shape(target, &[root, measure_help]);

    let measure = shape
        .commands
        .iter()
        .find(|command| command.path == ["measure"])
        .expect("measure candidate exists");
    assert!(measure.runtime_confirmed);
    assert!(measure.confidence > 0.90);
}

#[test]
fn auth_blocked_help_is_shape_precondition_not_help_unavailable() {
    let target = target();
    let root = observation(
        "e_000003",
        ProbeIntent::Help,
        vec![],
        "Commands:\n  model  Track AI model identity\n",
        Some(0),
    );
    let model_help = observation(
        "e_000005",
        ProbeIntent::Help,
        vec!["model".to_owned()],
        "error: rote requires login\n\nrun rote login",
        Some(77),
    );

    let shape = infer_shape(target, &[root, model_help]);
    let model = shape
        .commands
        .iter()
        .find(|command| command.path == ["model"])
        .expect("model candidate exists");

    assert!(!model.runtime_confirmed);
    assert!(matches!(
        model.runtime_state,
        super::CommandRuntimeState::PreconditionBlocked
    ));
    assert_eq!(model.preconditions.len(), 1);
    assert!(shape.gaps.iter().any(|gap| {
        gap.command_path == ["model"] && matches!(gap.kind, super::GapKind::PreconditionBlocked)
    }));
    assert!(!shape.gaps.iter().any(|gap| {
        gap.command_path == ["model"] && matches!(gap.kind, super::GapKind::HelpUnavailable)
    }));
}

#[test]
fn alternate_help_failure_is_nonblocking_when_direct_help_succeeds() {
    let target = target();
    let root = observation(
        "e_000003",
        ProbeIntent::Help,
        vec![],
        "Commands:\n  flow  Manage flows\n",
        Some(0),
    );
    let direct_help = observation_with_args(
        "e_000005",
        ProbeIntent::Help,
        vec!["flow".to_owned(), "list".to_owned()],
        vec![
            "cliare".to_owned(),
            "flow".to_owned(),
            "list".to_owned(),
            "--help".to_owned(),
        ],
        "cliare flow list\nList flows\n\nUSAGE:\n  cliare flow list [OPTIONS]\n\nOPTIONS:\n  --filter <TEXT>  Filter flows\n",
        Some(0),
    );
    let alternate_help = observation_with_args(
        "e_000007",
        ProbeIntent::Help,
        vec!["flow".to_owned(), "list".to_owned()],
        vec![
            "cliare".to_owned(),
            "help".to_owned(),
            "flow".to_owned(),
            "list".to_owned(),
        ],
        "No help available for 'flow'\n\nTry: cliare help\n",
        Some(0),
    );
    let invalid_flag = observation_with_args(
        "e_000009",
        ProbeIntent::InvalidFlag,
        vec!["flow".to_owned(), "list".to_owned()],
        vec![
            "cliare".to_owned(),
            "flow".to_owned(),
            "list".to_owned(),
            "--__cliare_unknown_flow_list_flag__".to_owned(),
        ],
        "error: unknown flag",
        Some(2),
    );

    let index =
        super::infer_command_index(target, &[root, direct_help, alternate_help, invalid_flag]);
    let flow_list = index
        .commands
        .iter()
        .find(|command| command.path == ["flow", "list"])
        .expect("flow list command exists");

    assert!(matches!(
        flow_list.agent_suitability,
        super::AgentSuitability::Ready
    ));
    assert!(
        flow_list
            .gaps
            .iter()
            .any(|gap| matches!(gap.kind, super::GapKind::AlternateHelpFormUnavailable))
    );
    assert!(
        !flow_list
            .gaps
            .iter()
            .any(|gap| matches!(gap.kind, super::GapKind::HelpUnavailable))
    );
}

#[test]
fn local_context_precondition_is_conditional_in_command_index() {
    let target = target();
    let root = observation(
        "e_000003",
        ProbeIntent::Help,
        vec![],
        "Commands:\n  stats  Show workspace statistics\n",
        Some(0),
    );
    let stats = observation(
        "e_000005",
        ProbeIntent::Help,
        vec!["stats".to_owned()],
        "error: not in a workspace directory\n\nFix:\n  cliare init demo\n  cd workspaces/demo\n\nhint: or list existing: 'cliare ls'\n",
        Some(1),
    );

    let index = super::infer_command_index(target, &[root, stats]);
    let stats = index
        .commands
        .iter()
        .find(|command| command.path == ["stats"])
        .expect("stats command exists");

    assert!(matches!(
        stats.runtime_state,
        super::CommandRuntimeState::PreconditionBlocked
    ));
    assert_eq!(
        stats.agent_suitability,
        super::AgentSuitability::Conditional
    );
    assert_eq!(
        stats.preconditions,
        vec![PreconditionKind::LocalContextRequired]
    );
}

#[test]
fn shape_includes_usage_positionals_and_flag_grammar() {
    let target = target();
    let deploy_help = observation(
        "e_000005",
        ProbeIntent::Help,
        vec!["project".to_owned(), "deploy".to_owned()],
        "Usage: cliare project deploy <PROJECT> [ENV] [FILES]...\n\nOptions:\n  -f, --format <KIND>       Output format\n  --color[=<WHEN>]          Optional color mode\n  --tag <TAG>...            Repeatable tag\n  --token <TOKEN>           Required authentication token\n  --dry-run                 Do not write changes\n",
        Some(0),
    );

    let shape = infer_shape(target, &[deploy_help]);
    let deploy = shape
        .commands
        .iter()
        .find(|command| command.path == ["project", "deploy"])
        .expect("deploy command exists");

    assert!(deploy.usage_observed);
    assert!(
        deploy.positionals.iter().any(|argument| {
            argument.name == "project" && argument.required && !argument.variadic
        })
    );
    assert!(
        deploy
            .positionals
            .iter()
            .any(|argument| argument.name == "env" && !argument.required)
    );
    assert!(
        deploy
            .positionals
            .iter()
            .any(|argument| argument.name == "files" && argument.variadic)
    );

    let format = shape
        .flags
        .iter()
        .find(|flag| flag.name == "--format")
        .expect("format flag exists");
    assert!(matches!(
        format.value_kind,
        super::FlagValueKindShape::Required
    ));
    assert_eq!(format.value_name.as_deref(), Some("kind"));
    assert_eq!(format.short.as_deref(), Some("-f"));

    let color = shape
        .flags
        .iter()
        .find(|flag| flag.name == "--color")
        .expect("color flag exists");
    assert!(matches!(
        color.value_kind,
        super::FlagValueKindShape::Optional
    ));

    let tag = shape
        .flags
        .iter()
        .find(|flag| flag.name == "--tag")
        .expect("tag flag exists");
    assert!(tag.repeatable);

    let token = shape
        .flags
        .iter()
        .find(|flag| flag.name == "--token")
        .expect("token flag exists");
    assert!(token.required);
}

#[test]
fn shape_keeps_nested_candidates_from_child_help() {
    let target = target();
    let flow_help = observation(
        "e_000003",
        ProbeIntent::Help,
        vec!["flow".to_owned()],
        "Commands:\n  search  Search flows\n",
        Some(0),
    );

    let shape = infer_shape(target, &[flow_help]);

    assert!(
        shape
            .commands
            .iter()
            .any(|command| command.path == ["flow", "search"])
    );
}

#[test]
fn diagnostic_probes_close_diagnostic_gaps() {
    let target = target();
    let observations = vec![
        observation(
            "e_000005",
            ProbeIntent::Help,
            vec!["measure".to_owned()],
            "Usage: cliare measure <TARGET>\n\nCommands:\n  nested  Nested command\n\nOptions:\n  --out <DIR>  Output directory\n",
            Some(0),
        ),
        observation(
            "e_000007",
            ProbeIntent::InvalidChild,
            vec!["measure".to_owned()],
            "error: unexpected argument",
            Some(2),
        ),
        observation(
            "e_000009",
            ProbeIntent::InvalidFlag,
            vec!["measure".to_owned()],
            "error: unexpected argument",
            Some(2),
        ),
    ];

    let shape = infer_shape(target, &observations);
    let measure = shape
        .commands
        .iter()
        .find(|command| command.path == ["measure"])
        .expect("measure command exists");

    assert!(measure.runtime_confirmed);
    assert!(!shape.gaps.iter().any(|gap| {
        gap.command_path == ["measure"]
            && matches!(
                gap.kind,
                super::GapKind::InvalidChildDiagnosticsUnknown
                    | super::GapKind::InvalidFlagDiagnosticsUnknown
            )
    }));
}

fn target() -> TargetFingerprint {
    TargetFingerprint {
        requested: "cliare".into(),
        resolved: "/tmp/cliare".into(),
        binary_sha256: "abc".to_owned(),
        size_bytes: 1,
    }
}

fn observation(
    evidence_id: &str,
    intent: ProbeIntent,
    path: Vec<String>,
    stdout: &str,
    exit_code: Option<i32>,
) -> ShapeObservation {
    let mut argv = vec!["cliare".to_owned()];
    argv.extend(path.iter().cloned());
    if matches!(intent, ProbeIntent::Help) {
        argv.push("--help".to_owned());
    }
    observation_with_args(evidence_id, intent, path, argv, stdout, exit_code)
}

fn observation_with_args(
    evidence_id: &str,
    intent: ProbeIntent,
    path: Vec<String>,
    argv: Vec<String>,
    stdout: &str,
    exit_code: Option<i32>,
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
            side_effects: cliare_runtime::sandbox::SideEffectSummary::default(),
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
