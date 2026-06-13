# 11 - Implementation Plan

> **Scope:** Phased implementation, repository layout, milestones, acceptance criteria, and public launch path.
> **Status:** Draft

---

## Overview

Implementation is divided into phases. Each phase has a checkpoint and acceptance criteria.

```
Phase 0: Project scaffold and schemas
Phase 1: Local probing and evidence log
Phase 2: Basic inference and shape catalog
Phase 3: Scorecard and report
Phase 4: CI guard and GitHub Action
Phase 5: Benchmark and calibration
Phase 6: Public scorecard publishing
Phase 7: Leaderboard and badge
Phase 8: v1 standard
```

---

## Repository Layout

Suggested repo:

```
modiqo/cliare
```

Suggested layout:

```
cliare/
  README.md
  LICENSE
  docs/
    spec/
      evidence.md
      command-shape.md
      scorecard.md
      scoring-model.md
      safety-model.md
    design/
      ...
  schemas/
    cliare.evidence.v1.schema.json
    cliare.command-shape.v1.schema.json
    cliare.scorecard.v1.schema.json
  crates/
    cliare-cli/
    cliare-core/
    cliare-sandbox/
    cliare-probe/
    cliare-infer/
    cliare-score/
    cliare-report/
    cliare-bench/
  fixtures/
    synthetic/
    framework/
  action/
    action.yml
  examples/
    simple-cli/
    github-action/
```

Language choice:

- Rust is a strong fit for local process execution, sandboxing, speed, and portable binaries.
- TypeScript can be used for GitHub Action wrapper and web leaderboard.

---

## Phase 0: Project Scaffold and Schemas

### Goal

Create the OSS project foundation and artifact schemas.

### Tasks

1. Create repo scaffold.
2. Add Apache-2.0 or dual Apache-2.0/MIT license.
3. Add top-level README.
4. Define evidence schema v1.
5. Define command-shape schema v1.
6. Define scorecard schema v1.
7. Add schema validation tests.
8. Add fixture artifact examples.

### Checkpoint 0 Pass Criteria

- `cargo test` passes.
- Example evidence validates.
- Example shape validates.
- Example scorecard validates.
- README explains local-first value prop.

---

## Phase 1: Local Probing and Evidence Log

### Goal

Run basic safe probes against a binary and record evidence.

### Tasks

1. Implement target fingerprinting.
2. Implement portable sandbox:
   - temp HOME
   - temp cwd
   - XDG dirs
   - env policy
   - timeout
   - output limit
   - cleanup
3. Implement bootstrap probes:
   - `--help`
   - `-h`
   - `help`
   - `--version`
   - `version`
   - unknown command
   - unknown flag
4. Implement evidence writer.
5. Add redaction hooks.
6. Add `cliare measure <binary> --profile safe`.

### Checkpoint 1 Pass Criteria

- Can run against a simple binary.
- Emits valid `evidence.jsonl`.
- Records exit codes, stdout, stderr, duration.
- Applies temp HOME and env isolation.
- Times out a hanging fixture.
- Cleans sandbox by default.

---

## Phase 2: Basic Inference and Shape Catalog

### Goal

Infer candidate commands and flags from safe evidence without assuming a specific CLI framework.

### Tasks

1. Implement a help document layout pass:
   - line tokenization
   - indentation and section grouping
   - aligned-row detection
   - continuation-line detection
   - command-like and flag-like token features
2. Implement a generic claim store:
   - candidate command claims
   - candidate flag claims
   - candidate arity/domain claims
   - evidence references
   - confidence values
3. Implement lightweight Bayesian updates:
   - weighted log-odds for binary claims
   - categorical beliefs for line class and output kind
4. Implement confirmation probe planning:
   - `<candidate> --help`
   - `help <candidate>`
   - invalid child and invalid flag probes
5. Implement basic flag inference:
   - existence
   - long/short
   - arity
   - required
6. Implement basic positional inference.
7. Implement output kind classifier.
8. Implement contradiction detector.
9. Emit `shape.json`.

### Checkpoint 2 Pass Criteria

- Synthetic CLI command tree is inferred through the generic claim pipeline.
- CLIARE can measure its own Clap-based CLI without a Clap-specific parser.
- Flags are inferred with confidence.
- Help/runtime contradictions are recorded.
- Shape validates against schema.
- Every shape claim references evidence.

### Current Checkpoint

The current implementation has local probing, evidence, and the first generic claim pipeline in place:

- `fingerprint` resolves explicit and PATH targets to absolute binaries and hashes them.
- `sandbox` creates an isolated runtime root under the artifact directory with deterministic HOME, PWD, XDG config/cache/data, temp dirs, and a cleared allowlisted environment.
- `process` executes probes with bounded stdout/stderr, timeouts, null stdin, and sandbox cwd/env.
- `process` snapshots sandbox HOME/cwd/XDG/TMP before and after each probe and attaches persistent file changes to process evidence.
- `evidence` records run-level sandbox metadata and per-probe cwd/env policy.
- `claims` converts observations into command and flag beliefs.
- `claims` also records advertised output contracts and parser results for safe output-mode probes.
- `planner` ranks confirmation, diagnostic, and output-mode probes deterministically.
- `shape` emits the catalog from claims rather than from a framework parser.
- Invalid-child probes are gated on evidence of nested commands so leaf commands with positionals are not misclassified as command trees.
- Output-mode probes combine the documented output flag with `--help` so CLIARE can validate machine-readable behavior without running a command body.
- Fixture CLI integration tests cover custom help, aliases, noisy help, runtime false-positive rejection, cache reuse, score guards, sandbox HOME/PWD isolation, parseable JSON output, malformed JSON output, clean probes, cache writes, and credential-like writes.

---

## Phase 3: Scorecard and Report

### Goal

Compute initial CLIARE score and produce human report.

### Tasks

1. Implement subscore calculations:
   - discovery
   - grammar
   - execution
   - output
   - safety
   - recovery
2. Implement simple posterior model.
3. Implement confidence intervals.
4. Implement finding generation.
5. Emit `scorecard.json`.
6. Emit `report.md`.
7. Add terminal summary.

### Checkpoint 3 Pass Criteria

- Synthetic good CLI scores higher than synthetic poor CLI.
- Adding JSON output improves output score.
- Adding dry-run improves safety score.
- Removing completion lowers discovery score.
- Report lists top findings with recommendations.

### Current Checkpoint

The current implementation emits experimental partial `scorecard.json`, `report.md`, `summary.md`, `findings.sarif`, and `junit.xml` artifacts from `measure`.

- Discovery, grammar, execution, recovery, output, and initial safety are scored from current evidence.
- Output scoring credits advertised JSON/YAML contracts and safe parse-probe success; CLIs with no machine-readable mode now receive output findings.
- Safety scoring uses persistent sandbox file changes from safe probes and reports created, modified, deleted, and credential-like paths.
- Fixture tests verify that a clearer CLI scores higher than a poor CLI.
- Fixture tests verify that parseable JSON output improves the output subscore and malformed JSON records a parse gap.
- Fixture tests verify that clean probes receive full safety credit and help-time writes lower the safety score.
- The Markdown report explains the score, measured coverage, output coverage, side-effect coverage, and findings.
- The CI summary renders a compact PR-oriented Markdown view of score, subscores, findings, output coverage, safety evidence, and artifact paths.
- SARIF maps CLIARE findings to code-scanning levels: high as error, medium as warning, and low as note.
- JUnit XML exposes each finding as a failure-style test case for CI systems that consume test reports.
- The CLI prints a terminal summary with score, probe count, finding count, and artifact paths.

---

## Phase 4: CI Guard and GitHub Action

### Goal

Make CLIARE useful in PRs and release pipelines.

### Tasks

1. Implement baseline accept:
   ```sh
   cliare baseline accept .cliare/scorecard.json
   ```
2. Implement guard mode:
   ```sh
   cliare guard ./mycli --baseline .cliare/baseline.scorecard.json
   ```
3. Implement policy file.
4. Implement SARIF output.
5. Implement JUnit output.
6. Create GitHub Action wrapper.
7. Add PR summary output.

### Checkpoint 4 Pass Criteria

- Guard passes with no regression.
- Guard fails on score drop beyond threshold.
- Guard fails when JSON output removed.
- Guard fails when new destructive command lacks mitigation.
- GitHub Action uploads artifacts.

### Current Checkpoint

The current implementation includes guard mode and CI artifacts:

- `cliare guard <TARGET> --baseline <scorecard.json>` measures the target and compares total score.
- `--allowed-drop <POINTS>` controls tolerated score regression.
- `--policy <FILE>` evaluates a `cliare.policy.v1` JSON policy after measurement.
- Policies support `min_total_score`, per-dimension `min_subscores`, side-effect `allow_paths`, `max_unapproved`, and `deny_credential_like`.
- Guard prints the measurement summary plus pass/fail comparison details.
- Guard rewrites `summary.md` and `junit.xml` with baseline score, current score, delta, allowed drop, policy status, policy failures, and pass/fail context.
- `findings.sarif` is emitted from the same scorecard findings that drive the Markdown and JUnit artifacts.
- The root `action.yml` composite action runs `measure` or `guard` in the caller's CI environment, accepts optional `policy` and `concurrency` inputs, uploads the artifact directory, appends `summary.md` to `$GITHUB_STEP_SUMMARY`, and exposes score/output paths.
- Fixture tests cover pass and fail behavior for total-score regressions, generated SARIF, generated JUnit XML, generated CI summaries, allowed cache writes, denied credential-like writes, output subscore thresholds, and total-score thresholds.

The default recursion budget and scheduler width have also been raised for real-world CLIs with deep subcommand hierarchies:

- `--profile quick` resolves to 3 command path segments, 64 probes, and concurrency 2.
- `--profile standard` resolves to 5 command path segments, 256 probes, and concurrency 4.
- `--profile deep` resolves to 8 command path segments, 1000 probes, and concurrency 8.
- `standard` is the default for `measure` and `guard`.
- `--max-depth` and `--max-probes` override the selected profile.
- Each profile also supplies a minimum expected-value threshold for dynamically scheduled probes.
- `--min-expected-value` overrides the selected profile's convergence threshold.
- `--concurrency` overrides the selected profile's concurrent probe limit.
- The planner still enforces both limits deterministically so CI runs stay bounded.
- The executor runs bounded async rounds and commits probe evidence in stable probe-id order.
- Each probe receives an isolated sandbox root under `sandbox/probes/<probe_id>` so concurrent side-effect snapshots cannot contaminate each other.

Coverage pressure is now explicit rather than hidden inside the score:

- terminal summaries show traversal profile, observed depth, depth budget, completed probes, probe budget, concurrency limit, scheduler rounds, scheduled probes, cancelled probes, remaining frontier, depth-skipped candidates, budget-skipped probes, and whether the run exhausted its budget
- terminal summaries also show the convergence threshold, highest pending expected value, convergence skips, stop reason, and traversal completion status
- `scorecard.json` records the same fields under `coverage`
- `report.md` renders the budget pressure fields for CI artifacts and human review
- score v0 does not directly penalize a CLI for being deep; it exposes budget pressure so callers can decide whether to rerun with a larger profile
- the planner now skips dynamic probes below the profile's expected-value threshold and counts those skips as convergence evidence
- stop reasons distinguish `converged`, `frontier_exhausted`, `depth_budget_exhausted`, and `probe_budget_exhausted`

Measurement cache reuse is now implemented:

- successful measurement writes `measure-cache.json`
- cache matching requires the same target fingerprint, traversal profile, resolved probe budget, expected-value threshold, concurrency limit, CLIARE package version, and measurement engine
- reusable cache requires `evidence.jsonl`, `shape.json`, `scorecard.json`, `report.md`, `summary.md`, `findings.sarif`, and `junit.xml` to still exist
- terminal summaries print `cache: hit` or `cache: miss`
- `--refresh` bypasses cache reuse for both `measure` and `guard`

Grammar extraction is now implemented for the current command-shape model:

- `shape.json` command entries include `aliases`, `positionals`, and `usage_observed`
- positional arguments are extracted from usage lines and include `name`, `required`, `variadic`, and evidence
- flag entries include `value_kind`, `value_name`, `required`, and `repeatable`
- supported flag value kinds are `boolean`, `required`, and `optional`
- usage parsing skips flag value placeholders so `--baseline <FILE> <TARGET>` does not misclassify `FILE` as a positional argument
- grammar scoring now credits extracted usage syntax and known flag arity instead of treating every confirmed command as arity-unknown

---

## Phase 5: Benchmark and Calibration

### Goal

Make the math credible.

### Tasks

1. Build synthetic fixture CLIs.
2. Build framework fixture CLIs.
3. Define ground truth schema.
4. Implement benchmark runner.
5. Compute Brier/log-loss metrics.
6. Add calibration plots/data.
7. Tune likelihood weights.
8. Publish calibration report.

### Checkpoint 5 Pass Criteria

- Inference metrics are computed against ground truth.
- Probability calibration is measured.
- False-safe rate is tracked.
- Score model version is documented.
- Public score remains labeled experimental until calibrated.

### Current Checkpoint

The current implementation has the first real CLI benchmark layer in place:

- `cliare benchmark --manifest benchmarks/local-corpus.json --out <DIR>` runs a manifest-defined corpus.
- The manifest schema is `cliare.benchmark-corpus.v1`.
- The report schema is `cliare.benchmark-report.v1`.
- Corpus defaults and per-target overrides cover traversal profile, depth, probe budget, expected-value threshold, per-probe concurrency, timeout, output limit, expected score band, runtime cap, and required/optional status.
- Target-level parallelism uses bounded `JoinSet` execution, and each target still uses the normal bounded async `measure` scheduler internally.
- Worker tasks write only their per-target measurement artifacts. The benchmark coordinator is the only writer for aggregate `benchmark.json` and `benchmark.md`.
- Aggregate reports are streamed after startup and after every completed target, then atomically replaced through temporary files and rename.
- A `.benchmark.lock` prevents two benchmark processes from writing the same output directory concurrently.
- Optional targets are skipped only for missing binaries; measurement errors in required targets fail the benchmark.
- Calibration aggregates include expected-band pass rate, traversal completion rate, budget exhaustion rate, score mean/range, total probes, findings, output-contract counts, parse successes, side-effect counts, and credential-like side effects.
- The local corpus currently covers `cliare`, `rote`, `git`, `supabase`, `gh`, `cargo`, `npm`, `docker`, and `deno`.

The latest deep local corpus run measured all nine targets, completed 2706 probes, produced a 100% expected-band pass rate, and finished without failed targets.

The remaining Phase 5 work is deeper calibration, not basic benchmark execution:

- human-verified truth subsets for selected command families
- Brier score, log loss, expected calibration error, and false-safe rate
- likelihood-weight tuning from benchmark results
- published model-version calibration reports
- score stability runs over repeated measurements

---

## Phase 6: Public Scorecard Publishing

### Goal

Allow projects to publish scorecards without uploading binaries.

### Tasks

1. Define publish API.
2. Implement scorecard validation service.
3. Add GitHub OIDC attestation support.
4. Store immutable scorecard records.
5. Display basic public page.
6. Implement verification levels.

### Checkpoint 6 Pass Criteria

- Valid scorecard accepted.
- Invalid schema rejected.
- GitHub provenance verified.
- Public page renders score and subscores.
- Verification level displayed.

---

## Phase 7: Leaderboard and Badge

### Goal

Create the public GTM loop.

### Tasks

1. Implement badge endpoint.
2. Implement leaderboard views.
3. Add filters by verification/profile/category.
4. Add trend view.
5. Add "most improved" view.
6. Add project category metadata.

### Checkpoint 7 Pass Criteria

- README badge works.
- Leaderboard filters by verified scorecards.
- Project page shows trend.
- Most-improved view computes deltas.

---

## Phase 8: v1 Standard

### Goal

Stabilize CLIARE as a standard.

### Tasks

1. Freeze v1 schemas.
2. Freeze score model v1.
3. Publish calibration report.
4. Publish governance process.
5. Document compatibility policy.
6. Encourage third-party implementations.
7. Publish reference benchmark corpus.

### Checkpoint 8 Pass Criteria

- v1 spec is stable.
- Score model has published calibration metrics.
- Two independent consumers can read scorecard.
- Leaderboard uses v1 model.
- Badge no longer says experimental.

---

## MVP Cut

Minimum useful launch:

- local binary
- safe profile
- evidence log
- shape inference from help/errors
- scorecard
- Markdown report
- baseline guard
- GitHub Action
- synthetic fixtures
- real CLI benchmark corpus with expected score bands

Do not block MVP on:

- hosted leaderboard
- network tracing
- Docker sandbox
- full completion support
- perfect calibration

But do not compromise on:

- evidence references
- confidence fields
- schema validation
- sandbox basics
- score decomposition

---

## Open Technical Decisions

1. Rust workspace vs mixed Rust/TypeScript monorepo.
2. Default license.
3. Whether evidence logs include inline stdout by default.
4. Which sandbox backend is required for certified score.
5. How to normalize scores across profiles.
6. How to categorize CLIs for leaderboard.
7. Whether hosted publish is under `cliare.dev`, `modiqo.dev/cliare`, or both.
8. How strict v1 compatibility should be.

---

## Immediate Next Steps

1. Decide final name and repo.
2. Create initial OSS repo.
3. Move this design packet into `docs/design`.
4. Write schemas first.
5. Implement portable sandbox.
6. Build three synthetic fixture CLIs.
7. Ship first internal dogfood score.
