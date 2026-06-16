# 09 - QA, Benchmarking, And Calibration

> **Scope:** Current CLIARE QA coverage, benchmark corpus runner, benchmark artifacts, corpus manifests, and the boundary between operational benchmarking and future statistical calibration.
> **Status:** Current Implementation And Future Calibration Direction

---

## Summary

CLIARE is a measurement tool, so its own correctness is part of the product. The current QA and benchmark system has two different jobs:

1. **Implementation QA**
   - unit tests for parsers, scoring helpers, command specs, policies, and small domain logic
   - integration-style fixture CLI tests in `tests/fixture_clis.rs`
   - benchmark runner tests for corpus reports, optional target skips, and score-band failures

2. **Operational benchmarking**
   - run CLIARE across a manifest of real or fixture CLIs
   - produce aggregate reports
   - catch score-band movement, runtime blowups, traversal budget pressure, side effects, and output-contract drift

The current benchmark runner is not full statistical calibration. It does not compute Brier score, log loss, expected calibration error, human-labeled truth-set accuracy, or certified leaderboard authority. Those remain future calibration work.

---

## Current Test Surface

The current Rust test suite includes:

- unit tests embedded in modules such as `belief`, `claims`, `layout`, `policy`, `score_model`, `command_spec`, and CLI parsing
- integration tests in `tests/fixture_clis.rs`
- fixture CLIs generated as temporary executable scripts
- tests that exercise full measurement paths: process execution, evidence logging, shape extraction, command index generation, scorecard generation, CI artifacts, issue listings, policies, guard, and benchmark reports

Representative fixture scenarios currently covered include:

- aligned help with multi-token commands
- aliases in help output
- noisy stderr during otherwise valid help
- poor or unstructured help
- runtime false positives from help-looking rows
- precondition and fixture-needed behavior
- machine-readable output parsing
- malformed JSON output
- cache-writing help paths
- credential-like side-effect paths
- policy allow-path handling
- guard baseline comparison
- benchmark optional target skips
- benchmark expected-score band failures

The tests are fixture-driven, not a complete framework matrix. There is no current generated suite that exhaustively covers Clap, Cobra, Click, argparse, Typer, oclif, and yargs as separate framework fixtures.

---

## Benchmark Command

Current command:

```sh
cliare benchmark --manifest benchmarks/local-corpus.json --out .cliare-bench --refresh
```

Command options:

| Option | Meaning |
|---|---|
| `--manifest <FILE>` | Benchmark corpus manifest. Defaults to `benchmarks/local-corpus.json`. |
| `--out <DIR>` | Benchmark artifact output directory. |
| `--target-concurrency <N>` | Maximum number of benchmark targets measured concurrently. |
| `--refresh` | Ignore reusable measurement artifacts and run probes again. |

Each benchmark target still runs the normal bounded `measure` engine internally. Benchmark-level target concurrency controls how many target CLIs are measured at once; per-target probe concurrency is configured separately in the manifest.

---

## Benchmark Manifest Schema

Current manifest schema:

```text
cliare.benchmark-corpus.v1
```

Top-level fields:

```json
{
  "schema_version": "cliare.benchmark-corpus.v1",
  "name": "local-popular-deep-cli-corpus",
  "defaults": {},
  "targets": []
}
```

Current `defaults` fields:

| Field | Meaning |
|---|---|
| `target_concurrency` | Number of benchmark targets to measure concurrently. |
| `profile` | Default traversal profile: `quick`, `standard`, or `deep`. |
| `max_depth` | Default command-path traversal depth. |
| `max_probes` | Default per-target probe budget. |
| `min_expected_value` | Default planner expected-value cutoff. |
| `concurrency` | Default per-target probe concurrency. |
| `timeout_ms` | Default per-probe timeout. |
| `output_limit_bytes` | Default stdout/stderr byte limit per stream. |

Current target fields:

| Field | Meaning |
|---|---|
| `id` | Stable benchmark target id. Used for artifact directory naming after sanitization. |
| `target` | Executable name or path. Relative path-like values resolve relative to the manifest directory. |
| `required` | Defaults to `true`. Optional missing binaries are skipped; required failures fail the target. |
| `tags` | Free-form labels for corpus analysis. |
| `profile` | Per-target traversal profile override. |
| `max_depth` | Per-target depth override. |
| `max_probes` | Per-target probe budget override. |
| `min_expected_value` | Per-target planner cutoff override. |
| `concurrency` | Per-target probe concurrency override. |
| `timeout_ms` | Per-target timeout override. |
| `output_limit_bytes` | Per-target output limit override. |
| `expected_score` | Optional `{ "min": <0..100>, "max": <0..100> }` band. |
| `max_duration_ms` | Optional runtime cap for the target measurement. |

Validation currently checks:

- manifest schema version
- positive integer fields where supplied
- expected-score band values are finite, within `0..=100`, and `min <= max`

---

## Current Corpus Files

The repository currently includes:

| Manifest | Purpose |
|---|---|
| `benchmarks/local-corpus.json` | Local popular/deep corpus for CLIARE, rote, git, supabase, gh, cargo, npm, docker, and deno where available. |
| `benchmarks/vendor-calibration-corpus.json` | Vendor-style corpus with train/validation/holdout candidate tags for future calibration work. |
| `benchmarks/launch-low-pretraining-corpus.json` | Launch corpus focused on newer, fast-moving, or lower-pretraining CLI surfaces where command indexes are likely to help agents. |
| `benchmarks/agent-harness-corpus.json` | Corpus for agent-harness CLIs such as coding agents and tool runners. |

These files are operational corpuses. Tags such as `train-candidate`, `validation-candidate`, and `holdout-candidate` are planning labels; the current runner does not enforce statistical split discipline.

---

## Benchmark Execution Model

Current runner behavior:

1. Reads and validates a benchmark manifest.
2. Creates the output directory.
3. Acquires `.benchmark.lock` in the output directory.
4. Starts up to `target_concurrency` target measurements.
5. Runs each target with normal `measure` artifacts under:

```text
<benchmark-out>/<sanitized-target-id>/
```

6. Writes aggregate progress after startup and after each target completes.
7. Writes `benchmark.json`, `benchmark.md`, `README.md`, and `AGENT_SKILL.md`.
8. Uses atomic write-and-rename for `benchmark.json` and `benchmark.md`.
9. Releases the lock when the process exits.

Optional targets are skipped only when the target binary is not found. Other measurement errors become target failures.

Required targets fail if the binary is missing or measurement fails.

---

## Benchmark Artifacts

Benchmark directory:

```text
.cliare-bench/
  benchmark.json
  benchmark.md
  README.md
  AGENT_SKILL.md
  <target-id>/
    scorecard.json
    command-index.json
    command-index.md
    issues.json
    evidence.jsonl
    ...
```

`benchmark.json` schema:

```text
cliare.benchmark-report.v1
```

Top-level fields:

- `schema_version`
- `corpus`
- `manifest_path`
- `artifact_dir`
- `duration_ms`
- `target_concurrency`
- `totals`
- `calibration`
- `targets`

The `calibration` field name is historical in the current report. It contains aggregate benchmark telemetry, not full statistical calibration.

---

## Benchmark Totals

Current totals:

| Field | Meaning |
|---|---|
| `targets` | Number of manifest targets. |
| `measured` | Targets with status `passed` or `failed`. |
| `skipped` | Optional targets skipped because the binary was missing. |
| `pending` | Targets not yet completed in an in-progress report. |
| `failed` | Targets whose benchmark-level checks failed. |
| `passed` | `true` when there are no failed or pending targets. |
| `complete` | `true` when there are no pending targets. |

Target statuses:

- `passed`
- `failed`
- `skipped`
- `pending`

---

## Benchmark Aggregate Metrics

The current `calibration` object includes:

| Field | Meaning |
|---|---|
| `measured_targets` | Number of targets with scores. |
| `score_mean` | Mean measured score. |
| `score_min` | Minimum measured score. |
| `score_max` | Maximum measured score. |
| `expected_band_targets` | Measured targets with expected-score bands. |
| `expected_band_passed` | Targets whose score was within the expected band. |
| `expected_band_pass_rate` | Expected-band pass ratio. |
| `traversal_complete_targets` | Targets whose traversal completed. |
| `traversal_completion_rate` | Traversal completion ratio. |
| `budget_exhausted_targets` | Targets that exhausted probe budget. |
| `budget_exhaustion_rate` | Budget exhaustion ratio. |
| `probes_completed_total` | Sum of completed probes. |
| `findings_total` | Sum of scorecard findings. |
| `commands_precondition_blocked` | Sum of precondition-blocked command counts. |
| `precondition_blocked_probes` | Sum of precondition-blocked probes. |
| `auth_required_probes` | Sum of auth-required probes. |
| `local_context_required_probes` | Sum of local-context-required probes. |
| `fixture_required_probes` | Sum of fixture-required probes. |
| `output_contracts_discovered` | Sum of discovered output contracts. |
| `machine_readable_output_contracts` | Sum of JSON/YAML output contracts. |
| `output_mode_parse_successes` | Sum of parse successes. |
| `output_mode_precondition_blocked` | Sum of output probes blocked by preconditions. |
| `side_effect_files_total` | Sum of observed filesystem side-effect changes. |
| `side_effect_probe_count` | Sum of probes with side effects. |
| `credential_like_side_effects` | Sum of credential-like side-effect paths. |

These metrics are useful for release QA and score movement analysis. They are not Brier score, log loss, expected calibration error, or truth-set accuracy.

---

## Target Reports

Each target entry records:

- id, target, resolved target, required flag, and tags
- status
- score
- expected-score band
- duration and max-duration cap
- probes completed
- finding count
- traversal completion and budget exhaustion state
- observed depth and max depth
- max probes and concurrency limit
- precondition counts
- output-contract counts
- side-effect counts
- target artifact directory
- benchmark-level issues

Benchmark-level issues are limited to:

- score outside expected band
- duration exceeding `max_duration_ms`
- target missing for required targets
- measurement failure

CLI product findings still live in the target artifact directory, especially `scorecard.json`, `issues.json`, and `report.md`.

---

## Expected Score Bands

Expected score bands are intentionally broad. They are not truth labels and should not be treated as exact quality assertions.

They are useful for:

- catching accidental scoring-model regressions
- detecting severe extractor regressions
- catching runtime blowups that collapse a known target score
- preventing benchmark drift from going unnoticed

They are not sufficient for:

- calibrated public ranking
- probability calibration
- command-level truth validation
- safety false-safe measurement

---

## Current QA Matrix

| Area | Current Coverage |
|---|---|
| CLI parsing and command spec | Unit tests. |
| Belief/log-odds behavior | Unit tests. |
| Score-model loading and hash coverage | Unit tests. |
| Layout and usage extraction | Unit tests plus fixture CLIs. |
| Claim inference | Unit tests plus fixture CLIs. |
| Output classification | Unit tests plus fixture CLIs. |
| Precondition diagnostics | Unit tests and fixture CLIs. |
| Sandbox side-effect capture | Unit tests and fixture CLIs. |
| Policy evaluation | Unit tests and fixture CLIs. |
| Guard baseline comparison | Fixture CLI tests. |
| CI artifacts | Fixture CLI tests. |
| Issues list and dispositions | Unit and fixture tests. |
| Benchmark report generation | Fixture CLI tests. |
| Real CLI corpus execution | `cliare benchmark` manifests. |

---

## Future Ground Truth Layer

A future calibration layer should add human-reviewed truth sets. Those truth sets should use the same vocabulary as `shape.json` and `command-index.json`, but with explicit labels.

Candidate truth categories:

- command existence
- command parent/child relationships
- flag existence
- flag arity
- positional requirements
- value domains
- output contract availability
- output parseability
- precondition class
- side-effect class
- safe-probe side-effect expectations
- invalid-input recovery quality

Negative facts are essential. For example:

```json
{
  "command": ["project", "delete"],
  "dry_run_supported": false
}
```

Without negative labels, calibration can reward overconfident false positives.

There is no current `cliare.ground-truth.v1` reader or truth-set comparison command in the implementation.

---

## Future Calibration Metrics

Future statistical calibration should compute metrics such as:

For binary claims:

- accuracy
- precision
- recall
- Brier score
- log loss
- expected calibration error

For categorical claims:

- top-1 accuracy
- top-3 accuracy
- categorical log loss
- expected calibration error

For command discovery:

- observed recall against truth
- false discovered command rate
- false confirmed command rate
- depth-weighted recall

For grammar:

- flag existence F1
- arity accuracy
- positional requiredness accuracy
- enum extraction accuracy

For safety:

- false-safe rate
- write/destructive recall
- side-effect class accuracy
- dry-run detection accuracy

The bundled score model already declares the intended calibration metrics, but the current `benchmark` command does not compute them.

---

## Future Stability And Monotonicity QA

Future stability tests should run the same deterministic target repeatedly and measure:

- total score standard deviation
- subscore standard deviation
- command confidence drift
- nondeterministic findings
- guard pass/fail flapping

Future monotonicity tests should use paired fixtures where one known improvement is introduced:

- add root help
- add command-specific help
- add JSON/YAML output
- add better unknown-flag diagnostics
- remove safe-probe side effects
- clarify flag arity

Expected behavior:

- relevant subscore improves
- unrelated subscores do not move materially
- newly revealed risk is reported as revealed risk rather than a monotonicity bug

Some paired fixture behavior exists today, but there is no dedicated stability/monotonicity benchmark harness with repeated-run statistics.

---

## Future Schema QA

Future schema QA should include formal JSON Schema validation and compatibility testing:

- every artifact validates against a published schema
- old artifacts remain readable where compatibility is promised
- unsupported schema versions fail clearly
- evidence references resolve
- command-index and shape references point back to evidence
- additive fields are accepted where intended
- breaking changes require a new schema version

Current implementation uses typed Rust serialization/deserialization and schema-version constants for key artifacts. It does not ship a full published JSON Schema validation suite.

---

## Future Leaderboard QA

Leaderboard ingestion is not implemented. Future hosted ingestion tests should cover:

- valid scorecard accepted
- invalid schema rejected
- unsupported score model rejected or quarantined
- duplicate release handled idempotently
- provenance verified
- artifact hash mismatch rejected
- verification level assigned correctly
- stale model displayed separately

Public leaderboard QA belongs after score-model calibration and reproducible profiles exist.

---

## Calibration Workflow Direction

The current workflow:

```text
run benchmark corpus -> inspect aggregate telemetry -> drill into target artifacts -> adjust implementation or model intentionally
```

Future statistical calibration workflow:

1. Build labeled truth sets.
2. Split CLI families into train, validation, and holdout groups before tuning.
3. Run CLIARE under fixed profiles.
4. Extract predicted claims and score outputs.
5. Compare predictions against truth.
6. Compute proper scoring metrics.
7. Tune priors, evidence weights, thresholds, and coefficients on train data.
8. Select candidate model revisions on validation data.
9. Report final metrics once on holdout.
10. Freeze the model artifact and publish its hash.

This is the path from `cliare-score-v0` as an operational CI model toward a future calibrated score model. It is not completed by the current benchmark runner.

For the public-authority bar, see [Calibration And Leaderboard Authority](calibration-and-leaderboard-authority.md).

---

## Current Release Criteria

For the current OSS release line, benchmark readiness means:

- `cargo test --workspace --all-features` passes
- fixture CLI coverage continues to exercise measurement, scoring, reporting, issue, guard, policy, and benchmark paths
- `cliare benchmark` can run the checked-in manifests without report-writer races
- optional missing targets are skipped rather than failing the corpus
- required missing targets fail clearly
- expected-score bands catch large unintended score movement
- runtime caps catch target blowups
- generated benchmark artifacts are navigable by humans and agents
- docs do not claim public leaderboard authority for `cliare-score-v0`

For future v1 standard claims, release criteria must additionally include labeled truth sets, proper scoring metrics, repeated-run stability, published model governance, and reproducible certified profiles.
