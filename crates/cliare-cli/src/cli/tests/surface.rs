use super::support::*;

#[test]
fn surface_query_accepts_intent_filters_and_format() {
    let cli = Cli::try_parse_from([
        "cliare",
        "surface",
        "query",
        "check job status",
        "--out",
        ".cliare-current",
        "--context",
        "clean",
        "--require-output",
        "json",
        "--limit",
        "3",
        "--format",
        "human",
    ])
    .expect("valid surface query command");

    match cli.command {
        Command::Surface(args) => match args.command {
            SurfaceCommand::Query(args) => {
                assert_eq!(args.intent, "check job status");
                assert_eq!(args.out, std::path::PathBuf::from(".cliare-current"));
                assert_eq!(args.context.as_deref(), Some("clean"));
                assert_eq!(args.require_output, Some(SurfaceOutputRequirement::Json));
                assert_eq!(args.limit, 3);
                assert_eq!(args.format, SurfaceFormat::Human);
            }
            SurfaceCommand::Explain(_) | SurfaceCommand::List(_) => {
                panic!("expected query command")
            }
        },
        Command::Measure(_)
        | Command::Jobs(_)
        | Command::Guard(_)
        | Command::Benchmark(_)
        | Command::Context(_)
        | Command::Report(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_) => {
            panic!("expected surface command")
        }
    }
}

#[test]
fn surface_explain_accepts_command_path_and_output_request() {
    let cli = Cli::try_parse_from([
        "cliare",
        "surface",
        "explain",
        "jobs",
        "status",
        "--require-output",
        "machine-readable",
    ])
    .expect("valid surface explain command");

    match cli.command {
        Command::Surface(args) => match args.command {
            SurfaceCommand::Explain(args) => {
                assert_eq!(args.command, vec!["jobs".to_owned(), "status".to_owned()]);
                assert_eq!(
                    args.require_output,
                    Some(SurfaceOutputRequirement::MachineReadable)
                );
                assert_eq!(args.format, SurfaceFormat::Json);
            }
            SurfaceCommand::Query(_) | SurfaceCommand::List(_) => {
                panic!("expected explain command")
            }
        },
        Command::Measure(_)
        | Command::Jobs(_)
        | Command::Guard(_)
        | Command::Benchmark(_)
        | Command::Context(_)
        | Command::Report(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_) => {
            panic!("expected surface command")
        }
    }
}

#[test]
fn surface_list_accepts_readiness_filter() {
    let cli = Cli::try_parse_from([
        "cliare", "surface", "list", "--state", "ready", "--limit", "12",
    ])
    .expect("valid surface list command");

    match cli.command {
        Command::Surface(args) => match args.command {
            SurfaceCommand::List(args) => {
                assert_eq!(args.state, Some(SurfaceReadiness::Ready));
                assert_eq!(args.limit, 12);
                assert_eq!(args.format, SurfaceFormat::Json);
            }
            SurfaceCommand::Query(_) | SurfaceCommand::Explain(_) => {
                panic!("expected list command")
            }
        },
        Command::Measure(_)
        | Command::Jobs(_)
        | Command::Guard(_)
        | Command::Benchmark(_)
        | Command::Context(_)
        | Command::Report(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_) => {
            panic!("expected surface command")
        }
    }
}
