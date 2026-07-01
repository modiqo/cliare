use super::support::*;

#[test]
fn skills_install_accepts_agent_scope_and_dry_run() {
    let cli = Cli::try_parse_from([
        "cliare",
        "skills",
        "install",
        "--agent",
        "claude",
        "--scope",
        "project",
        "--project-dir",
        ".",
        "--dry-run",
    ])
    .expect("valid skills install command");

    match cli.command {
        Command::Skills(args) => match args.command {
            SkillsCommand::Install(args) => {
                assert_eq!(args.agent, SkillAgent::Claude);
                assert_eq!(args.scope, SkillInstallScope::Project);
                assert_eq!(args.project_dir, Some(std::path::PathBuf::from(".")));
                assert!(args.dry_run);
            }
            SkillsCommand::List(_) => panic!("expected install command"),
        },
        Command::Measure(_)
        | Command::Jobs(_)
        | Command::Guard(_)
        | Command::Benchmark(_)
        | Command::Eval(_)
        | Command::Context(_)
        | Command::Report(_)
        | Command::Describe(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_)
        | Command::Surface(_) => {
            panic!("expected skills command")
        }
    }
}
