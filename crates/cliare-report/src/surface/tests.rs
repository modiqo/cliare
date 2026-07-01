use super::index::{
    CommandIndexArtifact, CommandIndexCommand, CommandIndexFlag, CommandIndexGap,
    CommandIndexOutputContract, CommandIndexParameters, CommandIndexPositional,
};
use super::packets::{SurfaceExplainPacket, SurfaceListPacket, SurfaceQueryPacket};
use cliare_cli::cli::{SurfaceOutputRequirement, SurfaceReadiness};
use std::path::Path;

#[test]
fn query_matches_intent_to_command_and_synthesizes_template() {
    let index = test_index();
    let packet = SurfaceQueryPacket::build(
        Path::new(".cliare/test"),
        "check job status",
        None,
        3,
        &index,
    );

    let first = packet.matches.first().expect("query has a match");
    assert_eq!(first.command, "jobs status");
    assert_eq!(first.readiness, "conditional");
    assert_eq!(first.argv_template, ["cliare", "jobs", "status"]);
    assert!(first.why.contains("command path"));
    assert!(
        first
            .suggested_flags
            .iter()
            .any(|flag| flag.name == "--out" && flag.reason == "artifact_directory")
    );
}

#[test]
fn query_can_require_json_output() {
    let index = test_index();
    let packet = SurfaceQueryPacket::build(
        Path::new(".cliare/test"),
        "list issues",
        Some(SurfaceOutputRequirement::Json),
        3,
        &index,
    );

    let first = packet.matches.first().expect("query has a JSON match");
    assert_eq!(first.command, "issues list");
    assert_eq!(
        first.argv_template,
        ["cliare", "issues", "list", "--format", "json"]
    );
    assert!(
        first
            .cautions
            .iter()
            .any(|caution| caution.contains("json output contract is unprobed"))
    );
}

#[test]
fn explain_reports_required_positionals_and_disposition_flags() {
    let index = test_index();
    let packet = SurfaceExplainPacket::build(
        Path::new(".cliare/test"),
        vec!["issues".to_owned(), "mark".to_owned()],
        None,
        &index,
    );

    let surface = packet.surface.expect("command exists");
    assert_eq!(surface.command, "issues mark");
    assert_eq!(
        surface.argv_template,
        ["cliare", "issues", "mark", "<issue_id>"]
    );
    assert_eq!(surface.required_positionals[0].name, "issue_id");
    assert!(
        surface
            .suggested_flags
            .iter()
            .any(|flag| flag.name == "--status" && flag.reason == "disposition")
    );
}

#[test]
fn list_filters_by_readiness() {
    let index = test_index();
    let packet = SurfaceListPacket::build(
        Path::new(".cliare/test"),
        Some(SurfaceReadiness::Conditional),
        None,
        10,
        &index,
    );

    assert!(
        packet
            .commands
            .iter()
            .all(|command| command.readiness == "conditional")
    );
    assert!(
        packet
            .commands
            .iter()
            .any(|command| command.command == "jobs status")
    );
}

fn test_index() -> CommandIndexArtifact {
    CommandIndexArtifact {
        commands: vec![
            CommandIndexCommand {
                id: "cliare.jobs.status".to_owned(),
                command: "jobs status".to_owned(),
                path: vec!["jobs".to_owned(), "status".to_owned()],
                argv: vec!["cliare".to_owned(), "jobs".to_owned(), "status".to_owned()],
                summary: Some(
                    "Print the latest detached or foreground measurement progress state"
                        .to_owned(),
                ),
                runtime_state: "runtime_confirmed".to_owned(),
                agent_suitability: "conditional".to_owned(),
                suitability_reasons: vec![
                    "invalid_flag_diagnostics_unknown: safe invalid-flag probe has not observed flag diagnostics"
                        .to_owned(),
                ],
                confidence: 0.99,
                parameters: CommandIndexParameters {
                    positionals: Vec::new(),
                    flags: vec![CommandIndexFlag {
                        name: "--out".to_owned(),
                        short: None,
                        summary: Some(
                            "Measurement artifact directory containing jobs/current".to_owned(),
                        ),
                        value_name: Some("dir".to_owned()),
                        required: false,
                        repeatable: false,
                    }],
                },
                preconditions: Vec::new(),
                output_contracts: Vec::new(),
                gaps: vec![CommandIndexGap {
                    kind: "invalid_flag_diagnostics_unknown".to_owned(),
                    reason: "safe invalid-flag probe has not observed flag diagnostics"
                        .to_owned(),
                }],
                evidence: vec!["e_0001".to_owned()],
            },
            CommandIndexCommand {
                id: "cliare.issues.list".to_owned(),
                command: "issues list".to_owned(),
                path: vec!["issues".to_owned(), "list".to_owned()],
                argv: vec!["cliare".to_owned(), "issues".to_owned(), "list".to_owned()],
                summary: Some("List generated issues with maintainer dispositions".to_owned()),
                runtime_state: "runtime_confirmed".to_owned(),
                agent_suitability: "needs_fixture".to_owned(),
                suitability_reasons: vec![
                    "machine-readable output contract needs fixture or command-local validation"
                        .to_owned(),
                ],
                confidence: 0.98,
                parameters: CommandIndexParameters {
                    positionals: Vec::new(),
                    flags: vec![
                        CommandIndexFlag {
                            name: "--format".to_owned(),
                            short: None,
                            summary: Some(
                                "Output format [possible values: human, markdown, json]"
                                    .to_owned(),
                            ),
                            value_name: Some("format".to_owned()),
                            required: false,
                            repeatable: false,
                        },
                        CommandIndexFlag {
                            name: "--out".to_owned(),
                            short: None,
                            summary: Some(
                                "Measurement artifact directory containing issues.json"
                                    .to_owned(),
                            ),
                            value_name: Some("dir".to_owned()),
                            required: false,
                            repeatable: false,
                        },
                    ],
                },
                preconditions: Vec::new(),
                output_contracts: vec![CommandIndexOutputContract {
                    mode: "json".to_owned(),
                    argv_fragment: vec!["--format".to_owned(), "json".to_owned()],
                    status: "unprobed".to_owned(),
                    preconditions: Vec::new(),
                    observed_kind: None,
                    diagnostic: None,
                }],
                gaps: Vec::new(),
                evidence: vec!["e_0002".to_owned()],
            },
            CommandIndexCommand {
                id: "cliare.issues.mark".to_owned(),
                command: "issues mark".to_owned(),
                path: vec!["issues".to_owned(), "mark".to_owned()],
                argv: vec!["cliare".to_owned(), "issues".to_owned(), "mark".to_owned()],
                summary: Some("Record a maintainer disposition for an issue id".to_owned()),
                runtime_state: "runtime_confirmed".to_owned(),
                agent_suitability: "conditional".to_owned(),
                suitability_reasons: Vec::new(),
                confidence: 0.98,
                parameters: CommandIndexParameters {
                    positionals: vec![CommandIndexPositional {
                        name: "issue_id".to_owned(),
                        required: true,
                        variadic: false,
                    }],
                    flags: vec![
                        CommandIndexFlag {
                            name: "--status".to_owned(),
                            short: None,
                            summary: Some("Maintainer disposition to record".to_owned()),
                            value_name: Some("status".to_owned()),
                            required: false,
                            repeatable: false,
                        },
                        CommandIndexFlag {
                            name: "--reason".to_owned(),
                            short: None,
                            summary: Some("Maintainer rationale for the disposition".to_owned()),
                            value_name: Some("reason".to_owned()),
                            required: false,
                            repeatable: false,
                        },
                    ],
                },
                preconditions: Vec::new(),
                output_contracts: Vec::new(),
                gaps: Vec::new(),
                evidence: vec!["e_0003".to_owned()],
            },
        ],
    }
}
