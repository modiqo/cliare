use super::support::*;

#[test]
fn report_accepts_persona_and_output_options() {
    let cli = Cli::try_parse_from([
        "cliare",
        "report",
        "security",
        "--out",
        ".cliare-current",
        "--context",
        "clean",
        "--format",
        "json",
        "--write",
    ])
    .expect("valid report command");

    match cli.command {
        Command::Report(args) => {
            assert_eq!(args.persona, ReportPersona::Security);
            assert_eq!(args.out, std::path::PathBuf::from(".cliare-current"));
            assert_eq!(args.context.as_deref(), Some("clean"));
            assert_eq!(args.format, ReportFormat::Json);
            assert!(args.write);
        }
        Command::Measure(_)
        | Command::Jobs(_)
        | Command::Guard(_)
        | Command::Benchmark(_)
        | Command::Eval(_)
        | Command::Context(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_)
        | Command::Surface(_) => {
            panic!("expected report command")
        }
    }
}

#[test]
fn report_accepts_typed_drilldown_options() {
    let cli = Cli::try_parse_from([
        "cliare",
        "report",
        "maintainer",
        "--area",
        "output-contracts",
        "--with-evidence",
        "--format",
        "bundle",
    ])
    .expect("valid focused report command");

    match cli.command {
        Command::Report(args) => {
            assert_eq!(args.persona, ReportPersona::Maintainer);
            assert_eq!(args.area, Some(ReportArea::OutputContracts));
            assert_eq!(args.issue, None);
            assert!(args.with_evidence);
            assert_eq!(args.format, ReportFormat::Bundle);
        }
        Command::Measure(_)
        | Command::Jobs(_)
        | Command::Guard(_)
        | Command::Benchmark(_)
        | Command::Eval(_)
        | Command::Context(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_)
        | Command::Surface(_) => {
            panic!("expected report command")
        }
    }
}

#[test]
fn report_rejects_area_and_issue_together() {
    let error = Cli::try_parse_from([
        "cliare",
        "report",
        "maintainer",
        "--area",
        "output-contracts",
        "--issue",
        "issue.output_mode_unprobed",
    ])
    .expect_err("area and issue filters are mutually exclusive");

    assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
}
