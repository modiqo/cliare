use std::path::PathBuf;

use cliare_cli::cli::{PlaybookArgs, PlaybookFormat, PlaybookRole};

use super::{DEFAULT_OUT_PLACEHOLDER, RolePlaybook, playbook};

#[test]
fn maintainer_playbook_contains_full_lifecycle() {
    let args = PlaybookArgs {
        role: PlaybookRole::Maintainer,
        target: Some("rote".to_owned()),
        out: PathBuf::from(DEFAULT_OUT_PLACEHOLDER),
        context: None,
        format: PlaybookFormat::Markdown,
    };

    let summary = playbook(args).expect("playbook renders");
    let text = summary.terminal_summary();

    assert!(text.contains("## 1. Measure"));
    assert!(text.contains("## 2. View"));
    assert!(text.contains("## 4. Disposition"));
    assert!(text.contains("## 6. Gate in CI"));
    assert!(text.contains("## 7. Publish Agent Surface"));
    assert!(text.contains("## Artifact Directory"));
    assert!(text.contains("cliare measure rote --out .cliare/rote --profile deep --refresh"));
    assert!(text.contains("cliare jobs status --out .cliare/rote"));
    assert!(text.contains("cliare issues list --out .cliare/rote --format markdown"));
    assert!(text.contains("cliare measure rote --out .cliare/rote --context authenticated"));
}

#[test]
fn maintainer_playbook_json_is_structured() {
    let args = PlaybookArgs {
        role: PlaybookRole::Maintainer,
        target: None,
        out: PathBuf::from(DEFAULT_OUT_PLACEHOLDER),
        context: Some("authenticated".to_owned()),
        format: PlaybookFormat::Json,
    };

    let summary = playbook(args).expect("playbook renders");
    let value: serde_json::Value =
        serde_json::from_str(summary.terminal_summary()).expect("json parses");

    assert_eq!(value["schema_version"], "cliare.playbook.v1");
    assert_eq!(value["role"], "maintainer");
    assert_eq!(value["title"], "CLIARE Maintainer Playbook");
    assert_eq!(value["target"], "<target-cli>");
    assert_eq!(value["out"], ".cliare/<target-cli>");
    assert_eq!(value["context"], "authenticated");
    assert!(value["artifact_layout"].is_array());
}

#[test]
fn maintainer_playbook_human_is_step_by_step() {
    let args = PlaybookArgs {
        role: PlaybookRole::Maintainer,
        target: Some("mise".to_owned()),
        out: PathBuf::from(DEFAULT_OUT_PLACEHOLDER),
        context: None,
        format: PlaybookFormat::Human,
    };

    let summary = playbook(args).expect("playbook renders");
    let text = summary.terminal_summary();

    assert!(text.contains("CLIARE maintainer walkthrough"));
    assert!(text.contains("artifacts: .cliare/mise"));
    assert!(text.contains("1. Measure"));
    assert!(text.contains("2. For long runs"));
    assert!(text.contains("cliare jobs status --out .cliare/mise"));
    assert!(text.contains("cliare issues list --out .cliare/mise --format markdown"));
    assert!(text.contains("Rules of thumb"));
    assert!(!text.contains("| Field | Value |"));
}

#[test]
fn maintainer_playbook_uses_context_for_view_commands() {
    let args = PlaybookArgs {
        role: PlaybookRole::Maintainer,
        target: Some("rote".to_owned()),
        out: PathBuf::from(".cliare-context"),
        context: Some("authenticated".to_owned()),
        format: PlaybookFormat::Markdown,
    };

    let packet = RolePlaybook::build_maintainer(&args);
    let report_command = packet.lifecycle[1].commands[1].command.as_str();

    assert!(report_command.contains("--context authenticated"));
}

#[test]
fn harness_playbook_contains_agent_execution_loop() {
    let args = PlaybookArgs {
        role: PlaybookRole::Harness,
        target: Some("rote".to_owned()),
        out: PathBuf::from(DEFAULT_OUT_PLACEHOLDER),
        context: None,
        format: PlaybookFormat::Markdown,
    };

    let summary = playbook(args).expect("playbook renders");
    let text = summary.terminal_summary();

    assert!(text.contains("# CLIARE Harness Playbook"));
    assert!(text.contains("## 2. Read Agent Surface"));
    assert!(
        text.contains("cliare surface query 'check job status' --out .cliare/rote --format json")
    );
    assert!(text.contains("cliare surface explain 'jobs status' --out .cliare/rote --format json"));
    assert!(text.contains("cliare surface list --out .cliare/rote --state ready --format json"));
    assert!(text.contains("cliare report harness --out .cliare/rote --format markdown"));
    assert!(text.contains("cliare skills install --agent all --scope project"));
    assert!(text.contains("AGENT_SKILL.md"));
}

#[test]
fn security_playbook_contains_review_and_decision_loop() {
    let args = PlaybookArgs {
        role: PlaybookRole::Security,
        target: Some("rote".to_owned()),
        out: PathBuf::from(DEFAULT_OUT_PLACEHOLDER),
        context: None,
        format: PlaybookFormat::Human,
    };

    let summary = playbook(args).expect("playbook renders");
    let text = summary.terminal_summary();

    assert!(text.contains("CLIARE security walkthrough"));
    assert!(text.contains("1. Measure Safely"));
    assert!(text.contains("cliare report security --out .cliare/rote --format markdown"));
    assert!(
        text.contains("cliare issues mark <issue-id> --out .cliare/rote --status accepted-risk")
    );
    assert!(text.contains("Completion criteria"));
}
