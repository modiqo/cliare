use super::support::*;

#[test]
fn metadata_exposes_parseable_output_mode() {
    let mut command = Cli::command();
    let help = command.render_long_help().to_string();

    assert!(help.contains("metadata"));

    let cli = Cli::try_parse_from(["cliare", "metadata", "--format", "json", "--help"])
        .expect("valid metadata command");

    match cli.command {
        Command::Metadata(args) => {
            assert_eq!(args.format, MetadataFormat::Json);
            assert!(args.help);
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
        | Command::Surface(_) => {
            panic!("expected metadata command")
        }
        Command::Playbook(_) => {
            panic!("expected metadata command")
        }
    }
}
