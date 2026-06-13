# CLIARE

CLIARE is an open-source standard and Rust reference implementation for measuring how ready a command-line interface is for agents and automation.

It treats a CLI as a black-box runtime system, exercises it in a controlled sandbox, records evidence, infers command shape, and produces a reproducible agent-readiness scorecard for CI, badges, and long-term improvement tracking.

CLIARE stands for **CLI for Agent Readiness**.

## Status

This repository is private while the project is being shaped. The initial commit contains the Rust project scaffold and the full design packet under [`docs/`](docs/00-index.md).

## Goals

- Infer command trees, flags, arguments, output contracts, and safety properties from runtime evidence.
- Score CLI readiness across discovery, grammar, execution, output, safety, and recovery.
- Run locally in CI without uploading binaries to a hosted service.
- Emit portable artifacts: evidence logs, command-shape catalogs, scorecards, reports, SARIF, JUnit XML, and CI summaries.
- Provide a public standard that CLI maintainers can use to improve agent operability.

## CLI

```sh
cliare measure ./mycli
cliare guard ./mycli --baseline .cliare/baseline.scorecard.json
cliare guard ./mycli --baseline .cliare/baseline.scorecard.json --policy cliare.policy.json
cliare benchmark --manifest benchmarks/local-corpus.json --out .cliare-bench
cliare certify ./mycli
cliare rescore .cliare/evidence.jsonl
```

The implemented `measure` command fingerprints a target binary, runs bounded safe probes inside isolated per-probe HOME/PWD/XDG/TMP sandboxes with a sanitized environment, records `evidence.jsonl`, emits a generic `shape.json`, and writes experimental `scorecard.json`, `report.md`, `summary.md`, `findings.sarif`, and `junit.xml` artifacts over currently measured dimensions. The shape artifact now includes aliases, usage-derived positionals, flag grammar such as boolean, required-value, optional-value, repeatable, and required flags, plus output contracts for advertised JSON/YAML/table/plain modes where help output exposes them. Command extraction is structural rather than framework-specific: it uses indentation, aligned rows, compact invocation cells, token morphology, block density, runtime confirmation, and manpage detection instead of hard-coded section titles such as `Commands` or `Subcommands`. CLIARE probes documented output flags only through safe help probes such as `--json --help` or `--format json --help`, then records parse success or failure in the shape and scorecard. Every probe is also wrapped in sandbox filesystem snapshots so persistent created, modified, and deleted files are recorded as safety evidence. It also writes `measure-cache.json`; later runs reuse artifacts when the target fingerprint, traversal profile, sandbox profile, resolved probe budget, expected-value threshold, concurrency limit, CLIARE version, measurement engine, and artifact set match. Use `--refresh` to force a new probe run. The implemented `guard` command measures a target, rewrites CI artifacts with guard context, fails on total-score regression against a baseline scorecard, and can also evaluate `cliare.policy.v1` JSON policies through `--policy`. Policies support `min_total_score`, per-dimension `min_subscores`, side-effect `allow_paths`, `max_unapproved`, and `deny_credential_like`. Traversal profiles provide useful presets: `quick` is depth 3 / 64 probes / concurrency 2, `standard` is depth 5 / 256 probes / concurrency 4, and `deep` is depth 8 / 1000 probes / concurrency 8. `--max-depth`, `--max-probes`, `--min-expected-value`, and `--concurrency` override the selected profile for larger, tighter, or more aggressive CI runs. Scorecards also report coverage pressure, output coverage, side-effect coverage, scheduler accounting, and runtime isolation metadata, including profile, observed depth, frontier remaining, expected-value convergence skips, candidates skipped by depth, stop reason, probes skipped by budget, probes scheduled, scheduler rounds, output parse successes, sandbox file changes, sandbox root, and env policy. The implemented `benchmark` command runs a manifest-defined real CLI corpus with target-level parallelism, per-target measurement artifacts, expected score bands, runtime caps, and streaming `benchmark.json`/`benchmark.md` reports. Benchmark aggregation is single-writer with atomic file replacement and an output-directory lock, so parallel target execution does not corrupt the aggregate report. The root `action.yml` composite action runs `measure` or `guard` in the caller's CI environment, uploads only CLIARE artifacts, appends the Markdown summary to the job summary, and exposes score/output paths. Other commands remain planned.

Example policy:

```json
{
  "schema_version": "cliare.policy.v1",
  "min_total_score": 80.0,
  "min_subscores": {
    "output": 50.0,
    "safety": 90.0
  },
  "side_effects": {
    "allow_paths": ["xdg-cache/fixture-cli/**"],
    "max_unapproved": 0,
    "deny_credential_like": true
  }
}
```

## Design Packet

Start here:

- [Design index](docs/00-index.md)
- [Mathematical model](docs/06-mathematical-model.md)
- [Rust runtime engineering](docs/13-rust-runtime-engineering.md)
- [Operational contracts](docs/14-operational-contracts.md)

## License

Apache-2.0. See [LICENSE](LICENSE).
