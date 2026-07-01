use super::{ArgKind, metadata, metadata_text};

#[test]
fn metadata_spec_contains_report_drilldown_flags() {
    let metadata = metadata();
    let report = metadata
        .command_spec
        .root
        .subcommands
        .iter()
        .find(|command| command.name == "report")
        .expect("report command is present");

    let area = report
        .args
        .iter()
        .find(|arg| arg.long.as_deref() == Some("area"))
        .expect("area option is present");
    let issue = report
        .args
        .iter()
        .find(|arg| arg.long.as_deref() == Some("issue"))
        .expect("issue option is present");
    let with_evidence = report
        .args
        .iter()
        .find(|arg| arg.long.as_deref() == Some("with-evidence"))
        .expect("with-evidence flag is present");

    assert_eq!(area.kind, ArgKind::Option);
    assert!(
        area.possible_values
            .iter()
            .any(|value| value.value == "output-contracts")
    );
    assert_eq!(issue.kind, ArgKind::Option);
    assert_eq!(with_evidence.kind, ArgKind::Flag);
}

#[test]
fn metadata_text_lists_command_tree() {
    let text = metadata_text();

    assert!(text.contains("cliare "));
    assert!(text.contains("Full structured spec: cliare metadata --format json"));
    assert!(text.contains("- cliare measure"));
    assert!(text.contains("- cliare issues"));
    assert!(text.contains("  - cliare issues list"));
    assert!(text.contains("- cliare playbook"));
    assert!(text.contains("Usage: cliare playbook [OPTIONS] <ROLE>"));
}

#[test]
fn metadata_spec_omits_hidden_internal_args() {
    let metadata = metadata();
    let measure = metadata
        .command_spec
        .root
        .subcommands
        .iter()
        .find(|command| command.name == "measure")
        .expect("measure command is present");

    assert!(
        measure
            .args
            .iter()
            .all(|arg| arg.long.as_deref() != Some("__cliare-detached-worker"))
    );
}

#[test]
fn metadata_spec_contains_issues_commands() {
    let metadata = metadata();
    let issues = metadata
        .command_spec
        .root
        .subcommands
        .iter()
        .find(|command| command.name == "issues")
        .expect("issues command is present");
    let mark = issues
        .subcommands
        .iter()
        .find(|command| command.name == "mark")
        .expect("issues mark command is present");
    let list = issues
        .subcommands
        .iter()
        .find(|command| command.name == "list")
        .expect("issues list command is present");

    assert_eq!(
        mark.usage,
        "Usage: cliare issues mark [OPTIONS] --status <STATUS> --reason <REASON> <ISSUE_ID>"
    );
    assert!(mark.args.iter().any(|arg| {
        arg.long.as_deref() == Some("status")
            && arg
                .possible_values
                .iter()
                .any(|value| value.value == "intentional")
    }));
    assert!(
        list.args
            .iter()
            .any(|arg| arg.long.as_deref() == Some("format"))
    );
    assert!(list.args.iter().any(|arg| {
        arg.long.as_deref() == Some("format")
            && arg
                .possible_values
                .iter()
                .any(|value| value.value == "human")
    }));
}

#[test]
fn metadata_spec_contains_surface_commands() {
    let metadata = metadata();
    let surface = metadata
        .command_spec
        .root
        .subcommands
        .iter()
        .find(|command| command.name == "surface")
        .expect("surface command is present");
    let query = surface
        .subcommands
        .iter()
        .find(|command| command.name == "query")
        .expect("surface query command is present");
    let explain = surface
        .subcommands
        .iter()
        .find(|command| command.name == "explain")
        .expect("surface explain command is present");
    let list = surface
        .subcommands
        .iter()
        .find(|command| command.name == "list")
        .expect("surface list command is present");

    assert!(query.args.iter().any(|arg| arg.id == "intent"));
    assert!(query.args.iter().any(|arg| {
        arg.long.as_deref() == Some("require-output")
            && arg
                .possible_values
                .iter()
                .any(|value| value.value == "machine-readable")
    }));
    assert!(
        explain
            .args
            .iter()
            .any(|arg| arg.id == "command" && arg.value_arity.max.is_none())
    );
    assert!(list.args.iter().any(|arg| {
        arg.long.as_deref() == Some("state")
            && arg
                .possible_values
                .iter()
                .any(|value| value.value == "conditional")
    }));
}

#[test]
fn metadata_spec_contains_playbook_roles() {
    let metadata = metadata();
    let playbook = metadata
        .command_spec
        .root
        .subcommands
        .iter()
        .find(|command| command.name == "playbook")
        .expect("playbook command is present");

    assert_eq!(playbook.usage, "Usage: cliare playbook [OPTIONS] <ROLE>");
    assert!(playbook.args.iter().any(|arg| {
        arg.id == "role"
            && arg
                .possible_values
                .iter()
                .any(|value| value.value == "maintainer")
    }));
    assert!(playbook.args.iter().any(|arg| {
        arg.id == "role"
            && arg
                .possible_values
                .iter()
                .any(|value| value.value == "harness")
    }));
    assert!(playbook.args.iter().any(|arg| {
        arg.id == "role"
            && arg
                .possible_values
                .iter()
                .any(|value| value.value == "security")
    }));
    assert!(
        playbook
            .args
            .iter()
            .any(|arg| arg.long.as_deref() == Some("target"))
    );
    assert!(playbook.args.iter().any(|arg| {
        arg.long.as_deref() == Some("format")
            && arg
                .possible_values
                .iter()
                .any(|value| value.value == "human")
    }));
}

#[test]
fn metadata_spec_records_default_single_value_arity() {
    let metadata = metadata();
    let measure = metadata
        .command_spec
        .root
        .subcommands
        .iter()
        .find(|command| command.name == "measure")
        .expect("measure command is present");
    let target = measure
        .args
        .iter()
        .find(|arg| arg.id == "target")
        .expect("target positional is present");

    assert_eq!(measure.usage, "Usage: cliare measure [OPTIONS] <TARGET>");
    assert_eq!(target.value_arity.min, 1);
    assert_eq!(target.value_arity.max, Some(1));
}
