use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    RuntimeContext, RuntimeContextCwdPolicy, RuntimeContextInput, RuntimeContextProfile,
    RuntimeContextState, measurement_dir, resolve_measurement_dir,
};

#[test]
fn context_measurements_write_under_contexts_directory() {
    let context = RuntimeContext::from_input(RuntimeContextInput {
        profile: Some(RuntimeContextProfile::Authenticated),
        name: None,
        auth_state: None,
        local_context_state: None,
        fixture_state: None,
        network_state: None,
        runtime_dependency_state: None,
        workdir: None,
    });

    assert_eq!(
        measurement_dir(std::path::Path::new(".cliare/rote"), &context),
        std::path::Path::new(".cliare/rote/contexts/authenticated")
    );
    assert_eq!(context.auth_state, RuntimeContextState::Present);
    assert_eq!(context.local_context_state, RuntimeContextState::Absent);
}

#[test]
fn context_workdir_marks_local_context_present() {
    let context = RuntimeContext::from_input(RuntimeContextInput {
        profile: Some(RuntimeContextProfile::LocalContext),
        name: Some("repo".to_owned()),
        auth_state: None,
        local_context_state: None,
        fixture_state: None,
        network_state: None,
        runtime_dependency_state: None,
        workdir: Some("/tmp/repo".into()),
    });

    assert_eq!(context.local_context_state, RuntimeContextState::Present);
    assert_eq!(context.cwd_policy, RuntimeContextCwdPolicy::Provided);
    assert_eq!(context.folder_name(), "repo");
}

#[tokio::test]
async fn resolver_requires_context_for_multi_context_suite_root() {
    let root = unique_test_dir("context-resolver-required");
    write_context_fixture(&root, "clean", RuntimeContextProfile::Clean).await;
    write_context_fixture(&root, "local-context", RuntimeContextProfile::LocalContext).await;

    let error = resolve_measurement_dir(&root, None, "cliare report")
        .await
        .expect_err("multi-context suite root needs explicit context");
    let message = error.to_string();

    assert!(message.contains("cliare report needs a concrete measurement context"));
    assert!(message.contains("clean"));
    assert!(message.contains("local-context"));

    let _ = tokio::fs::remove_dir_all(root).await;
}

#[tokio::test]
async fn resolver_selects_explicit_context_from_suite_root() {
    let root = unique_test_dir("context-resolver-explicit");
    let clean = write_context_fixture(&root, "clean", RuntimeContextProfile::Clean).await;
    let local =
        write_context_fixture(&root, "local-context", RuntimeContextProfile::LocalContext).await;

    assert_eq!(
        resolve_measurement_dir(&root, Some("clean"), "cliare report")
            .await
            .expect("clean context resolves"),
        clean
    );
    assert_eq!(
        resolve_measurement_dir(&root, Some("local_context"), "cliare report")
            .await
            .expect("underscore aliases sanitize to context folder"),
        local
    );

    let _ = tokio::fs::remove_dir_all(root).await;
}

#[tokio::test]
async fn resolver_rejects_missing_measurement_root() {
    let root = unique_test_dir("context-resolver-missing");

    let error = resolve_measurement_dir(&root, None, "cliare issues list")
        .await
        .expect_err("missing artifact root is rejected");
    let message = error.to_string();

    assert!(message.contains("cliare issues list could not find"));
    assert!(message.contains(&root.display().to_string()));
}

#[tokio::test]
async fn resolver_rejects_workspace_root_without_project_selection() {
    let root = unique_test_dir("context-resolver-workspace");
    tokio::fs::create_dir_all(root.join("rote"))
        .await
        .expect("creates rote workspace child");
    tokio::fs::create_dir_all(root.join("corpus-runs"))
        .await
        .expect("creates corpus workspace child");

    let error = resolve_measurement_dir(&root, None, "cliare issues list")
        .await
        .expect_err("workspace root is not a measurement");
    let message = error.to_string();

    assert!(message.contains("does not contain scorecard.json"));
    assert!(message.contains("corpus-runs, rote"));
    assert!(message.contains("Pass --out"));
    assert!(message.contains("<project>"));

    let _ = tokio::fs::remove_dir_all(root).await;
}

async fn write_context_fixture(
    root: &std::path::Path,
    name: &str,
    profile: RuntimeContextProfile,
) -> std::path::PathBuf {
    let dir = root.join("contexts").join(name);
    tokio::fs::create_dir_all(&dir)
        .await
        .expect("context fixture directory is created");
    tokio::fs::write(dir.join("scorecard.json"), "{}")
        .await
        .expect("scorecard marker is written");
    let context = RuntimeContext::from_input(RuntimeContextInput {
        profile: Some(profile),
        name: Some(name.to_owned()),
        auth_state: None,
        local_context_state: None,
        fixture_state: None,
        network_state: None,
        runtime_dependency_state: None,
        workdir: None,
    });
    let bytes = serde_json::to_vec(&context).expect("runtime context serializes");
    tokio::fs::write(dir.join("runtime-context.json"), bytes)
        .await
        .expect("runtime context is written");
    dir
}

fn unique_test_dir(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("cliare-{name}-{}-{nonce}", std::process::id()))
}
