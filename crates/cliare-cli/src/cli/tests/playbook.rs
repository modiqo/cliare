use super::support::*;

#[test]
fn playbook_maintainer_accepts_target_context_and_format() {
    let cli = Cli::try_parse_from([
        "cliare",
        "playbook",
        "maintainer",
        "--target",
        "rote",
        "--out",
        ".cliare-context",
        "--context",
        "authenticated",
        "--format",
        "json",
    ])
    .expect("valid playbook command");

    match cli.command {
        Command::Playbook(args) => {
            assert_eq!(args.role, PlaybookRole::Maintainer);
            assert_eq!(args.target.as_deref(), Some("rote"));
            assert_eq!(args.out, std::path::PathBuf::from(".cliare-context"));
            assert_eq!(args.context.as_deref(), Some("authenticated"));
            assert_eq!(args.format, PlaybookFormat::Json);
        }
        Command::Measure(_)
        | Command::Jobs(_)
        | Command::Guard(_)
        | Command::Benchmark(_)
        | Command::Context(_)
        | Command::Report(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Issues(_)
        | Command::Metadata(_)
        | Command::Surface(_) => {
            panic!("expected playbook command")
        }
    }
}

#[test]
fn playbook_accepts_harness_and_security_roles() {
    for (role, expected) in [
        ("harness", PlaybookRole::Harness),
        ("security", PlaybookRole::Security),
    ] {
        let cli = Cli::try_parse_from(["cliare", "playbook", role]).expect("valid playbook role");

        match cli.command {
            Command::Playbook(args) => {
                assert_eq!(args.role, expected);
            }
            Command::Measure(_)
            | Command::Jobs(_)
            | Command::Guard(_)
            | Command::Benchmark(_)
            | Command::Context(_)
            | Command::Report(_)
            | Command::Describe(_)
            | Command::Skills(_)
            | Command::Issues(_)
            | Command::Metadata(_)
            | Command::Surface(_) => {
                panic!("expected playbook command")
            }
        }
    }
}

#[test]
fn playbook_help_includes_maintainer_workflow_and_profiles() {
    let mut command = Cli::command();
    let playbook = command
        .find_subcommand_mut("playbook")
        .expect("playbook command exists");
    let help = playbook.render_long_help().to_string();

    assert!(help.contains("Maintainer workflow"));
    assert!(help.contains("Available playbooks"));
    assert!(help.contains("harness"));
    assert!(help.contains("security"));
    assert!(help.contains(".cliare/<target-cli>"));
    assert!(help.contains("human"));
    assert!(help.contains("--profile quick|standard|deep"));
    assert!(help.contains("Measure profiles used by generated commands"));
    assert!(help.contains("Do not pass --profile to `cliare playbook`"));
    assert!(help.contains("quick"));
    assert!(help.contains("standard"));
    assert!(help.contains("deep"));
    assert!(help.contains("cliare report maintainer"));
    assert!(help.contains("cliare guard"));
    assert!(help.contains("cliare report harness"));
}
