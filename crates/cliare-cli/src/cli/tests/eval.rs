use super::support::*;

#[test]
fn eval_shape_quality_accepts_shape_truth_and_output_paths() {
    let cli = Cli::parse_from([
        "cliare",
        "eval",
        "shape-quality",
        "--shape",
        ".cliare/shape.json",
        "--truth",
        "benchmarks/truth/fixture.shape-truth.json",
        "--out",
        ".cliare-eval/fixture",
    ]);

    let Command::Eval(args) = cli.command else {
        panic!("expected eval command");
    };
    let EvalCommand::ShapeQuality(args) = args.command;
    assert_eq!(args.shape, std::path::PathBuf::from(".cliare/shape.json"));
    assert_eq!(
        args.truth,
        std::path::PathBuf::from("benchmarks/truth/fixture.shape-truth.json")
    );
    assert_eq!(args.out, std::path::PathBuf::from(".cliare-eval/fixture"));
}
