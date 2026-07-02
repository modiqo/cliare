use super::{ClaimSet, FlagValueKind};
use crate::observation::ShapeObservation;
use cliare_core::probe_intent::ProbeIntent;
use cliare_core::process_status::ProcessStatus;
use cliare_evidence::ProcessCompleted;
use cliare_inference::precondition::PreconditionKind;
use cliare_inference::score_model::ScoreModelSpec;
use cliare_runtime::process::OutputCapture;

#[test]
fn claims_track_layout_and_runtime_confirmation() {
    let observations = vec![
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

    let claims = ClaimSet::from_observations("cliare", &observations);
    let measure = claims
        .commands()
        .find(|claim| claim.path().as_slice() == ["measure"])
        .expect("measure claim exists");

    assert!(measure.runtime_confirmed());
    assert!(measure.confidence() > 0.90);
    assert!(measure.usage_observed());
    assert!(
        measure
            .positionals()
            .any(|argument| argument.name() == "target" && argument.required())
    );

    let out = claims
        .flags()
        .find(|flag| flag.name() == "--out")
        .expect("out flag exists");
    assert_eq!(out.value_kind(), FlagValueKind::Required);
    assert_eq!(out.value_name(), Some("dir"));
}

#[test]
fn multiline_usage_help_confirms_current_command() {
    let observations = vec![observation(
        "e_000024",
        ProbeIntent::Help,
        vec!["backups".to_owned()],
        "Manage Supabase physical backups\n\nUsage:\n  supabase backups [command]\n\nAvailable Commands:\n  list     Lists available physical backups\n  restore  Restore to a specific timestamp using PITR\n\nFlags:\n  -h, --help  help for backups\n",
        Some(0),
    )];

    let claims = ClaimSet::from_observations("supabase", &observations);
    let backups = claims
        .commands()
        .find(|claim| claim.path().as_slice() == ["backups"])
        .expect("backups claim exists");

    assert!(backups.runtime_confirmed());
    assert!(!backups.help_unavailable());
}

#[test]
fn claims_record_negative_diagnostic_probes() {
    let observations = vec![
        observation(
            "e_000005",
            ProbeIntent::Help,
            vec!["measure".to_owned()],
            "Usage: cliare measure <TARGET>\n\nCommands:\n  nested  Nested command\n",
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
            "error: unexpected argument '--__cliare_unknown_cliare_measure_flag__' found\n\nUsage: cliare measure [OPTIONS] <TARGET>\n\nFor more information, try '--help'.\n",
            Some(2),
        ),
    ];

    let claims = ClaimSet::from_observations("cliare", &observations);
    let measure = claims
        .commands()
        .find(|claim| claim.path().as_slice() == ["measure"])
        .expect("measure claim exists");

    assert!(measure.invalid_child_rejected());
    assert!(measure.invalid_flag_rejected());
    assert!(measure.has_child_candidates());
    assert!(!measure.precondition_blocked());
}

#[test]
fn absolute_help_references_are_not_nested_under_current_command() {
    let observations = vec![observation(
        "e_000011",
        ProbeIntent::Help,
        vec!["adapter".to_owned(), "list".to_owned()],
        "rote adapter list - Show installed adapters\n\nNEXT STEPS\n  rote adapter info <ID>    Show details\n  rote adapter reauth <ID>  Re-authorize an adapter\n",
        Some(0),
    )];

    let claims = ClaimSet::from_observations("rote", &observations);

    assert!(
        claims
            .commands()
            .any(|claim| claim.path().as_slice() == ["adapter", "info"])
    );
    assert!(
        !claims
            .commands()
            .any(|claim| claim.path().as_slice() == ["adapter", "list", "adapter", "info"])
    );
    let adapter_list = claims
        .commands()
        .find(|claim| claim.path().as_slice() == ["adapter", "list"])
        .expect("current command claim exists");
    assert!(!adapter_list.has_child_candidates());
}

#[test]
fn parent_help_tables_are_scoped_to_the_matching_ancestor() {
    let observations = vec![observation(
        "e_000013",
        ProbeIntent::Help,
        vec!["flow".to_owned(), "search".to_owned()],
        "FLOW COMMANDS\n\nCOMMANDS:\n  list [--json]        List flows\n  search <QUERY>       Search flows\n  doctor               Check all flows\n",
        Some(0),
    )];

    let claims = ClaimSet::from_observations("rote", &observations);

    assert!(
        claims
            .commands()
            .any(|claim| claim.path().as_slice() == ["flow", "doctor"])
    );
    assert!(
        !claims
            .commands()
            .any(|claim| claim.path().as_slice() == ["flow", "search", "doctor"])
    );
    let flow_search = claims
        .commands()
        .find(|claim| claim.path().as_slice() == ["flow", "search"])
        .expect("current command claim exists");
    assert!(!flow_search.has_child_candidates());
}

#[test]
fn parent_help_echo_does_not_confirm_or_attach_features_to_child_candidate() {
    let observations = vec![observation(
        "e_000017",
        ProbeIntent::Help,
        vec![
            "adapter".to_owned(),
            "set".to_owned(),
            "base_url".to_owned(),
        ],
        "tool adapter set - Mutate a key\n\nUSAGE\n  tool adapter set <ID> <KEY> <VALUE> [--json]\n\nFLAGS\n  --json  Emit result as JSON\n",
        Some(0),
    )];

    let claims = ClaimSet::from_observations("tool", &observations);
    let child = claims
        .commands()
        .find(|claim| claim.path().as_slice() == ["adapter", "set", "base_url"])
        .expect("child claim exists");

    assert!(!child.runtime_confirmed());
    assert!(!child.help_unavailable());
    assert!(claims.flags().any(
        |claim| claim.command_path().as_slice() == ["adapter", "set"] && claim.name() == "--json"
    ));
    assert!(!claims.flags().any(|claim| claim.command_path().as_slice()
        == ["adapter", "set", "base_url"]
        && claim.name() == "--json"));
}

#[test]
fn single_parent_help_row_does_not_duplicate_current_tail() {
    let observations = vec![observation(
        "e_000015",
        ProbeIntent::Help,
        vec!["install".to_owned(), "skill".to_owned()],
        "rote install - Installation utilities\n\nSUBCOMMANDS\n  skill  Install provider integration\n",
        Some(0),
    )];

    let claims = ClaimSet::from_observations("rote", &observations);

    assert!(
        claims
            .commands()
            .any(|claim| claim.path().as_slice() == ["install", "skill"])
    );
    assert!(
        !claims
            .commands()
            .any(|claim| claim.path().as_slice() == ["install", "skill", "skill"])
    );
}

#[test]
fn auth_blocked_help_records_precondition_without_negative_help_failure() {
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
    ];

    let claims = ClaimSet::from_observations("rote", &observations);
    let model = claims
        .commands()
        .find(|claim| claim.path().as_slice() == ["model"])
        .expect("model claim exists");

    assert!(model.precondition_blocked());
    assert_eq!(
        model.preconditions().collect::<Vec<_>>(),
        vec![PreconditionKind::AuthRequired]
    );
    assert!(!model.help_unavailable());
    assert!(!model.runtime_confirmed());
    assert!(model.confidence() > 0.5);
}

#[test]
fn claim_confidence_uses_supplied_model_inference_weights() {
    let observations = vec![observation(
        "e_000005",
        ProbeIntent::Help,
        vec!["model".to_owned()],
        "error: rote requires login\n\nrun rote login",
        Some(77),
    )];
    let default_claims = ClaimSet::from_observations("rote", &observations);
    let default_model = default_claims
        .commands()
        .find(|claim| claim.path().as_slice() == ["model"])
        .expect("default model claim exists");
    assert!(default_model.confidence() > 0.08);

    let mut score_model = ScoreModelSpec::bundled().clone();
    score_model.evidence_weights.runtime_precondition_block = 0.0;

    let custom_claims = ClaimSet::from_observations_with_model("rote", &observations, &score_model);
    let custom_model = custom_claims
        .commands()
        .find(|claim| claim.path().as_slice() == ["model"])
        .expect("custom model claim exists");

    assert!(custom_model.precondition_blocked());
    assert!((custom_model.confidence() - 0.08).abs() < 0.000_000_000_001);
}

#[test]
fn flag_confidence_uses_supplied_model_inference_weights() {
    let observations = vec![observation(
        "e_000003",
        ProbeIntent::Help,
        vec![],
        "Usage: cliare metadata [--format <FORMAT>]\n\nOptions:\n  --format <FORMAT>  Output format\n",
        Some(0),
    )];
    let mut score_model = ScoreModelSpec::bundled().clone();
    score_model.claim_priors.flag_exists = 0.5;
    score_model.evidence_weights.layout_candidate = 0.0;

    let claims = ClaimSet::from_observations_with_model("cliare", &observations, &score_model);
    let format = claims
        .flags()
        .find(|flag| flag.name() == "--format")
        .expect("format flag exists");

    assert!((format.confidence() - 0.5).abs() < f64::EPSILON);
}

#[test]
fn output_mode_probe_records_runtime_precondition() {
    let path = vec!["issue".to_owned(), "list".to_owned()];
    let observations = vec![
        observation_with_argv_streams(
            "e_000011",
            ProbeIntent::Help,
            path.clone(),
            vec![
                "gh".to_owned(),
                "issue".to_owned(),
                "list".to_owned(),
                "--help".to_owned(),
            ],
            "USAGE\n  gh issue list [flags]\n\nFLAGS\n      --json fields        Output JSON with the specified fields\n",
            "",
            Some(0),
        ),
        observation_with_argv_streams(
            "e_000013",
            ProbeIntent::OutputJson,
            path.clone(),
            vec![
                "gh".to_owned(),
                "issue".to_owned(),
                "list".to_owned(),
                "--json".to_owned(),
                "json".to_owned(),
            ],
            "",
            "gh: To use GitHub CLI in automation, set the GH_TOKEN environment variable.",
            Some(4),
        ),
    ];

    let claims = ClaimSet::from_observations("gh", &observations);
    let issue_list = claims
        .commands()
        .find(|claim| claim.path().as_slice() == ["issue", "list"])
        .expect("command claim exists");
    assert!(issue_list.precondition_blocked());
    assert_eq!(
        issue_list.preconditions().collect::<Vec<_>>(),
        vec![PreconditionKind::AuthRequired]
    );

    let contract = claims
        .output_contracts()
        .find(|claim| {
            claim.command_path().as_slice() == ["issue", "list"] && claim.flag_name() == "--json"
        })
        .expect("output contract exists");
    assert!(contract.probed());
    assert!(contract.precondition_blocked());
    assert!(!contract.parse_success());
    assert_eq!(
        contract.preconditions().collect::<Vec<_>>(),
        vec![PreconditionKind::AuthRequired]
    );
}

#[test]
fn output_mode_probe_records_network_precondition() {
    let path = vec!["issue".to_owned(), "list".to_owned()];
    let observations = vec![
        observation_with_argv_streams(
            "e_000011",
            ProbeIntent::Help,
            path.clone(),
            vec![
                "gh".to_owned(),
                "issue".to_owned(),
                "list".to_owned(),
                "--help".to_owned(),
            ],
            "USAGE\n  gh issue list [flags]\n\nFLAGS\n      --json fields        Output JSON with the specified fields\n",
            "",
            Some(0),
        ),
        observation_with_argv_streams(
            "e_000013",
            ProbeIntent::OutputJson,
            path,
            vec![
                "gh".to_owned(),
                "issue".to_owned(),
                "list".to_owned(),
                "--json".to_owned(),
                "number,title,url".to_owned(),
            ],
            "",
            "error connecting to api.example.com\ncheck your internet connection",
            Some(1),
        ),
    ];

    let claims = ClaimSet::from_observations("gh", &observations);
    let contract = claims
        .output_contracts()
        .find(|claim| {
            claim.command_path().as_slice() == ["issue", "list"]
                && claim
                    .preconditions()
                    .any(|kind| kind == PreconditionKind::NetworkUnavailable)
        })
        .expect("network-blocked output contract exists");

    assert!(contract.probed());
    assert!(contract.precondition_blocked());
    assert!(!contract.parse_success());
    assert_eq!(
        contract.preconditions().collect::<Vec<_>>(),
        vec![PreconditionKind::NetworkUnavailable]
    );
}

fn observation(
    evidence_id: &str,
    intent: ProbeIntent,
    path: Vec<String>,
    stdout: &str,
    exit_code: Option<i32>,
) -> ShapeObservation {
    observation_with_argv_streams(
        evidence_id,
        intent,
        path,
        vec!["cliare".to_owned(), "--help".to_owned()],
        stdout,
        "",
        exit_code,
    )
}

fn observation_with_argv_streams(
    evidence_id: &str,
    intent: ProbeIntent,
    path: Vec<String>,
    argv: Vec<String>,
    stdout: &str,
    stderr: &str,
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
            stderr: output(stderr),
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
