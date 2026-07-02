use std::path::Path;

use super::actions::{ci_action_text, issue_meaning, render_maintainer_action_item};
use super::findings::{
    render_maintainer_finding, render_persona_finding, render_persona_finding_row,
};
use super::guidance::{persona_issue_action, use_when_text};
use super::samples::command_section_heading;
use super::summary::render_plain_english_guide;
use crate::report_model::{
    ActionCategory, ActionSeverity, AgentReadinessArea, Issue, IssueCommand, IssueConfidence,
    IssueVerification, Persona,
};

#[test]
fn plain_english_guide_frames_agent_navigation_as_evidence() {
    let mut guide = String::new();
    render_plain_english_guide(&mut guide);

    assert!(guide.contains("CLIARE reports evidence for agent navigation capabilities"));
    assert!(guide.contains("just because an agent could explore it by trial and error"));
    assert!(guide.contains("| Discovery | Runtime-confirmed help"));
    assert!(guide.contains("| Safety | Help, version, diagnostic"));
}

#[test]
fn inferred_runtime_confirmed_issues_are_commands_to_verify() {
    let issue = test_issue(
        IssueConfidence::Inferred,
        ActionCategory::Discovery,
        Some("runtime_confirmed"),
    );

    assert_eq!(command_section_heading(&issue), "Commands to verify");
    assert!(
        persona_issue_action(Persona::Maintainer, &issue).starts_with("Verify the measured gap")
    );
}

#[test]
fn inferred_unconfirmed_issues_are_candidate_examples() {
    let issue = test_issue(
        IssueConfidence::Inferred,
        ActionCategory::Discovery,
        Some("inferred"),
    );

    assert_eq!(
        command_section_heading(&issue),
        "Candidate examples to review"
    );
    assert!(
        persona_issue_action(Persona::Harness, &issue)
            .starts_with("Do not expose inferred candidates")
    );
}

#[test]
fn harness_safety_action_is_profile_scoped() {
    let issue = test_issue(IssueConfidence::Observed, ActionCategory::Safety, None);

    let action = persona_issue_action(Persona::Harness, &issue);
    assert!(action.starts_with("Do not run this probe profile"));
    assert!(!action.contains("affected commands"));
}

#[test]
fn persona_issue_markdown_is_table_row_with_drilldown() {
    let issue = test_issue(
        IssueConfidence::Observed,
        ActionCategory::Safety,
        Some("runtime_confirmed"),
    );

    let mut row = String::new();
    render_persona_finding_row(&mut row, Persona::Harness, &issue, 1);
    assert!(
        row.starts_with("| P1 | CLIARE directly saw this runtime behavior; review the evidence")
    );
    assert!(row.contains("`open` | 1 | `issue.test` Test issue"));
    assert!(row.contains("Do not run this probe profile"));

    let mut detail = String::new();
    render_persona_finding(&mut detail, Persona::Harness, &issue, 1);
    assert!(detail.contains("<details>"));
    assert!(detail.contains("<summary>P1: Test issue (`issue.test`)</summary>"));
    assert!(detail.contains("- Assessment: `issue.test` Test issue"));
    assert!(detail.contains("- Meaning: impact"));
    assert!(
        detail.contains("- Evidence interpretation: CLIARE directly saw this runtime behavior")
    );
    assert!(detail.contains("- Associated commands: `1` affected."));
    assert!(detail.contains("- Suggested remedy: recommendation"));
    assert!(detail.contains("</details>"));
}

#[test]
fn maintainer_issue_markdown_uses_agent_readiness_area() {
    let mut issue = test_issue(
        IssueConfidence::NeedsFixture,
        ActionCategory::Output,
        Some("runtime_confirmed"),
    );
    issue.agent_readiness_area = AgentReadinessArea::OutputContracts;

    let mut action = String::new();
    render_maintainer_action_item(&mut action, Path::new(".cliare/test"), &issue, 1);
    assert!(action.starts_with("### 1. Test issue"));
    assert!(action.contains("- Issue: `issue.test` (`medium`, 1 affected)."));
    assert!(action.contains("- Agent outcome: Agents cannot safely read command results"));
    assert!(action.contains("- Fix: Add a safe validation path"));
    assert!(action.contains("- If acceptable: record a disposition"));
    assert!(action.contains("cliare issues mark issue.test --out .cliare/test"));
    assert!(action.contains("- Evidence: 1 affected command. Open the drill-down for context."));
    assert!(action.contains("- Verify: `cliare measure test`"));

    let mut detail = String::new();
    render_maintainer_finding(&mut detail, Path::new(".cliare/test"), &issue);
    assert!(detail.contains("<summary>Output Contracts: Test issue (`issue.test`)</summary>"));
    assert!(detail.contains("- Assessment: `issue.test` Test issue"));
    assert!(detail.contains("- Meaning: Agents cannot safely read command results"));
    assert!(detail.contains("- Associated commands: `1` affected."));
    assert!(detail.contains("- Suggested remedy: Add a safe validation path"));
    assert!(detail.contains("- If acceptable: record a disposition"));
    assert!(detail.contains("- Area: Output Contracts"));
    assert!(!detail.contains("P1"));
}

#[test]
fn issue_meaning_translates_report_conditions() {
    assert!(
        issue_meaning(&test_issue(
            IssueConfidence::Blocked,
            ActionCategory::Discovery,
            Some("runtime_confirmed"),
        ))
        .contains("setup or required inputs blocked")
    );
    assert!(
        issue_meaning(&test_issue(
            IssueConfidence::Inferred,
            ActionCategory::Discovery,
            None,
        ))
        .contains("inferred this from help text")
    );
}

#[test]
fn ci_action_text_makes_fixture_work_actionable() {
    let mut issue = test_issue(
        IssueConfidence::NeedsFixture,
        ActionCategory::Output,
        Some("runtime_confirmed"),
    );
    issue.affected_commands[0]
        .required_positionals
        .push("persona".to_owned());

    let action = ci_action_text(Persona::Devrel, &issue);

    assert!(action.contains("safe sample operand or fixture profile"));
    assert!(action.contains("advertised contract"));
}

#[test]
fn use_when_text_removes_redundant_prefix() {
    assert_eq!(
        use_when_text("Use before exposing a CLI subset to agents."),
        "before exposing a CLI subset to agents."
    );
    assert_eq!(
        use_when_text("whenever policy changes."),
        "whenever policy changes."
    );
}

fn test_issue(
    confidence: IssueConfidence,
    category: ActionCategory,
    command_state: Option<&str>,
) -> Issue {
    Issue {
        id: "issue.test".to_owned(),
        status: "open",
        severity: ActionSeverity::Medium,
        category,
        agent_readiness_area: AgentReadinessArea::Diagnostics,
        confidence,
        title: "Test issue".to_owned(),
        impact: "impact".to_owned(),
        why_it_matters: "why".to_owned(),
        recommendation: "recommendation".to_owned(),
        verification: IssueVerification {
            command: "cliare measure test".to_owned(),
            expected_change: "expected".to_owned(),
        },
        affected_commands: command_state
            .map(|state| {
                vec![IssueCommand {
                    path: vec!["cmd".to_owned()],
                    argv: vec!["target".to_owned(), "cmd".to_owned()],
                    state: state.to_owned(),
                    confidence: Some(0.8),
                    summary: None,
                    required_positionals: Vec::new(),
                    output_contracts: Vec::new(),
                    reason: "reason".to_owned(),
                }]
            })
            .unwrap_or_default(),
        evidence: Vec::new(),
        disposition: None,
        personas: Vec::new(),
        score_dimensions: Vec::new(),
    }
}
