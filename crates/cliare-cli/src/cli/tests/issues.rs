use super::support::*;

#[test]
fn issues_mark_accepts_status_and_reason() {
    let cli = Cli::try_parse_from([
        "cliare",
        "issues",
        "mark",
        "issue.alternate_help_form_unavailable",
        "--out",
        ".cliare-current",
        "--context",
        "authenticated",
        "--status",
        "intentional",
        "--reason",
        "direct help is canonical",
    ])
    .expect("valid issues mark command");

    match cli.command {
        Command::Issues(args) => match args.command {
            IssuesCommand::Mark(args) => {
                assert_eq!(args.issue_id, "issue.alternate_help_form_unavailable");
                assert_eq!(args.out, std::path::PathBuf::from(".cliare-current"));
                assert_eq!(args.context.as_deref(), Some("authenticated"));
                assert_eq!(args.status, IssueDispositionStatus::Intentional);
                assert_eq!(args.reason, "direct help is canonical");
            }
            IssuesCommand::List(_) => panic!("expected mark command"),
        },
        Command::Measure(_)
        | Command::Jobs(_)
        | Command::Guard(_)
        | Command::Benchmark(_)
        | Command::Eval(_)
        | Command::Context(_)
        | Command::Report(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Playbook(_)
        | Command::Metadata(_)
        | Command::Surface(_) => {
            panic!("expected issues command")
        }
    }
}
