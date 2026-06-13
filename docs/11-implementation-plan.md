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

The current implementation has the first generic claim pipeline in place:

- `claims` converts observations into command and flag beliefs.
- `planner` ranks confirmation and diagnostic probes deterministically.
- `shape` emits the catalog from claims rather than from a framework parser.
- Invalid-child probes are gated on evidence of nested commands so leaf commands with positionals are not misclassified as command trees.
- Fixture CLI integration tests cover custom help, aliases, noisy help, and runtime false-positive rejection.

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

The current implementation emits experimental partial `scorecard.json` and `report.md` artifacts from `measure`.

- Discovery, grammar, execution, and recovery are scored from current evidence.
- Output and safety are present as `not_measured` dimensions until dedicated probes exist.
- Fixture tests verify that a clearer CLI scores higher than a poor CLI.
- The Markdown report explains the partial score, measured coverage, and findings.
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

The current implementation includes a first guard mode:

- `cliare guard <TARGET> --baseline <scorecard.json>` measures the target and compares total score.
- `--allowed-drop <POINTS>` controls tolerated score regression.
- Guard prints the measurement summary plus pass/fail comparison details.
- Fixture tests cover pass and fail behavior for total-score regressions.

The default recursion budget has also been raised for real-world CLIs with deep subcommand hierarchies:

- `--max-depth` defaults to 5 command path segments.
- `--max-probes` defaults to 256 probes.
- `measure` and `guard` share the same defaults.
- The planner still enforces both limits deterministically so CI runs stay bounded.

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

Do not block MVP on:

- hosted leaderboard
- network tracing
- Docker sandbox
- real-world benchmark corpus
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
