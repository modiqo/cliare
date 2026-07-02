use super::support::*;

#[test]
fn measure_accepts_configurable_snapshot_limits() {
    let cli = Cli::try_parse_from([
        "cliare",
        "measure",
        "target",
        "--snapshot-max-files",
        "12",
        "--snapshot-max-directories",
        "13",
        "--snapshot-max-hash-bytes",
        "14",
    ])
    .expect("valid measure command");

    match cli.command {
        Command::Measure(args) => {
            assert_eq!(args.snapshot_limits(), SnapshotLimits::new(12, 13, 14));
        }
        Command::Guard(_)
        | Command::Jobs(_)
        | Command::Benchmark(_)
        | Command::Eval(_)
        | Command::Context(_)
        | Command::Report(_)
        | Command::Summary(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_)
        | Command::Surface(_) => {
            panic!("expected measure command")
        }
    }
}

#[test]
fn measure_and_guard_share_deep_recursion_defaults() {
    let measure = Cli::try_parse_from(["cliare", "measure", "target"]).expect("valid measure");
    let guard = Cli::try_parse_from(["cliare", "guard", "target", "--baseline", "scorecard.json"])
        .expect("valid guard");

    match measure.command {
        Command::Measure(args) => {
            assert_eq!(args.profile, TraversalProfile::Standard);
            assert_eq!(args.resolved_max_depth(), STANDARD_MAX_DEPTH);
            assert_eq!(args.resolved_max_probes(), STANDARD_MAX_PROBES);
            assert_eq!(
                args.resolved_min_expected_value(),
                STANDARD_MIN_EXPECTED_VALUE
            );
            assert_eq!(args.resolved_concurrency(), STANDARD_CONCURRENCY);
        }
        Command::Guard(_)
        | Command::Jobs(_)
        | Command::Benchmark(_)
        | Command::Eval(_)
        | Command::Context(_)
        | Command::Report(_)
        | Command::Summary(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_)
        | Command::Surface(_) => {
            panic!("expected measure command")
        }
    }

    match guard.command {
        Command::Guard(args) => {
            let measure_args = MeasureArgs::from(&args);
            assert_eq!(args.profile, TraversalProfile::Standard);
            assert_eq!(measure_args.resolved_max_depth(), STANDARD_MAX_DEPTH);
            assert_eq!(measure_args.resolved_max_probes(), STANDARD_MAX_PROBES);
            assert_eq!(
                measure_args.resolved_min_expected_value(),
                STANDARD_MIN_EXPECTED_VALUE
            );
            assert_eq!(measure_args.resolved_concurrency(), STANDARD_CONCURRENCY);
        }
        Command::Measure(_)
        | Command::Jobs(_)
        | Command::Benchmark(_)
        | Command::Eval(_)
        | Command::Report(_)
        | Command::Summary(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Context(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_)
        | Command::Surface(_) => {
            panic!("expected guard command")
        }
    }
}

#[test]
fn measure_accepts_detached_job_mode() {
    let cli = Cli::try_parse_from([
        "cliare",
        "measure",
        "target",
        "--out",
        ".cliare-target",
        "--detach",
    ])
    .expect("valid detached measure command");

    match cli.command {
        Command::Measure(args) => {
            assert_eq!(args.out, std::path::PathBuf::from(".cliare-target"));
            assert!(args.detach);
            assert!(!args.detached_worker);
            assert_eq!(args.job_id, None);
        }
        Command::Jobs(_)
        | Command::Guard(_)
        | Command::Benchmark(_)
        | Command::Eval(_)
        | Command::Context(_)
        | Command::Report(_)
        | Command::Summary(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_)
        | Command::Surface(_) => {
            panic!("expected measure command")
        }
    }
}

#[test]
fn measure_accepts_host_execution_mode() {
    let cli = Cli::try_parse_from(["cliare", "measure", "target", "--execution-mode", "host"])
        .expect("valid host execution mode");

    match cli.command {
        Command::Measure(args) => {
            assert_eq!(args.execution_mode, SandboxProfile::Host);
        }
        Command::Jobs(_)
        | Command::Guard(_)
        | Command::Benchmark(_)
        | Command::Eval(_)
        | Command::Context(_)
        | Command::Report(_)
        | Command::Summary(_)
        | Command::Describe(_)
        | Command::Skills(_)
        | Command::Issues(_)
        | Command::Playbook(_)
        | Command::Metadata(_)
        | Command::Surface(_) => {
            panic!("expected measure command")
        }
    }
}
