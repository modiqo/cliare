use std::time::{SystemTime, UNIX_EPOCH};

use super::{FileChangeKind, Sandbox, SandboxProfile, SandboxRegion, SnapshotLimits};

#[tokio::test]
async fn snapshots_report_created_modified_and_deleted_files_by_region() {
    let root = unique_test_dir("sandbox-side-effects");
    let sandbox = Sandbox::create(&root).await.expect("sandbox is created");
    let execution = sandbox.execution();

    let deleted_path = sandbox.metadata().home.join("deleted");
    let modified_path = sandbox.metadata().workdir.join("modified");
    tokio::fs::write(&deleted_path, "before")
        .await
        .expect("deleted fixture is written");
    tokio::fs::write(&modified_path, "before")
        .await
        .expect("modified fixture is written");

    let before = execution
        .snapshot()
        .await
        .expect("before snapshot succeeds");

    tokio::fs::remove_file(&deleted_path)
        .await
        .expect("deleted fixture is removed");
    tokio::fs::write(&modified_path, "after")
        .await
        .expect("modified fixture is changed");
    tokio::fs::write(sandbox.metadata().tmp.join("created"), "new")
        .await
        .expect("created fixture is written");

    let after = execution.snapshot().await.expect("after snapshot succeeds");
    let diff = before.diff(&after);

    assert_eq!(diff.created, 1);
    assert_eq!(diff.modified, 1);
    assert_eq!(diff.deleted, 1);
    assert!(diff.changes.iter().any(|change| {
        change.kind == FileChangeKind::Created && change.region == SandboxRegion::Tmp
    }));
    assert!(diff.changes.iter().any(|change| {
        change.kind == FileChangeKind::Modified && change.region == SandboxRegion::Workdir
    }));
    assert!(diff.changes.iter().any(|change| {
        change.kind == FileChangeKind::Deleted && change.region == SandboxRegion::Home
    }));

    let _ = tokio::fs::remove_dir_all(root).await;
}

#[tokio::test]
async fn provided_workdir_uses_metadata_snapshots() {
    let root = unique_test_dir("sandbox-provided-workdir");
    let out_dir = root.join("out");
    let workdir = root.join("project");
    tokio::fs::create_dir_all(&workdir)
        .await
        .expect("provided workdir is created");
    let tracked = workdir.join("tracked.txt");
    tokio::fs::write(&tracked, "before")
        .await
        .expect("tracked fixture is written");

    let sandbox = Sandbox::create_with_workdir(&out_dir, Some(&workdir))
        .await
        .expect("sandbox is created with provided workdir");
    let execution = sandbox.execution();
    let before = execution
        .snapshot()
        .await
        .expect("before snapshot succeeds");

    tokio::fs::write(&tracked, "after with different length")
        .await
        .expect("tracked fixture is modified");

    let after = execution.snapshot().await.expect("after snapshot succeeds");
    let diff = before.diff(&after);

    assert_eq!(diff.modified, 1);
    let change = diff
        .changes
        .iter()
        .find(|change| change.region == SandboxRegion::Workdir)
        .expect("workdir change is reported");
    assert_eq!(change.kind, FileChangeKind::Modified);
    assert_eq!(change.sha256, None);

    let _ = tokio::fs::remove_dir_all(root).await;
}

#[tokio::test]
async fn snapshot_reports_truncation_when_file_budget_is_exhausted() {
    let root = unique_test_dir("sandbox-snapshot-budget");
    let sandbox = Sandbox::create(&root).await.expect("sandbox is created");
    let execution = sandbox.execution();

    let before = execution
        .snapshot()
        .await
        .expect("before snapshot succeeds");
    for index in 0..40 {
        tokio::fs::write(
            sandbox.metadata().tmp.join(format!("created-{index}")),
            "new",
        )
        .await
        .expect("created fixture is written");
    }
    let after = execution.snapshot().await.expect("after snapshot succeeds");
    let diff = before.diff(&after);

    assert!(diff.truncated);
    assert_eq!(
        diff.truncation_reason.as_deref(),
        Some("file_budget_exhausted")
    );

    let _ = tokio::fs::remove_dir_all(root).await;
}

#[tokio::test]
async fn snapshot_uses_limits_supplied_by_profile_configuration() {
    let root = unique_test_dir("sandbox-configured-snapshot-budget");
    let limits = SnapshotLimits::new(1, 64, 1024);
    let sandbox =
        Sandbox::create_with_profile_and_limits(&root, None, SandboxProfile::Isolated, limits)
            .await
            .expect("sandbox is created");
    let execution = sandbox
        .execution_for_probe("p_000001")
        .await
        .expect("probe execution is created");

    assert_eq!(sandbox.metadata().snapshot_limits, limits);
    assert!(!sandbox.metadata().hostile_binary_containment);
    let evidence = sandbox.probe_evidence_for(&execution);
    assert_eq!(evidence.snapshot_limits, limits);
    assert!(!evidence.hostile_binary_containment);

    let before = execution
        .snapshot()
        .await
        .expect("before snapshot succeeds");
    tokio::fs::write(execution.cwd.join("one"), "new")
        .await
        .expect("first fixture is written");
    tokio::fs::write(execution.cwd.join("two"), "new")
        .await
        .expect("second fixture is written");
    let after = execution.snapshot().await.expect("after snapshot succeeds");
    let diff = before.diff(&after);

    assert!(diff.truncated);
    assert_eq!(
        diff.truncation_reason.as_deref(),
        Some("file_budget_exhausted")
    );

    let _ = tokio::fs::remove_dir_all(root).await;
}

fn unique_test_dir(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("cliare-{name}-{}-{nonce}", std::process::id()))
}
