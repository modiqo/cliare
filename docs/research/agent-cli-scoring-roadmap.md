# Agent CLI Scoring Roadmap

> **Scope:** Proposed scoring-model extensions for CLIARE's maintainer feedback and agent-shape use cases.
> **Status:** Design proposal; not current implementation.

---

## Current Baseline

The current CLIARE v0 model is a deterministic, evidence-backed scorecard. It
already has several strong properties:

- runtime evidence is the root proof source;
- command, flag, and output-contract claims carry confidence;
- scoring weights live in a bundled model artifact with a hash;
- scorecards include coverage, findings, model metadata, and runtime context;
- reports point maintainers at concrete improvements;
- command index artifacts give agents a structured view of the CLI surface.

The next model should preserve those properties while adding two explicit
top-level views:

```text
maintainer_readiness_score
shape_confidence_score
```

These are related but distinct.

- **Maintainer readiness** asks how agent-effective the CLI is.
- **Shape confidence** asks how safely and directly an agent can rely on the
  emitted shape before additional probing.

---

## Proposed Scorecard Structure

Future scorecards should contain:

```json
{
  "score": {
    "total": 82,
    "maintainer_readiness": 80,
    "shape_confidence": 86,
    "measured_weight": 0.95,
    "model": "cliare-score-vNext",
    "status": "experimental_calibrating"
  },
  "views": {
    "maintainer": {},
    "harness": {}
  },
  "subscores": {},
  "shape_quality": {},
  "coverage": {},
  "findings": [],
  "model": {}
}
```

`total` can remain as a compatibility field, but reports should explain which
view is being used.

---

## Maintainer Readiness Score

Maintainer readiness measures how much the CLI helps or hinders agent use.

```text
MR = weighted_mean(
  discovery,
  grammar,
  output,
  recovery,
  preconditions,
  safety,
  efficiency,
  stability
)
```

### Proposed Dimensions

| Dimension | Question | Current Coverage | Proposed Extension |
|---|---|---|---|
| Discovery | Can an agent find the command surface? | Current discovery score and command confidence. | Add command recall against truth sets, alias coverage, shell-completion evidence where safe. |
| Grammar | Can an agent construct valid calls? | Current flag confidence, value kind, grammar gaps. | Add positional arity, enum values, repeatability, mutually exclusive groups, required-together groups. |
| Output | Can an agent consume results? | Current machine-readable output contracts and parse probes. | Add schema stability, streaming/progress prefix handling, examples, output field selectors. |
| Recovery | Can an agent repair failures? | Current invalid-probe rejection and precondition recovery. | Add actionable diagnostic structure, suggested command validation, error category precision. |
| Preconditions | Does the CLI expose hidden state requirements? | Current auth/local-context/fixture/network/runtime precondition counts. | Promote preconditions to a first-class dimension with context/fixture declaration quality. |
| Safety | Can an agent explore without collateral damage? | Current filesystem side effects and credential-like path checks. | Add dry-run support, destructive-command affordance detection, network side effects, secret redaction. |
| Efficiency | How costly is successful use? | Current probe counts, budgets, timeouts. | Add harness step savings, repeated-call stability, output token cost, latency variance. |
| Stability | Does shape remain valid across versions and contexts? | Target fingerprint and context artifacts exist. | Add shape drift metrics, semantic version comparison, command deprecation and compatibility markers. |

### Maintainer Findings

Each readiness dimension should map to actions:

- "Add `--json` or `--format json` to these commands."
- "Expose required `<project_id>` fixture in help and examples."
- "Classify auth errors with a stable diagnostic and recovery command."
- "Add dry-run to commands that mutate persistent state."
- "Avoid printing progress text before JSON unless `--progress` is requested."
- "Make aliases visible or remove stale alias references."

The maintainer score should be useful even without a benchmarked agent run.

---

## Harness Shape Confidence Score

Shape confidence measures whether the emitted artifact is safe for a harness to
use as planning input.

```text
SC = weighted_mean(
  claim_confidence,
  evidence_provenance,
  runtime_confirmation,
  grammar_completeness,
  output_contract_confidence,
  precondition_confidence,
  safety_confidence,
  freshness
)
```

### Proposed Shape Confidence Dimensions

| Dimension | Meaning |
|---|---|
| Claim confidence | Average calibrated confidence for command, flag, positional, and output-contract claims. |
| Evidence provenance | Fraction of shape entries with evidence ids, target fingerprint, model hash, and runtime context. |
| Runtime confirmation | Fraction of command and output claims confirmed by execution rather than layout-only evidence. |
| Grammar completeness | Fraction of invocable commands with enough argument grammar to construct valid calls. |
| Output contract confidence | Fraction of advertised machine-readable outputs that were probed and parsed successfully. |
| Precondition confidence | Fraction of blocked commands with classified blockers and actionable recovery or fixture hints. |
| Safety confidence | Whether side-effect observation was supported, not truncated, and free of high-risk findings. |
| Freshness | Whether the shape target fingerprint and CLI version match the current executable. |

### Harness Policy From Shape Confidence

Harnesses should be able to use thresholds:

| Shape State | Suggested Harness Behavior |
|---|---|
| `ready` | Use directly when task intent matches and safety policy allows. |
| `conditional` | Use only after satisfying declared preconditions. |
| `needs_fixture` | Ask user or fixture provider for required operands before execution. |
| `candidate` | Verify with help or non-mutating probe before use. |
| `blocked` | Avoid unless user explicitly resolves blocker. |
| `unsafe` | Do not run automatically; require explicit approval or dry-run path. |

This turns shape confidence into an operational contract, not just a score.

---

## Proposed Formula Sketch

Let:

- `C_i` be a shape claim.
- `p_i` be calibrated confidence that `C_i` is true.
- `w_i` be workload or command-importance weight.
- `r_i` be runtime-confirmation multiplier.
- `s_i` be safety eligibility multiplier.
- `f_i` be freshness multiplier.

Then:

```text
shape_confidence = 100 * sum_i(w_i * p_i * r_i * s_i * f_i) / sum_i(w_i)
```

For maintainer readiness:

```text
maintainer_readiness =
  100 * E[U(agent, task, cli) | runtime evidence, model]
```

In vNext, `U` can remain an auditable weighted approximation:

```text
U =
  w_d * discovery_success
+ w_g * valid_invocation_probability
+ w_o * parseable_output_probability
+ w_r * recovery_probability
+ w_p * precondition_resolvability
+ w_s * safety_survival
+ w_e * efficiency
+ w_t * stability
```

The model artifact should declare all weights, thresholds, and calibration
status. The scorecard should embed model id and hash as it does today.

---

## Workload Weighting

The current score treats discovered surface mostly uniformly. Agent harnesses do
not use all commands equally. Future scoring should support optional workload
profiles:

| Workload Profile | Examples |
|---|---|
| `developer-default` | help, inspect, test, build, format, lint, status. |
| `ci-automation` | noninteractive execution, machine-readable output, stable exit codes. |
| `incident-debug` | read-only inspection, logs, diagnostics, service status. |
| `data-export` | list/get/export commands, pagination, JSON/YAML/CSV output. |
| `admin-mutating` | create/update/delete commands with dry-run, confirmation, rollback. |
| `agent-harness` | commands likely used by coding agents and terminal planners. |

Workload profiles should not hide raw coverage. They should produce additional
views over the same evidence.

---

## New Evidence To Add

The proposed model becomes stronger if CLIARE records more evidence types.

### Shell Completion Evidence

When safe and deterministic:

- detect shell-completion support,
- extract command and flag candidates,
- treat completions as weak evidence,
- never bypass runtime confirmation.

### Fixture Declaration Evidence

Allow maintainers to provide safe operands:

```text
fixtures:
  project_id: "cliare-test-project"
  input_file: "fixtures/input.json"
```

This lets CLIARE probe commands that are currently marked `needs_fixture`.

### Output Schema Evidence

For machine-readable output:

- record top-level JSON/YAML shape,
- record stable fields across repeated runs,
- record parse errors with byte offsets where practical,
- detect progress text mixed with JSON.

### Diagnostic Structure Evidence

Classify diagnostics by:

- stable error code,
- human message,
- machine-readable error format,
- recovery command,
- documentation link,
- precondition category.

### Safety Evidence

Extend beyond current filesystem side effects:

- network endpoint attempts when a containment backend supports it,
- child process spawn tree,
- destructive command affordances,
- dry-run/preview availability,
- credential and token exposure in stdout/stderr,
- rollback or undo hints.

---

## Calibration Plan

### Claim Calibration

For each claim family:

```text
predicted confidence -> truth label
```

Report:

- Brier score,
- log loss,
- expected calibration error,
- precision/recall at thresholds,
- reliability diagram buckets.

### Harness Utility Calibration

For each target/task/harness condition:

```text
shape features -> task success / cost / safety outcome
```

Fit dimension weights against:

- task success,
- step-count reduction,
- malformed-call reduction,
- unsafe-action reduction,
- output-parse success,
- recovery success.

### Maintainer Intervention Calibration

For each fix category:

```text
CLIARE finding -> maintainer change -> score delta -> harness outcome delta
```

This validates whether CLIARE recommendations actually improve agent use.

---

## Compatibility With Current v0

The proposal can be introduced incrementally.

### Phase 1: Add Views Without Changing Total

- Keep current `score.total`.
- Add `score.maintainer_readiness`.
- Add `score.shape_confidence`.
- Add `shape_quality` section.
- Keep status `experimental_partial`.

### Phase 2: Add New Evidence And Metrics

- Add fixture support.
- Add output schema summaries.
- Add shell-completion evidence.
- Add shape-quality evaluator for fixture truth sets.

### Phase 3: Calibrated vNext

- Publish model artifact with calibration metrics.
- Freeze train/validation/holdout corpus versions.
- Report confidence intervals.
- Promote status to `experimental_calibrated` only after holdout reports.

### Phase 4: Certified Profiles

- Publish certified profile definitions.
- Require reproducible containment backend for safety-sensitive profiles.
- Publish leaderboard only for certified profiles.

---

## Risks And Guardrails

| Risk | Guardrail |
|---|---|
| Score becomes opaque. | Keep model artifact explicit and hashed. |
| Harness over-trusts stale shape. | Include target fingerprint, version, freshness, and confidence thresholds. |
| Framework-specific shortcuts become truth. | Treat framework data as weak evidence; require runtime confirmation. |
| Safety score misses dangerous behavior. | Separate "not measured" from "safe"; report strict and measured views. |
| Shape doc becomes another stale doc. | Tie every claim to evidence ids and model version. |
| Calibration overfits popular CLIs. | Split by target family and version; hold out low-pretraining CLIs. |

---

## Recommended Next Work

1. Add a `shape_confidence` section to scorecard output using existing fields.
2. Add fixture-ground-truth shape evaluation for controlled CLIs.
3. Add harness A/B trace schema to measure shape consultation.
4. Add maintainer-readiness findings that map directly to the reference CLI
   behavior guide.
5. Add output schema summaries for JSON/YAML parse successes.
6. Add strict-vs-measured score reporting for dimensions that are not safely
   observed.
