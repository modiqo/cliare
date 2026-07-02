use super::{RecoveryActionFamily, RecoveryBlockKind, RecoveryQuality, analyze_process};
use crate::precondition::PreconditionKind;
use cliare_core::process_status::ProcessStatus;

#[test]
fn classifies_workspace_context_from_structured_recovery() {
    let text = "error: not in a workspace directory\n\n  Fix:\n    rote init workflow-name --seq\n    cd ~/.rote/workspaces/workflow-name\n\nhint: or list existing: 'rote ls'\n";
    let analysis = analyze_process(&ProcessStatus::Exited { code: Some(1) }, None, Some(text));

    assert_eq!(
        analysis.precondition,
        Some(PreconditionKind::LocalContextRequired)
    );
    assert_eq!(analysis.recovery.quality, RecoveryQuality::Actionable);
    assert!(
        analysis
            .recovery
            .labeled_blocks
            .contains(&RecoveryBlockKind::Fix)
    );
    assert!(
        analysis
            .recovery
            .action_families
            .contains(&RecoveryActionFamily::ChangeDirectory)
    );
}

#[test]
fn classifies_repository_context_without_exact_phrase_matching() {
    let text = "failed to run git: fatal: not a git repository (or any parent): .git";
    let analysis = analyze_process(&ProcessStatus::Exited { code: Some(1) }, None, Some(text));

    assert_eq!(
        analysis.precondition,
        Some(PreconditionKind::LocalContextRequired)
    );
    assert_eq!(analysis.recovery.quality, RecoveryQuality::None);
}

#[test]
fn classifies_token_environment_variable_as_auth_required() {
    let text = "gh: To use GitHub CLI in automation, set the GH_TOKEN environment variable.";
    let analysis = analyze_process(&ProcessStatus::Exited { code: Some(4) }, None, Some(text));

    assert_eq!(analysis.precondition, Some(PreconditionKind::AuthRequired));
    assert_eq!(analysis.recovery.quality, RecoveryQuality::None);
}

#[test]
fn classifies_inline_auth_command_recovery() {
    let text = "To get started with GitHub CLI, please run:  gh auth login -s codespace";
    let analysis = analyze_process(&ProcessStatus::Exited { code: Some(4) }, None, Some(text));

    assert_eq!(analysis.precondition, Some(PreconditionKind::AuthRequired));
    assert_eq!(analysis.recovery.quality, RecoveryQuality::Actionable);
    assert!(
        analysis
            .recovery
            .action_families
            .contains(&RecoveryActionFamily::Authenticate)
    );
}

#[test]
fn classifies_missing_fixture_inputs_without_treating_output_as_malformed() {
    let analysis = analyze_process(
        &ProcessStatus::Exited { code: Some(1) },
        None,
        Some("owner is required when not running interactively"),
    );

    assert_eq!(
        analysis.precondition,
        Some(PreconditionKind::FixtureRequired)
    );
    assert_eq!(analysis.recovery.quality, RecoveryQuality::None);

    let analysis = analyze_process(
        &ProcessStatus::Exited { code: Some(2) },
        None,
        Some("missing required argument <PROJECT_ID>"),
    );

    assert_eq!(
        analysis.precondition,
        Some(PreconditionKind::FixtureRequired)
    );

    let analysis = analyze_process(
        &ProcessStatus::Exited { code: Some(1) },
        None,
        Some("required flag(s) \"title\" not set"),
    );

    assert_eq!(
        analysis.precondition,
        Some(PreconditionKind::FixtureRequired)
    );
}

#[test]
fn keeps_plain_usage_errors_unclassified() {
    let analysis = analyze_process(
        &ProcessStatus::Exited { code: Some(2) },
        None,
        Some("error: unexpected argument '--wat'"),
    );

    assert_eq!(analysis.precondition, None);
    assert_eq!(analysis.recovery.quality, RecoveryQuality::None);

    let analysis = analyze_process(
        &ProcessStatus::Exited { code: Some(2) },
        None,
        Some(
            "error: unexpected argument '--__cliare_unknown_cliare_guard_flag__' found\n\n  tip: to pass '--__cliare_unknown_cliare_guard_flag__' as a value, use '-- --__cliare_unknown_cliare_guard_flag__'\n\nUsage: cliare guard [OPTIONS] --baseline <FILE> <TARGET>\n\nFor more information, try '--help'.\n",
        ),
    );

    assert_eq!(analysis.precondition, None);
    assert_eq!(analysis.recovery.quality, RecoveryQuality::Mentioned);
}

#[test]
fn keeps_invalid_subcommand_diagnostics_unclassified() {
    let analysis = analyze_process(
        &ProcessStatus::Exited { code: Some(2) },
        None,
        Some(
            "error: unrecognized subcommand 'frobnicate'\n\nUsage: cliare context <COMMAND>\n\nFor more information, try '--help'.\n",
        ),
    );

    assert_eq!(analysis.precondition, None);
}

#[test]
fn keeps_unrecognized_diagnostics_unclassified() {
    let analysis = analyze_process(
        &ProcessStatus::Exited { code: Some(2) },
        None,
        Some("operacion rechazada por condicion externa"),
    );

    assert_eq!(analysis.precondition, None);
    assert_eq!(analysis.confidence, 0.0);
}
