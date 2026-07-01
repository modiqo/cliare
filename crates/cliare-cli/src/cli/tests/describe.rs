use super::support::*;

#[test]
fn describe_accepts_folder_format_and_write_options() {
    let cli = Cli::try_parse_from([
        "cliare",
        "describe",
        ".cliare-current",
        "--context",
        "clean",
        "--format",
        "json",
        "--write",
    ])
    .expect("valid describe command");

    match cli.command {
        Command::Describe(args) => {
            assert_eq!(args.folder, std::path::PathBuf::from(".cliare-current"));
            assert_eq!(args.context.as_deref(), Some("clean"));
            assert_eq!(args.format, DescribeFormat::Json);
            assert!(args.write);
        }
        Command::Measure(_)
        | Command::Jobs(_)
        | Command::Guard(_)
        | Command::Benchmark(_)
        | Command::Eval(_)
        | Command::Context(_)
        | Command::Report(_)
        | Command::Skills(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_)
        | Command::Surface(_) => {
            panic!("expected describe command")
        }
    }
}
