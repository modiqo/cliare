# CLIARE

CLIARE evaluates command-line interfaces for use by agents, harnesses, CI systems, and automation.

It treats a CLI as a black-box runtime system: the installed executable is fingerprinted, exercised under bounded probes, and reduced to evidence logs, command-shape artifacts, scorecards, and CI reports. The result is a reproducible measurement of discovery quality, invocation grammar, execution behavior, machine-readable output, safety signals, recovery behavior, and drift across releases.

CLIARE stands for **CLI Agent Readiness Evaluation**.

## Mission

The future of agent harnesses is increasingly CLI-native. Agents already lean on tools like `git`, `gh`, `docker`, `kubectl`, `supabase`, cloud CLIs, internal platform CLIs, and product-specific command surfaces because CLIs are easy to install, script, version, permission, and run in CI. At the same time, CLIs are evolving faster as new models learn to operate tools, vendors add agent-oriented workflows, and teams ship more automation-first surfaces.

CLIARE exists to make those command surfaces measurable, navigable, and reliable. The intended deployment point is the same CI pipeline that builds and releases the CLI.

The project is built on a few commitments:

- No source-code requirement: measure the executable users actually install.
- No framework assumption: work across clap, cobra, argparse, hand-rolled parsers, shell wrappers, and poorly documented CLIs.
- No hosted dependency: run locally in the maintainer's CI environment by default.
- Evidence first: every score should be traceable to runtime observations.
- Improvement oriented: scores should move when maintainers improve discoverability, grammar, outputs, safety, recovery, and stability.
- Agent-operable artifacts: emitted shape catalogs and scorecards should help agents navigate CLIs without rediscovering the same surface through blind trial and error.

## What CLIARE Unlocks

CLIARE turns a CLI from an opaque executable into a measured, versioned, evidence-backed interface.

For maintainers, it provides a regression gate and improvement loop:

- Detect command, flag, help, output, exit-code, auth, and safety drift between releases.
- Track score improvements as the CLI adds clearer help, safer probes, better JSON output, dry-run support, noninteractive modes, and predictable errors.
- Publish scorecards, CI summaries, badges, and release artifacts that show whether a CLI is becoming more usable by agents and automation.
- Catch issues before release without uploading binaries or private command output to a cloud service.

For agent harnesses, it provides a navigation index:

- Command trees, aliases, flags, positionals, output contracts, and runtime states.
- Safe probe evidence and destructive-risk signals.
- Auth-gated and precondition-blocked paths that should not be mistaken for missing commands.
- Confidence scores that help an agent choose high-quality paths before trying uncertain ones.
- Portable shape artifacts that can be loaded by planners, tool routers, adapter builders, and benchmark runners.

For model training and evaluation, it can create a structured corpus of real CLI behavior:

- Runtime-derived command catalogs rather than hand-written benchmark assumptions.
- Evidence-linked examples of discovery, recovery, output parsing, and precondition handling.
- Release-to-release drift records that teach agents how tool surfaces change over time.
- Long-tail CLI coverage beyond the handful of popular CLIs already overrepresented in pretraining.

The goal is not only to score CLIs. It is to raise the quality bar for CLI design, give maintainers a concrete improvement loop, and give agents a reliable map for operating unfamiliar command surfaces.

## Status

This repository is private while the project is being shaped. The initial commit contains the Rust project scaffold and the full design packet under [`docs/`](docs/00-index.md).

CLIARE measures itself in GitHub Actions. Pull requests measure the freshly built `cliare` binary with the `quick` profile, pushes to `main` use `standard`, scheduled weekly runs use `deep`, and each run publishes a job-summary score plus uploaded evidence, shape, scorecard, and report artifacts.

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
cliare metadata --format json
cliare certify ./mycli
cliare rescore .cliare/evidence.jsonl
```

The implemented `measure` command fingerprints a target binary, runs bounded safe probes inside isolated per-probe HOME/PWD/XDG/TMP sandboxes with a sanitized environment, records `evidence.jsonl`, emits a generic `shape.json`, and writes `scorecard.json`, `report.md`, `summary.md`, `findings.sarif`, and `junit.xml` artifacts over currently measured dimensions. The shape artifact includes aliases, usage-derived positionals, flag grammar such as boolean, required-value, optional-value, repeatable, and required flags, plus output contracts for advertised JSON/YAML/table/plain modes where help output exposes them. Command extraction is structural rather than framework-specific: it uses indentation, aligned rows, compact invocation cells, token morphology, block density, runtime confirmation, and manpage detection instead of hard-coded section titles such as `Commands` or `Subcommands`. CLIARE distinguishes command absence from precondition-blocked runtime evidence: auth/login diagnostics are represented as `runtime_state: precondition_blocked` with `auth_required`, not as ordinary command failures. Output-contract inference excludes file-path defaults such as `report.json` unless the flag is actually advertised as a format or machine-output selector. Every probe is wrapped in sandbox filesystem snapshots so persistent created, modified, and deleted files are recorded as safety evidence. `measure-cache.json` allows later runs to reuse artifacts when the target fingerprint, traversal profile, sandbox profile, resolved probe budget, expected-value threshold, concurrency limit, CLIARE version, measurement engine, and artifact set match. Use `--refresh` to force a new probe run.

The implemented `guard` command measures a target, rewrites CI artifacts with guard context, fails on total-score regression against a baseline scorecard, and can evaluate `cliare.policy.v1` JSON policies through `--policy`. Policies support `min_total_score`, per-dimension `min_subscores`, side-effect `allow_paths`, `max_unapproved`, and `deny_credential_like`. Traversal profiles provide useful presets: `quick` is depth 3 / 64 probes / concurrency 2, `standard` is depth 5 / 256 probes / concurrency 4, and `deep` is depth 8 / 1000 probes / concurrency 8. `--max-depth`, `--max-probes`, `--min-expected-value`, and `--concurrency` override the selected profile for larger, tighter, or more aggressive CI runs.

Scorecards report coverage pressure, output coverage, precondition-blocked probes, side-effect coverage, scheduler accounting, and runtime isolation metadata, including profile, observed depth, frontier remaining, expected-value convergence skips, candidates skipped by depth, stop reason, probes skipped by budget, probes scheduled, scheduler rounds, output parse successes, sandbox file changes, sandbox root, and env policy. The implemented `benchmark` command runs a manifest-defined real CLI corpus with target-level parallelism, per-target measurement artifacts, expected score bands, runtime caps, precondition-blocked counts, and streaming `benchmark.json`/`benchmark.md` reports. Benchmark aggregation is single-writer with atomic file replacement and an output-directory lock, so parallel target execution does not corrupt the aggregate report. `metadata --format json` emits a parseable CLIARE implementation contract. The root `action.yml` composite action runs `measure` or `guard` in the caller's CI environment, uploads only CLIARE artifacts, appends the Markdown summary to the job summary, and exposes score/output paths. Other commands remain planned.

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
- [Scoring model and Bayesian confidence](docs/17-scoring-model-and-bayesian-confidence.md)
- [Calibration and leaderboard authority](docs/18-calibration-and-leaderboard-authority.md)
- [Rust runtime engineering](docs/13-rust-runtime-engineering.md)
- [Operational contracts](docs/14-operational-contracts.md)

## License

Apache-2.0. See [LICENSE](LICENSE).
