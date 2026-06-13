#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cliare::cli::{GuardArgs, MeasureArgs};
use serde_json::Value;

#[tokio::test]
async fn custom_help_tree_confirms_nested_commands_and_aliases() {
    let artifacts = measure_fixture(
        "custom_help_tree_confirms_nested_commands_and_aliases",
        custom_help_tree_script(),
        64,
    )
    .await;

    let project_list = command(&artifacts.shape, &["project", "list"]);
    assert!(project_list["runtime_confirmed"].as_bool().unwrap_or(false));
    assert!(project_list["confidence"].as_f64().unwrap_or(0.0) > 0.90);
    assert!(has_flag(&artifacts.shape, &["project", "list"], "--format"));

    let rm = command(&artifacts.shape, &["rm"]);
    assert!(rm["runtime_confirmed"].as_bool().unwrap_or(false));
    let remove = command(&artifacts.shape, &["remove"]);
    assert!(remove["runtime_confirmed"].as_bool().unwrap_or(false));

    assert!(artifacts.evidence.contains("\"intent\":\"invalid_flag\""));
}

#[tokio::test]
async fn false_positive_help_rows_are_de_rated_by_runtime_evidence() {
    let artifacts = measure_fixture(
        "false_positive_help_rows_are_de_rated_by_runtime_evidence",
        custom_help_tree_script(),
        64,
    )
    .await;

    let env_var = command(&artifacts.shape, &["API_TOKEN"]);
    assert!(!env_var["runtime_confirmed"].as_bool().unwrap_or(true));
    assert!(env_var["confidence"].as_f64().unwrap_or(1.0) < 0.20);
    assert!(has_gap(
        &artifacts.shape,
        &["API_TOKEN"],
        "existence_unconfirmed"
    ));
    assert!(has_gap(
        &artifacts.shape,
        &["API_TOKEN"],
        "help_unavailable"
    ));
}

#[tokio::test]
async fn noisy_help_still_infers_from_stdout_layout() {
    let artifacts = measure_fixture(
        "noisy_help_still_infers_from_stdout_layout",
        noisy_help_script(),
        32,
    )
    .await;

    let run = command(&artifacts.shape, &["run"]);
    assert!(run["runtime_confirmed"].as_bool().unwrap_or(false));
    assert!(artifacts.evidence.contains("startup warning"));
    assert!(
        artifacts.scorecard["score"]["total"]
            .as_f64()
            .unwrap_or(0.0)
            > 0.0
    );
    assert_eq!(
        artifacts.scorecard["coverage"]["max_depth"].as_u64(),
        Some(2)
    );
    assert_eq!(
        artifacts.scorecard["coverage"]["max_probes"].as_u64(),
        Some(32)
    );
    assert!(
        artifacts.scorecard["coverage"]["observed_max_depth"]
            .as_u64()
            .unwrap_or(0)
            >= 1
    );
    assert!(
        artifacts.scorecard["coverage"]["frontier_remaining"]
            .as_u64()
            .is_some()
    );
    assert!(artifacts.report.contains("# CLIARE Report"));
    assert!(artifacts.report.contains("not measured"));
    assert!(artifacts.report.contains("Budget exhausted"));
}

#[tokio::test]
async fn clearer_cli_scores_higher_than_poor_cli() {
    let poor = measure_fixture(
        "clearer_cli_scores_higher_than_poor_cli_poor",
        poor_help_script(),
        16,
    )
    .await;
    let clear = measure_fixture(
        "clearer_cli_scores_higher_than_poor_cli_clear",
        noisy_help_script(),
        32,
    )
    .await;

    assert!(
        clear.scorecard["score"]["total"].as_f64().unwrap_or(0.0)
            > poor.scorecard["score"]["total"].as_f64().unwrap_or(100.0)
    );
}

#[tokio::test]
async fn guard_passes_against_same_score_baseline() {
    let workspace = TempWorkspace::new("guard_passes_against_same_score_baseline");
    let target = workspace.write_executable("fixture-cli", noisy_help_script());
    let baseline_out = workspace.path().join("baseline");
    let guard_out = workspace.path().join("guard");

    cliare::measure::measure(MeasureArgs {
        target: target.clone(),
        out: baseline_out.clone(),
        timeout_ms: 1_000,
        output_limit_bytes: 64 * 1024,
        max_depth: 2,
        max_probes: 32,
    })
    .await
    .expect("baseline measurement succeeds");

    let summary = cliare::guard::guard(GuardArgs {
        target,
        baseline: baseline_out.join("scorecard.json"),
        out: guard_out,
        allowed_drop: 0.0,
        timeout_ms: 1_000,
        output_limit_bytes: 64 * 1024,
        max_depth: 2,
        max_probes: 32,
    })
    .await
    .expect("guard measurement succeeds");

    assert!(summary.passed);
    assert!(summary.terminal_summary().contains("result: pass"));
}

#[tokio::test]
async fn guard_fails_when_score_drops_beyond_allowed_threshold() {
    let workspace = TempWorkspace::new("guard_fails_when_score_drops_beyond_allowed_threshold");
    let target = workspace.write_executable("fixture-cli", poor_help_script());
    let baseline = workspace.path().join("baseline.scorecard.json");
    fs::write(
        &baseline,
        r#"{"schema_version":"cliare.scorecard.v1","score":{"total":100.0}}"#,
    )
    .expect("baseline scorecard is written");

    let summary = cliare::guard::guard(GuardArgs {
        target,
        baseline,
        out: workspace.path().join("guard"),
        allowed_drop: 1.0,
        timeout_ms: 1_000,
        output_limit_bytes: 64 * 1024,
        max_depth: 2,
        max_probes: 16,
    })
    .await
    .expect("guard measurement succeeds");

    assert!(!summary.passed);
    assert!(summary.delta < -1.0);
    assert!(summary.terminal_summary().contains("result: fail"));
}

async fn measure_fixture(name: &str, script: &str, max_probes: usize) -> MeasuredFixture {
    let workspace = TempWorkspace::new(name);
    let target = workspace.write_executable("fixture-cli", script);
    let out = workspace.path().join("artifacts");

    cliare::measure::measure(MeasureArgs {
        target,
        out: out.clone(),
        timeout_ms: 1_000,
        output_limit_bytes: 64 * 1024,
        max_depth: 2,
        max_probes,
    })
    .await
    .expect("measurement succeeds");

    let shape = read_json(&out.join("shape.json"));
    let scorecard = read_json(&out.join("scorecard.json"));
    let report = fs::read_to_string(out.join("report.md")).expect("report is readable");
    let evidence =
        fs::read_to_string(out.join("evidence.jsonl")).expect("evidence log is readable");

    MeasuredFixture {
        _workspace: workspace,
        shape,
        scorecard,
        report,
        evidence,
    }
}

struct MeasuredFixture {
    _workspace: TempWorkspace,
    shape: Value,
    scorecard: Value,
    report: String,
    evidence: String,
}

struct TempWorkspace {
    path: PathBuf,
}

impl TempWorkspace {
    fn new(name: &str) -> Self {
        let unique = format!(
            "cliare-fixture-{}-{}-{}",
            name,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time is after unix epoch")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).expect("temporary workspace is created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write_executable(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, contents).expect("fixture script is written");
        let mut permissions = fs::metadata(&path)
            .expect("fixture metadata exists")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("fixture script is executable");
        path
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn read_json(path: &Path) -> Value {
    let text = fs::read_to_string(path).expect("json artifact is readable");
    serde_json::from_str(&text).expect("json artifact parses")
}

fn command<'a>(shape: &'a Value, path: &[&str]) -> &'a Value {
    shape["commands"]
        .as_array()
        .expect("shape commands is an array")
        .iter()
        .find(|command| path_matches(&command["path"], path))
        .unwrap_or_else(|| panic!("command path not found: {path:?}"))
}

fn has_flag(shape: &Value, command_path: &[&str], name: &str) -> bool {
    shape["flags"]
        .as_array()
        .expect("shape flags is an array")
        .iter()
        .any(|flag| {
            path_matches(&flag["command_path"], command_path) && flag["name"].as_str() == Some(name)
        })
}

fn has_gap(shape: &Value, command_path: &[&str], kind: &str) -> bool {
    shape["gaps"]
        .as_array()
        .expect("shape gaps is an array")
        .iter()
        .any(|gap| {
            path_matches(&gap["command_path"], command_path) && gap["kind"].as_str() == Some(kind)
        })
}

fn path_matches(value: &Value, expected: &[&str]) -> bool {
    value.as_array().is_some_and(|actual| {
        actual
            .iter()
            .map(Value::as_str)
            .eq(expected.iter().copied().map(Some))
    })
}

fn custom_help_tree_script() -> &'static str {
    r#"#!/bin/sh

root_help() {
  cat <<'EOF'
TOOLS:
  project list    List projects
  rm, remove      Remove an item
  API_TOKEN       Environment variable for authentication

OPTIONS:
  --help          Show help
EOF
}

project_help() {
  cat <<'EOF'
Commands:
  list  List projects

Options:
  --help  Show help
EOF
}

project_list_help() {
  cat <<'EOF'
Usage: fixture-cli project list [OPTIONS]

Options:
  --format <KIND>  Output format
  --help           Show help
EOF
}

remove_help() {
  cat <<'EOF'
Usage: fixture-cli remove <ID>

Options:
  --force  Skip confirmation
  --help   Show help
EOF
}

case "$1" in
  ""|"--help"|"-h")
    root_help
    exit 0
    ;;
  "help")
    case "$2 $3" in
      "project list")
        project_list_help
        exit 0
        ;;
    esac
    case "$2" in
      "")
        root_help
        exit 0
        ;;
      "project")
        project_help
        exit 0
        ;;
      "rm"|"remove")
        remove_help
        exit 0
        ;;
      *)
        echo "unknown help topic: $2" >&2
        exit 2
        ;;
    esac
    ;;
  "--version"|"version")
    echo "fixture-cli 1.0.0"
    exit 0
    ;;
  --__cliare_unknown_*)
    echo "unknown option: $1" >&2
    exit 2
    ;;
  __cliare_unknown_*)
    echo "unknown command: $1" >&2
    exit 2
    ;;
  "project")
    case "$2" in
      ""|"--help")
        project_help
        exit 0
        ;;
      "list")
        case "$3" in
          ""|"--help")
            project_list_help
            exit 0
            ;;
          --__cliare_unknown_*)
            echo "unknown option: $3" >&2
            exit 2
            ;;
          *)
            echo "unexpected argument: $3" >&2
            exit 2
            ;;
        esac
        ;;
      __cliare_unknown_*)
        echo "unknown child: $2" >&2
        exit 2
        ;;
      *)
        echo "unknown project command: $2" >&2
        exit 2
        ;;
    esac
    ;;
  "rm"|"remove")
    case "$2" in
      ""|"--help")
        remove_help
        exit 0
        ;;
      --__cliare_unknown_*)
        echo "unknown option: $2" >&2
        exit 2
        ;;
      *)
        echo "unexpected remove argument: $2" >&2
        exit 2
        ;;
    esac
    ;;
  "API_TOKEN")
    echo "unknown command: API_TOKEN" >&2
    exit 2
    ;;
  *)
    echo "unknown command: $1" >&2
    exit 2
    ;;
esac
"#
}

fn noisy_help_script() -> &'static str {
    r#"#!/bin/sh

case "$1" in
  ""|"--help"|"-h"|"help")
    echo "startup warning: config file not found" >&2
    cat <<'EOF'
Commands:
  run  Run the job

Options:
  --help  Show help
EOF
    exit 0
    ;;
  "run")
    case "$2" in
      ""|"--help")
        cat <<'EOF'
Usage: fixture-cli run [OPTIONS]

Options:
  --dry-run  Do not write changes
  --help     Show help
EOF
        exit 0
        ;;
      --__cliare_unknown_*)
        echo "unknown option: $2" >&2
        exit 2
        ;;
      *)
        echo "unexpected argument: $2" >&2
        exit 2
        ;;
    esac
    ;;
  "--version"|"version")
    echo "fixture-cli 1.0.0"
    exit 0
    ;;
  *)
    echo "unknown command: $1" >&2
    exit 2
    ;;
esac
"#
}

fn poor_help_script() -> &'static str {
    r#"#!/bin/sh

case "$1" in
  "--version"|"version")
    echo "fixture-cli 1.0.0"
    exit 0
    ;;
  --__cliare_unknown_*|__cliare_unknown_*)
    echo "bad input"
    exit 0
    ;;
  *)
    echo "try docs"
    exit 0
    ;;
esac
"#
}
