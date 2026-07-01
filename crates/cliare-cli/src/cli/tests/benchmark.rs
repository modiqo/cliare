use super::support::*;

#[test]
fn benchmark_uses_local_corpus_defaults() {
    let cli = Cli::try_parse_from(["cliare", "benchmark"]).expect("valid benchmark");

    match cli.command {
        Command::Benchmark(args) => {
            assert_eq!(
                args.manifest,
                std::path::PathBuf::from("benchmarks/local-corpus.json")
            );
            assert_eq!(args.out, std::path::PathBuf::from(".cliare-bench"));
            assert_eq!(args.target_concurrency, None);
            assert!(!args.refresh);
        }
        Command::Measure(_)
        | Command::Jobs(_)
        | Command::Guard(_)
        | Command::Eval(_)
        | Command::Context(_)
        | Command::Report(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_)
        | Command::Surface(_) => {
            panic!("expected benchmark command")
        }
    }
}
