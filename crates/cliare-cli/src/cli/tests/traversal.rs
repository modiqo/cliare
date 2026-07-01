use super::support::*;

#[test]
fn traversal_profiles_resolve_budget_presets_and_overrides() {
    let quick = Cli::try_parse_from(["cliare", "measure", "target", "--profile", "quick"])
        .expect("valid quick profile");
    let deep = Cli::try_parse_from(["cliare", "measure", "target", "--profile", "deep"])
        .expect("valid deep profile");
    let override_depth = Cli::try_parse_from([
        "cliare",
        "measure",
        "target",
        "--profile",
        "quick",
        "--max-depth",
        "7",
        "--max-probes",
        "128",
        "--min-expected-value",
        "90",
        "--concurrency",
        "11",
    ])
    .expect("valid overrides");

    assert_budget(
        quick,
        TraversalProfile::Quick,
        QUICK_MAX_DEPTH,
        QUICK_MAX_PROBES,
        QUICK_MIN_EXPECTED_VALUE,
        QUICK_CONCURRENCY,
    );
    assert_budget(
        deep,
        TraversalProfile::Deep,
        DEEP_MAX_DEPTH,
        DEEP_MAX_PROBES,
        DEEP_MIN_EXPECTED_VALUE,
        DEEP_CONCURRENCY,
    );
    assert_budget(override_depth, TraversalProfile::Quick, 7, 128, 90, 11);
}

fn assert_budget(
    cli: Cli,
    profile: TraversalProfile,
    max_depth: usize,
    max_probes: usize,
    min_expected_value: u16,
    concurrency: usize,
) {
    match cli.command {
        Command::Measure(args) => {
            assert_eq!(args.profile, profile);
            assert_eq!(args.resolved_max_depth(), max_depth);
            assert_eq!(args.resolved_max_probes(), max_probes);
            assert_eq!(args.resolved_min_expected_value(), min_expected_value);
            assert_eq!(args.resolved_concurrency(), concurrency);
        }
        Command::Guard(_)
        | Command::Jobs(_)
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
            panic!("expected measure command")
        }
    }
}
