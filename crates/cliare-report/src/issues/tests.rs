use super::model::IssueDispositionList;
use super::render::{render_list_human, render_list_markdown};
use crate::report_format::shell_arg;
use cliare_issues::issue_disposition::{
    IssueDispositionStatus, IssueDispositions, disposition_path,
};

#[tokio::test]
async fn list_includes_dispositions_without_issue_ledger() {
    let root = std::env::temp_dir().join(format!("cliare-issues-list-test-{}", std::process::id()));
    tokio::fs::create_dir_all(&root)
        .await
        .expect("creates temp issue dir");
    let mut dispositions = IssueDispositions::default();
    dispositions.mark(
        "issue.test".to_owned(),
        IssueDispositionStatus::Intentional,
        "deliberate".to_owned(),
    );

    let packet = IssueDispositionList::build(&root, &dispositions)
        .await
        .expect("list packet builds");
    let markdown = render_list_markdown(&packet);

    assert_eq!(packet.dispositions_path, disposition_path(&root));
    assert_eq!(packet.summary.issues_total, 1);
    assert_eq!(packet.summary.reviewed_decisions, 1);
    assert!(markdown.contains("issue.test"));
    assert!(markdown.contains("intentional"));
}

#[tokio::test]
async fn list_projects_issue_commands_and_maintainer_context() {
    let root = std::env::temp_dir().join(format!(
        "cliare-issues-list-rich-test-{}",
        std::process::id()
    ));
    tokio::fs::create_dir_all(&root)
        .await
        .expect("creates temp issue dir");
    let issues = serde_json::json!({
        "issues": [
            {
                "id": "issue.invalid_flag_diagnostics_unknown",
                "status": "open",
                "title": "1 command needs clearer invalid-flag diagnostics",
                "severity": "medium",
                "category": "recovery",
                "agent_readiness_area": "diagnostics",
                "confidence": "inferred",
                "impact": "Agents depend on precise nonzero diagnostics.",
                "why_it_matters": "Diagnostics let agents repair bad command attempts.",
                "recommendation": "Reject unknown flags with clear nonzero diagnostics.",
                "verification": {
                    "command": "cliare measure mise --out .cliare --profile deep --refresh",
                    "expected_change": "The issue disappears or is dispositioned."
                },
                "affected_commands": [
                    {
                        "path": ["outdated"],
                        "argv": ["mise", "outdated"],
                        "state": "runtime_confirmed",
                        "confidence": 0.99,
                        "summary": "Shows outdated tool versions",
                        "reason": "safe invalid-flag probe has not observed flag diagnostics",
                        "required_positionals": []
                    }
                ]
            }
        ]
    });
    tokio::fs::write(
        root.join(cliare_core::artifacts::ISSUES_JSON),
        serde_json::to_vec(&issues).expect("issues fixture serializes"),
    )
    .await
    .expect("writes issues fixture");

    let packet = IssueDispositionList::build(&root, &IssueDispositions::default())
        .await
        .expect("list packet builds");
    let markdown = render_list_markdown(&packet);
    let human = render_list_human(&packet);

    assert_eq!(packet.issues.len(), 1);
    let issue = &packet.issues[0];
    assert_eq!(issue.affected_command_count, 1);
    assert_eq!(issue.command_samples[0].command, "mise outdated");
    assert_eq!(
        issue.command_samples[0].summary.as_deref(),
        Some("Shows outdated tool versions")
    );
    let normalized_command = format!(
        "cliare measure mise --out {} --profile deep --refresh",
        shell_arg(&root.display().to_string())
    );
    assert_eq!(
        issue
            .verification
            .as_ref()
            .map(|entry| entry.command.as_str()),
        Some(normalized_command.as_str())
    );
    assert!(markdown.contains("mise outdated"));
    assert!(markdown.contains("Shows outdated tool versions"));
    assert!(markdown.contains("Reject unknown flags"));
    assert!(markdown.contains(&format!(
        "cliare measure mise --out {}",
        shell_arg(&root.display().to_string())
    )));
    assert!(human.contains("Action required (1)"));
    assert!(human.contains("1 command needs clearer invalid-flag diagnostics"));
    assert!(human.contains("mise outdated: Shows outdated tool versions"));
    assert!(human.contains("Disposition examples:"));

    let _ = tokio::fs::remove_dir_all(root).await;
}
