use super::health::command_health;
use super::packets::ReportSelection;
use super::recommendations::persona_priority;
use super::{
    ActionCategory, ActionSeverity, AgentReadinessArea, CommandIndexArtifact, CommandIndexCommand,
    CommandIndexGap, CommandIndexParameters, CommandIndexSummaryArtifact, CommandReadinessState,
    Issue, IssueConfidence, IssueVerification, Persona,
};

#[test]
fn persona_priority_matches_primary_users() {
    assert!(
        persona_priority(Persona::Security, ActionCategory::Safety)
            < persona_priority(Persona::Security, ActionCategory::Output)
    );
    assert!(
        persona_priority(Persona::Harness, ActionCategory::Output)
            < persona_priority(Persona::Harness, ActionCategory::Recovery)
    );
}

#[test]
fn command_health_uses_command_index_readiness() {
    let index = CommandIndexArtifact {
        summary: CommandIndexSummaryArtifact {
            ready: 1,
            conditional: 0,
            needs_fixture: 0,
            blocked: 0,
            candidate: 0,
        },
        commands: vec![CommandIndexCommand {
            id: "rote.flow.list".to_owned(),
            path: vec!["flow".to_owned(), "list".to_owned()],
            argv: vec!["rote".to_owned(), "flow".to_owned(), "list".to_owned()],
            summary: Some("List flows".to_owned()),
            runtime_state: "runtime_confirmed".to_owned(),
            agent_suitability: "ready".to_owned(),
            suitability_reasons: vec![
                "runtime-confirmed with parseable machine-readable output".to_owned(),
            ],
            confidence: 0.99,
            parameters: CommandIndexParameters::default(),
            preconditions: Vec::new(),
            output_contracts: Vec::new(),
            gaps: vec![CommandIndexGap {
                kind: "alternate_help_form_unavailable".to_owned(),
                reason: "optional `help <command path>` probe did not resolve this command"
                    .to_owned(),
                evidence: vec!["e_000345".to_owned()],
            }],
            evidence: vec!["e_000889".to_owned()],
        }],
    };

    let health = command_health(&index);

    assert_eq!(health.len(), 1);
    assert_eq!(health[0].readiness_state, CommandReadinessState::Ready);
    assert_eq!(health[0].gaps[0].kind, "alternate_help_form_unavailable");
    assert_eq!(
        health[0].suitability_reasons,
        ["runtime-confirmed with parseable machine-readable output"]
    );
}

#[test]
fn report_selection_area_is_persona_scoped() {
    let issues = vec![
        issue(
            "issue.output_mode_unprobed",
            AgentReadinessArea::OutputContracts,
            &[Persona::Maintainer],
        ),
        issue(
            "issue.security_side_effect",
            AgentReadinessArea::Safety,
            &[Persona::Security],
        ),
    ];

    let selected = ReportSelection::Area(AgentReadinessArea::OutputContracts)
        .select(Persona::Maintainer, &issues);

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id, "issue.output_mode_unprobed");
}

#[test]
fn report_selection_issue_id_is_exact() {
    let issues = vec![
        issue(
            "issue.output_mode_unprobed",
            AgentReadinessArea::OutputContracts,
            &[Persona::Maintainer],
        ),
        issue(
            "issue.output_mode_parse_failed",
            AgentReadinessArea::OutputContracts,
            &[Persona::Maintainer],
        ),
    ];

    let selected = ReportSelection::Issue("issue.output_mode_parse_failed".to_owned())
        .select(Persona::Maintainer, &issues);

    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id, "issue.output_mode_parse_failed");
}

fn issue(id: &str, area: AgentReadinessArea, personas: &[Persona]) -> Issue {
    Issue {
        id: id.to_owned(),
        status: "open",
        severity: ActionSeverity::Medium,
        category: ActionCategory::Output,
        agent_readiness_area: area,
        confidence: IssueConfidence::Observed,
        title: "Issue title".to_owned(),
        impact: "impact".to_owned(),
        why_it_matters: "why".to_owned(),
        recommendation: "recommendation".to_owned(),
        verification: IssueVerification {
            command: "cliare measure target".to_owned(),
            expected_change: "expected".to_owned(),
        },
        affected_commands: Vec::new(),
        evidence: Vec::new(),
        disposition: None,
        personas: personas.to_vec(),
        score_dimensions: Vec::new(),
    }
}
