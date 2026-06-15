# 25 - Calibration Workflow TODO

> **Scope:** Near-term implementation plan for turning CLIARE measurement artifacts into calibrated model evaluation.
> **Status:** TODO

---

## Purpose

This document records the calibration workflow that should be added before CLIARE claims a certified public score model.

CLIARE already produces runtime evidence, command shape, command indexes, issue ledgers, persona reports, scorecards, and benchmark reports. Those artifacts are sufficient to start calibration work, but they are not a substitute for ground truth. Calibration must compare CLIARE predictions against human-reviewed labels rather than training the system to agree with its own current scores.

The immediate goal is to create a disciplined workflow:

```text
measure -> scaffold truth set -> label -> validate corpus -> evaluate model -> report calibration quality
```

The workflow should make `cliare-score-v0` measurable against truth sets and create the path toward a future `cliare-score-v1`. It should not prematurely publish `cliare-score-v1`.

---

## Current Position

The current score model is:

```text
model id: cliare-score-v0
status: experimental_partial
calibration stage: uncalibrated_v0
schema: cliare.score-model.v1
```

The score model is now a typed, bundled artifact at:

```text
score-models/cliare-score-v0.json
```

The Rust implementation validates that artifact through:

```text
src/score_model.rs
```

This is the right foundation. We can audit model identity, model hash, dimension weights, scoring coefficients, priors, evidence weights, thresholds, and calibration requirements. The next step is not to rename the model to v1. The next step is to measure whether the model is right.

---

## Calibration Levels

CLIARE should support three levels of calibration maturity.

| Level | Input | Output | Appropriate Use |
|---|---|---|---|
| L0 empirical baseline | Raw CLIARE runs only | Score distributions, benchmark bands, parser defects, run stability | Internal QA and regression tracking |
| L1 observed-fact calibration | Runtime-observed facts such as parse success, timeouts, side effects, and precondition-blocked probes | Execution, output, recovery, and safety quality metrics | CI model hardening |
| L2 truth-set calibration | Human-reviewed labels for commands, flags, arity, preconditions, output contracts, and side effects | Proper calibration metrics and model-selection evidence | Candidate public standard model |

Only L2 should support a certified score model.

---

## Command Surface TODO

Add a `calibrate` command group with focused subcommands.

### `cliare calibrate init`

Scaffold a truth-set directory from an existing measurement artifact.

Example:

```sh
cliare calibrate init \
  --measurement .cliare-vendor-calibration/gh/contexts/repo-host \
  --out benchmarks/corpus/gh
```

Responsibilities:

- read `scorecard.json`, `shape.json`, `command-index.json`, `issues.json`, and `runtime-context.json`
- create a corpus directory if needed
- write `truth.json` with candidate labels marked `unknown`
- write `notes.md` with reviewer instructions and artifact provenance
- preserve source artifact paths and hashes
- never overwrite human-reviewed labels unless explicitly requested

### `cliare calibrate check`

Validate corpus structure and labeling readiness.

Example:

```sh
cliare calibrate check benchmarks/corpus
```

Responsibilities:

- validate `cliare.truth.v1`
- verify split assignments
- detect missing target metadata
- detect unlabeled required fields
- warn about train, validation, and holdout leakage
- ensure labels are scoped to runtime context
- report whether a corpus is suitable for L0, L1, or L2 calibration

### `cliare calibrate evaluate`

Compare CLIARE predictions against truth labels and emit calibration metrics.

Example:

```sh
cliare calibrate evaluate \
  --corpus benchmarks/corpus \
  --artifacts .cliare-vendor-calibration \
  --model cliare-score-v0 \
  --out .cliare-calibration
```

Responsibilities:

- extract predicted claims from CLIARE artifacts
- match predictions to truth labels
- compute binary, categorical, and safety metrics
- write JSON and Markdown reports
- keep train, validation, and holdout results separate
- refuse to report holdout metrics when holdout labels are missing

### Deferred: `cliare calibrate fit`

Do not add model fitting until enough labeled train, validation, and holdout data exists.

When added, `fit` should refuse to run unless the corpus meets minimum labeling thresholds. It should produce a candidate model artifact, not overwrite the bundled model:

```text
score-models/cliare-score-v1-candidate.json
```

---

## Artifact TODO

The calibration workflow should produce:

```text
.cliare-calibration/
  calibration-report.json
  calibration-report.md
  claim-metrics.json
  claim-calibration.md
  precondition-confusion.md
  output-contract-metrics.md
  safety-metrics.md
  coverage-gaps.md
```

The corpus side should use:

```text
benchmarks/corpus/<cli-id>/
  README.md
  truth.json
  expected.json
  notes.md
  fixtures/
```

The tracker remains:

```text
docs/24-cli-benchmark-corpus-tracker.md
```

---

## Truth-Set Schema TODO

Add a typed `cliare.truth.v1` model.

The schema should support at least:

- target id, binary path, version, OS, architecture, and install source
- runtime context name and context states
- corpus split: train, validation, holdout, or excluded
- review status: scaffolded, partial, human_reviewed, disputed
- command labels
- flag labels
- flag arity and value-domain labels
- positional labels
- precondition labels
- output contract labels
- parseability labels
- side-effect labels
- destructive-risk labels
- reviewer notes and evidence references

Truth labels should use explicit states:

| Label | Meaning |
|---|---|
| `true` | The claim is verified correct. |
| `false` | The claim is verified incorrect. |
| `unknown` | The reviewer cannot determine the answer from available evidence. |
| `conditional` | The claim is true only under a named runtime condition. |
| `out_of_scope` | The claim is outside the calibrated subset. |

Negative labels are mandatory. A calibration corpus that only records true commands cannot measure invented commands or false confidence.

---

## Prediction Model TODO

Calibration should compare truth against normalized predictions rather than raw JSON fields.

Add a prediction layer with comparable claims:

```text
PredictedClaim {
  key,
  kind,
  predicted_label,
  probability,
  runtime_context,
  evidence_refs
}

TruthClaim {
  key,
  kind,
  truth_label,
  runtime_condition,
  review_status,
  evidence_refs
}
```

Claim kinds should include:

- command exists
- flag exists
- flag arity
- positional exists
- precondition kind
- output contract exists
- output parseable
- side-effect class
- destructive-risk class

This separation is important. `shape.json`, `command-index.json`, and `scorecard.json` can evolve without rewriting every metric.

---

## Metric TODO

The first evaluation implementation should include:

| Metric | Applies To |
|---|---|
| Brier score | binary claim confidence |
| log loss | binary claim confidence |
| expected calibration error | confidence calibration |
| precision / recall / F1 | command and flag discovery |
| false confirmed command rate | command existence |
| false missing command rate | command existence |
| confusion matrix | precondition classification |
| arity accuracy | flag grammar |
| parseability accuracy | output contracts |
| false-safe rate | safety classification |
| repeated-run score variance | stability |
| depth-weighted recall | deep command surfaces |

The reports should make clear which metrics are unavailable because labels are missing.

---

## Split Discipline

Calibration splits must be by CLI family, not by individual command rows.

If `gh` is used to tune output-field probing, it cannot also be used as pristine holdout evidence for that same model change. The current vendor tracker marks splits as provisional. `calibrate check` should report leakage risks when a CLI family appears in multiple roles.

Recommended first pass:

| Split | Examples | Purpose |
|---|---|---|
| Train | Stripe, Valyu, PostHog, AgentMail | Tune priors, evidence weights, and thresholds |
| Validation | Supabase, ElevenLabs, Vercel | Select candidate model revisions |
| Holdout | Ramp, Google Workspace, selected unseen CLIs | Final model-quality report |

GitHub CLI should remain a reference target, but it should not be treated as pristine holdout for output-field probing because it has already influenced implementation fixes.

---

## Context Discipline

Calibration labels must be scoped to runtime context.

The same binary may have different truthful behavior under:

```text
clean
authenticated
local_context
fixture
authenticated_local_context
```

For example, a command may be true and parseable inside a repository with auth and network, while the clean context truth label is conditional on local context and authentication. Calibration should not flatten those states into one score.

---

## Quality Requirements

The calibration implementation should follow these engineering constraints:

- Keep calibration code in a dedicated module tree.
- Keep command orchestration separate from truth parsing, artifact reading, prediction extraction, metric computation, and report rendering.
- Use typed structs and enums for truth labels, claim kinds, runtime conditions, metric names, and split names.
- Avoid repeated ad hoc JSON traversal.
- Avoid stringly typed metric keys in core logic.
- Avoid report generation inside metric functions.
- Do not add a god struct for every calibration field.
- Preserve unknown labels explicitly; do not coerce them to false.
- Return explicit errors for invalid schemas, missing artifacts, unsupported model ids, and split leakage.
- Write deterministic reports with stable ordering.
- Add fixture tests for every metric and schema validation path.

Recommended module layout:

```text
src/calibration/
  mod.rs
  command.rs
  truth.rs
  corpus.rs
  artifact.rs
  prediction.rs
  metrics.rs
  report.rs
```

The key design boundary is:

```text
truth + prediction -> matched claims -> metrics -> reports
```

That boundary should remain stable even as schemas grow.

---

## Acceptance Criteria

The first calibration checkpoint is complete when:

- `cliare calibrate init` scaffolds a reviewable truth directory from one measurement artifact.
- `cliare calibrate check` validates corpus structure and split discipline.
- `cliare calibrate evaluate` reports metrics against labeled truth files.
- Reports distinguish missing labels from failed predictions.
- Metrics are split by train, validation, and holdout.
- `cliare-score-v0` remains the active model.
- The code path is covered by unit and fixture tests.
- Documentation explains that calibration is required before `cliare-score-v1` can be frozen.

---

## Non-Goals

This checkpoint should not:

- rename `cliare-score-v0` to `cliare-score-v1`
- tune weights from unlabeled scorecards
- treat CLIARE's own current predictions as truth
- publish leaderboard authority
- hide context differences behind one global score
- add fitting before sufficient labels exist

---

## Immediate Next Step

Start with one reference target and one validation target:

```text
gh
supabase
```

For each target:

1. Run a fresh measurement under a named runtime context.
2. Generate the artifact map.
3. Scaffold `benchmarks/corpus/<target>/truth.json`.
4. Human-label a small but representative slice.
5. Run `calibrate check`.
6. Run `calibrate evaluate`.
7. Expand labels only after the first report proves the workflow is usable.
