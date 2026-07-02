use super::support::*;

#[test]
fn summary_accepts_artifact_directory_and_limits() {
    let cli = Cli::try_parse_from([
        "cliare",
        "summary",
        "--out",
        ".cliare/rote",
        "--context",
        "authenticated",
        "--format",
        "json",
        "--max-findings",
        "4",
        "--max-examples",
        "3",
    ])
    .expect("valid summary command");

    match cli.command {
        Command::Summary(args) => {
            assert_eq!(args.out, std::path::PathBuf::from(".cliare/rote"));
            assert_eq!(args.context.as_deref(), Some("authenticated"));
            assert_eq!(args.format, SummaryFormat::Json);
            assert_eq!(args.max_findings, 4);
            assert_eq!(args.max_examples, 3);
        }
        Command::Measure(_)
        | Command::Jobs(_)
        | Command::Guard(_)
        | Command::Benchmark(_)
        | Command::Eval(_)
        | Command::Context(_)
        | Command::Report(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_)
        | Command::Surface(_) => {
            panic!("expected summary command")
        }
    }
}
