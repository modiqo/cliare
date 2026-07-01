use super::support::*;

#[test]
fn jobs_status_accepts_output_directory() {
    let cli = Cli::try_parse_from([
        "cliare",
        "jobs",
        "status",
        "--out",
        ".cliare-target",
        "--context",
        "clean",
    ])
    .expect("valid jobs status command");

    match cli.command {
        Command::Jobs(args) => match args.command {
            JobsCommand::Status(args) => {
                assert_eq!(args.out, std::path::PathBuf::from(".cliare-target"));
                assert_eq!(args.context.as_deref(), Some("clean"));
            }
        },
        Command::Measure(_)
        | Command::Guard(_)
        | Command::Benchmark(_)
        | Command::Context(_)
        | Command::Report(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_)
        | Command::Surface(_) => {
            panic!("expected jobs command")
        }
    }
}
