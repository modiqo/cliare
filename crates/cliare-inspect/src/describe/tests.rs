use std::fs;

use cliare_cli::cli::{DescribeArgs, DescribeFormat};

#[tokio::test]
async fn describe_measurement_directory_writes_artifact_map() {
    let folder = std::env::temp_dir().join(format!("cliare-describe-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).expect("creates fixture directory");
    fs::write(
        folder.join("scorecard.json"),
        r#"{
  "schema_version": "cliare.scorecard.v1",
  "score": {"total": 82, "status": "experimental_partial", "model": "cliare-score-v0"},
  "coverage": {"probes_completed": 7, "max_probes": 64, "traversal_complete": true, "budget_exhausted": false}
}"#,
    )
    .expect("writes scorecard");
    fs::write(
        folder.join("command-index.json"),
        r#"{"schema_version":"cliare.command-index.v1","commands":[{"runtime_state":"runtime_confirmed","agent_suitability":"ready","preconditions":[]}]}"#,
    )
    .expect("writes command index");
    fs::write(
        folder.join("issues.json"),
        r#"{"schema_version":"cliare.issues.v1","issues":[{"severity":"high","confidence":"observed"}]}"#,
    )
    .expect("writes issues");
    fs::write(
        folder.join("shape.json"),
        r#"{"schema_version":"cliare.command-shape.v1"}"#,
    )
    .expect("writes shape");
    fs::write(folder.join("evidence.jsonl"), "{}\n{}\n").expect("writes evidence");

    let summary = super::describe(DescribeArgs {
        folder: folder.clone(),
        context: None,
        format: DescribeFormat::Markdown,
        write: true,
    })
    .await
    .expect("describe succeeds");

    assert_eq!(summary.artifact_kind, super::ArtifactKind::Measurement);
    assert_eq!(summary.missing_required, 0);
    assert!(summary.terminal_summary().contains("CLIARE Artifact Map"));
    assert!(folder.join("artifact-map.json").is_file());
    assert!(folder.join("artifact-map.md").is_file());

    let _ = fs::remove_dir_all(folder);
}

#[tokio::test]
async fn describe_context_suite_root_lists_persisted_contexts() {
    let folder = std::env::temp_dir().join(format!(
        "cliare-describe-context-suite-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&folder);
    let clean = folder.join("contexts/clean");
    let local = folder.join("contexts/local-context");
    fs::create_dir_all(&clean).expect("creates clean context");
    fs::create_dir_all(&local).expect("creates local context");
    fs::write(folder.join("context-suite.json"), "{}").expect("writes suite");
    fs::write(folder.join("context-compare.md"), "# compare\n").expect("writes comparison");
    fs::write(clean.join("scorecard.json"), "{}").expect("writes clean scorecard");
    fs::write(local.join("scorecard.json"), "{}").expect("writes local scorecard");
    fs::write(
        clean.join("runtime-context.json"),
        r#"{"schema_version":"cliare.runtime-context.v1","profile":"clean","name":"clean","auth_state":"absent","local_context_state":"absent","fixture_state":"absent","network_state":"unknown","runtime_dependency_state":"unknown","cwd_policy":"isolated","workdir":null,"declared_by":"cli"}"#,
    )
    .expect("writes clean runtime context");
    fs::write(
        local.join("runtime-context.json"),
        r#"{"schema_version":"cliare.runtime-context.v1","profile":"local_context","name":"local-context","auth_state":"unknown","local_context_state":"present","fixture_state":"absent","network_state":"unknown","runtime_dependency_state":"unknown","cwd_policy":"provided","workdir":"/tmp/project","declared_by":"cli"}"#,
    )
    .expect("writes local runtime context");

    let summary = super::describe(DescribeArgs {
        folder: folder.clone(),
        context: None,
        format: DescribeFormat::Markdown,
        write: false,
    })
    .await
    .expect("describe succeeds");

    assert_eq!(summary.artifact_kind, super::ArtifactKind::ContextSuite);
    assert!(summary.terminal_summary().contains("Persisted contexts: 2"));
    assert!(summary.terminal_summary().contains("contexts/clean"));
    assert!(
        summary
            .terminal_summary()
            .contains("contexts/local-context")
    );

    let _ = fs::remove_dir_all(folder);
}
