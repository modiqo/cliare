# 18 - Calibration and Leaderboard Authority

> **Scope:** What CLIARE must build before a public leaderboard score should be considered authoritative.
> **Status:** Standard-readiness plan

---

## Summary

CLIARE can be useful before it is authoritative.

The current `cliare-score-v0` is already useful for:

- local CLI quality measurement
- release-to-release drift detection
- CI regression gates
- internal scorecards
- benchmark corpus experimentation
- agent harness navigation artifacts

An authoritative public leaderboard needs a higher bar. It must prove that scores are calibrated, reproducible, comparable, hard to game, and tied to transparent evidence.

This document defines that bar.

The target posture is:

```text
v0: evidence-backed experimental score for CI and iterative improvement
v1: calibrated public standard score for certification and leaderboard ranking
```

---

## Why Calibration Must Be A Product Feature

If CLIARE is going to become a standard, calibration cannot be an internal QA note. It has to be part of the product surface.

A maintainer, agent harness author, or leaderboard viewer should be able to ask:

- How accurate is CLIARE at command discovery?
- When CLIARE says confidence is 0.80, is it right about 80% of the time?
- How often does it invent commands?
- How often does it miss real commands?
- How often does it classify unsafe behavior as safe?
- How stable is the score across repeated runs?
- Which model version produced this score?
- Which corpus calibrated that model?
- Can this score be reproduced locally?

Without those answers, a score is a useful engineering signal but not an authoritative public ranking.

---

## Authority Requirements

A CLIARE leaderboard score should be considered authoritative only when all of these are true:

1. The score uses a frozen model version such as `cliare-score-v1`.
2. The inference model and scoring model are both versioned in the scorecard.
3. The run uses a documented certified profile.
4. The target binary fingerprint is included.
5. The CLIARE binary version is included.
6. The scorecard includes profile, sandbox, OS, architecture, and runtime metadata.
7. The score model has a published calibration report.
8. The calibration corpus version is recorded.
9. Claim accuracy is evaluated against human-reviewed truth sets.
10. Safety classification reports false-safe rate.
11. Repeated-run stability is within published thresholds.
12. Public ranking separates verified CI runs from self-reported local runs.
13. Old score model versions are not mixed with current model rankings.
14. Users can reproduce the score locally from the same binary and profile.

If any of these are missing, the score can still be displayed, but it should be labeled experimental, unverified, stale, or profile-specific.

---

## Calibration Corpus

CLIARE needs a versioned calibration corpus with two layers:

1. Synthetic truth corpus.
2. Real CLI truth corpus.

Both are necessary.

Synthetic CLIs let CLIARE test exact behavior, edge cases, and adversarial constructions. Real CLIs prove the model generalizes beyond fixtures.

### Synthetic Truth Corpus

The synthetic corpus should include purpose-built black-box CLIs where the full command surface is known.

Required families:

| Family | Purpose |
|---|---|
| clear-help | Ideal command and flag help |
| poor-help | Sparse or misleading help |
| no-help | Runtime behavior with little documentation |
| completion-only | Surface exposed mostly through completions |
| custom-parser | No framework conventions |
| deep-tree | Deep nested subcommands |
| huge-surface | Large fanout and budget pressure |
| auth-gated | Commands blocked by login/profile preconditions |
| plugin-surface | Commands discovered only through plugin state |
| json-good | Valid machine-readable output |
| json-mixed | JSON contaminated with prose/progress |
| table-only | Human-readable but machine-poor output |
| cache-writer | Benign discovery-time file writes |
| credential-writer | Unsafe credential-like writes |
| destructive-risk | Mutating commands with and without guardrails |
| interactive | Prompts, TTY checks, and noninteractive failures |
| nondeterministic | Time, order, and randomized output drift |
| adversarial-help | Rows designed to fool naive parsers |

Synthetic targets should be fast, deterministic by default, and portable across Linux and macOS. Nondeterministic fixtures should be explicit and isolated.

### Real CLI Truth Corpus

The real corpus should include popular and long-tail CLIs.

Initial targets:

- `cliare`
- `rote`
- `git`
- `gh`
- `docker`
- `kubectl`
- `cargo`
- `npm`
- `deno`
- `supabase`
- `terraform`
- `aws`
- `gcloud`
- `vercel`
- `stripe`

Real truth sets do not need complete coverage on day one. They should start with representative command families and grow over time.

Each target should record:

- target id
- binary name
- exact version
- OS and architecture
- installation source
- expected command families
- known plugin dependencies
- known auth/profile preconditions
- known safe probe boundaries
- expected output modes
- expected side-effect classes
- expected score band for current model

---

## Truth Set Schema

Truth sets should use the same vocabulary as `shape.json`, but with reviewer-grade certainty.

Example:

```json
{
  "schema_version": "cliare.truth.v1",
  "target": {
    "id": "git",
    "binary": "git",
    "version": "2.45.0",
    "platform": "darwin-arm64"
  },
  "review": {
    "status": "human_reviewed",
    "reviewers": 2,
    "last_reviewed": "2026-06-13"
  },
  "commands": [
    {
      "path": ["git", "remote", "add"],
      "exists": true,
      "runtime_state": "runtime_confirmed",
      "flags": [
        {
          "name": "--fetch",
          "exists": true,
          "arity": "boolean",
          "required": false,
          "repeatable": false
        }
      ],
      "positionals": [
        {
          "name": "name",
          "required": true,
          "variadic": false
        },
        {
          "name": "url",
          "required": true,
          "variadic": false
        }
      ],
      "output_modes": [],
      "side_effect_class": "local_write",
      "destructive_risk": "low"
    }
  ],
  "negative_claims": [
    {
      "kind": "command_exists",
      "path": ["git", "not-a-real-command"],
      "expected": false
    }
  ]
}
```

Negative claims are mandatory. A model cannot be calibrated if the corpus only says what exists. CLIARE must know when it invented a command, flag, output mode, or safety property.

---

## Human Review Discipline

Real CLI truth sets need human review because CLIs are messy.

Review process:

1. CLIARE proposes a shape.
2. Reviewer A labels a subset as true, false, or unknown.
3. Reviewer B independently labels the same subset.
4. Disagreements are resolved and recorded.
5. The truth set is versioned.

Reviewer labels:

| Label | Meaning |
|---|---|
| true | Claim is verified correct |
| false | Claim is verified incorrect |
| unknown | Claim cannot be determined reliably |
| conditional | Claim is true only under named runtime conditions |
| out_of_scope | Claim is outside the calibrated subset |

Precondition-gated commands should not be labeled false just because they cannot be executed in a clean runtime. They should be labeled conditional with an explicit precondition such as `auth_required`, `profile_required`, or `local_context_required`. The diagnostic text that motivated the label should be stored as calibration evidence, but production classifiers should prefer diagnostic structure, recovery actions, and calibrated token features over one-off phrase lists.

---

## Calibration Metrics

CLIARE should publish metrics for every inference and score model version.

### Binary Claim Metrics

Used for claims such as command existence, flag existence, dry-run support, parse success, and precondition detection.

Required metrics:

- accuracy
- precision
- recall
- F1
- Brier score
- log loss
- expected calibration error
- false positive rate
- false negative rate

Brier score:

```text
Brier = mean_i (p_i - y_i)^2
```

Log loss:

```text
LogLoss = -mean_i [y_i log(p_i) + (1 - y_i) log(1 - p_i)]
```

Expected calibration error:

```text
ECE = sum_b (n_b / n) * abs(acc(b) - conf(b))
```

Where `b` is a confidence bucket.

### Categorical Claim Metrics

Used for arity, output kind, runtime state, side-effect class, and destructive-risk class.

Required metrics:

- top-1 accuracy
- top-3 accuracy where relevant
- macro F1
- categorical log loss
- expected calibration error
- confusion matrix

### Discovery Metrics

Required command discovery metrics:

- command precision
- command recall
- command F1
- false discovered command count
- missed command count
- depth-weighted recall
- command family recall
- budget-adjusted recall

Depth-weighted recall matters because many popular CLIs have deep surfaces. A model that only performs well at depth 1 should not look strong.

### Grammar Metrics

Required grammar metrics:

- flag existence precision, recall, and F1
- flag arity accuracy
- positional requiredness accuracy
- variadic detection accuracy
- enum/value-domain extraction accuracy
- required flag accuracy
- repeatable flag accuracy

### Output Metrics

Required output metrics:

- machine-readable mode precision
- machine-readable mode recall
- parse success precision
- false advertised mode rate
- mixed-output detection recall
- stdout/stderr separation issue rate

### Safety Metrics

Safety needs stricter treatment than discovery.

Required safety metrics:

- unsafe recall
- destructive-risk recall
- credential-like side-effect recall
- side-effect class accuracy
- dry-run detection accuracy
- false-safe rate

False-safe rate:

```text
false_safe_rate = unsafe_or_destructive_claims_labeled_safe / unsafe_or_destructive_truth
```

False-safe rate should be a headline calibration metric. A model that misses commands is annoying. A model that classifies unsafe behavior as safe is dangerous.

---

## Score Stability

An authoritative score must be stable.

Run the same target multiple times under the same profile and environment class.

Required measurements:

- total score mean
- total score standard deviation
- subscore standard deviation
- shape diff rate
- finding diff rate
- timeout variance
- output parse variance
- side-effect variance

Suggested thresholds for certified deterministic targets:

```text
total_score_stddev <= 1.0
subscore_stddev <= 2.0
shape_jaccard_similarity >= 0.98
finding_set_jaccard_similarity >= 0.95
guard_pass_fail_flaps = 0
```

Nondeterministic CLIs can still be measured, but the scorecard must expose variance and confidence intervals.

---

## Confidence Intervals

Leaderboard scores should display intervals, not only point estimates.

Example:

```text
CLIARE Score: 84.2
95% interval: 81.0 - 87.4
Model: cliare-score-v1
Profile: certified
Calibration corpus: cliare-calibration-2026-06
```

Intervals can come from:

- posterior sampling over claim probabilities
- bootstrap resampling over commands and probes
- repeated-run variance
- calibration uncertainty from truth-set metrics

The interval should widen when:

- the traversal is incomplete
- the frontier still contains high-value candidates
- claim confidence is weak
- repeated runs are unstable
- output parsing is nondeterministic
- safety classification lacks direct evidence

---

## Certified Profiles

Leaderboard scores should not mix arbitrary budgets and profiles.

CLIARE should define named profiles:

| Profile | Purpose | Public ranking use |
|---|---|---|
| quick | Local developer feedback | no |
| standard | Normal CI gate | maybe, labeled |
| deep | Release readiness and large surfaces | maybe, labeled |
| certified | Public leaderboard and badge | yes |

The certified profile should specify:

- maximum depth
- maximum probes
- expected-value threshold
- concurrency rules
- timeout policy
- output byte limit
- sandbox profile
- environment policy
- network policy
- retry policy
- minimum repeated runs
- required artifact set

The leaderboard should rank only within the same profile and score model.

---

## Provenance and Verification Levels

Every scorecard needs provenance.

Required fields:

- scorecard schema version
- score model version
- inference model version
- calibration corpus version
- CLIARE binary version
- target binary path and hash
- target version
- OS and architecture
- sandbox profile
- traversal profile
- run timestamp
- artifact hashes
- CI provider metadata when available
- repository and commit metadata when available

Verification levels:

| Level | Meaning |
|---|---|
| local_unverified | User ran CLIARE locally; no external attestation |
| ci_attested | Score came from a recognized CI provider with artifact hashes |
| repo_verified | CI run is tied to the claimed repository and commit |
| release_verified | Score is tied to a signed release artifact |
| certified | Score uses certified profile, current model, calibration version, and required repeatability |

Leaderboards should show verification level beside every score.

---

## Anti-Gaming Requirements

Any public score can be gamed. CLIARE should assume maintainers may optimize for the score.

Adversarial fixtures should include:

- help output that lists commands that do not exist
- help output that hides commands that do exist
- prose that looks like command rows
- numeric menus that look like subcommands
- fake JSON modes
- JSON mixed with warnings or progress text
- CLIs that write token-like files during help
- CLIs that behave differently when `CLIARE` appears in the environment
- CLIs that pass `--help` but hang on subcommand help
- CLIs that expose huge low-value surfaces to exhaust budgets
- destructive commands hidden behind friendly names
- commands that exit zero for invalid flags
- commands that print stack traces instead of diagnostics

Anti-gaming checks:

- runtime confirmation must outweigh static help claims
- side effects must be observed, not inferred from names alone
- safety should prefer unknown over false safe
- budget pressure must be visible in scorecards
- certified runs should sanitize CLIARE-specific environment markers
- public scorecards should expose enough metadata for reproducibility

---

## The `calibrate` Command

CLIARE should expose calibration as a first-class command.

Proposed interface:

```sh
cliare calibrate \
  --corpus benchmarks/corpus.json \
  --truth benchmarks/truth \
  --out .cliare-calibration
```

Useful options:

```sh
cliare calibrate \
  --corpus benchmarks/corpus.json \
  --truth benchmarks/truth \
  --profile certified \
  --runs 5 \
  --target-concurrency 3 \
  --model cliare-score-v1-candidate \
  --out .cliare-calibration
```

Artifacts:

```text
.cliare-calibration/
  calibration.json
  calibration.md
  claim-metrics.json
  score-stability.json
  truth-coverage.json
  confusion-matrices.json
  false-safe-report.json
  model-recommendations.md
```

`calibration.json` should include:

- corpus version
- model versions
- target list
- truth coverage
- binary claim metrics
- categorical claim metrics
- score stability metrics
- false-safe rate
- profile metadata
- pass/fail against release thresholds

---

## Calibration Report Example

Example report shape:

```text
Model: cliare-score-v1-candidate
Inference: cliare-infer-v1-candidate
Corpus: cliare-calibration-2026-06
Profile: certified
Targets: 42
Repeated runs: 5

Command existence:
  Precision: 0.982
  Recall: 0.941
  F1: 0.961
  Brier: 0.028
  LogLoss: 0.091
  ECE: 0.037

Flag existence:
  Precision: 0.963
  Recall: 0.912
  F1: 0.937

Output modes:
  Parse precision: 0.971
  Mixed-output recall: 0.884

Safety:
  Unsafe recall: 0.946
  False-safe rate: 0.011

Stability:
  Total score stddev p95: 0.72
  Subscore stddev p95: 1.61
  Guard flaps: 0

Decision:
  Not ready for v1 freeze

Blocking issues:
  - Mixed-output recall below 0.90 threshold
  - Deep command recall below 0.90 for plugin-heavy CLIs
```

The report should be public for every frozen model.

---

## Model Freeze Criteria

`cliare-score-v1` should not be frozen until release thresholds are met.

Suggested initial thresholds:

| Metric | Threshold |
|---|---:|
| Command precision | `>= 0.97` |
| Command recall | `>= 0.92` |
| Flag existence F1 | `>= 0.90` |
| Flag arity accuracy | `>= 0.90` |
| Runtime state accuracy | `>= 0.92` |
| Machine-output parse precision | `>= 0.95` |
| Mixed-output recall | `>= 0.90` |
| Unsafe recall | `>= 0.95` |
| False-safe rate | `<= 0.02` |
| Total score stddev p95 | `<= 1.0` |
| Guard pass/fail flaps | `0` |

These thresholds should be treated as governance inputs, not eternal constants. They can evolve, but each model version must publish the thresholds it was judged against.

---

## Leaderboard Display Rules

The leaderboard should make trust legible.

Every public entry should show:

- score
- confidence interval
- score model
- profile
- verification level
- target version
- date measured
- traversal completion state
- budget exhaustion state
- precondition-blocked count
- safety findings count
- calibration corpus version

Entries should be grouped by:

- score model
- certified profile
- target category
- verification level

The leaderboard should not rank an experimental `quick` profile score above a certified score. It can display both, but they belong in different lanes.

---

## Public Badges

Badges should be strict about model and verification level.

Examples:

```text
CLIARE certified: 91.4
CLIARE CI: 86.2
CLIARE experimental: 74.8
```

Badge metadata should include:

- model
- profile
- verification level
- measured date
- target version
- artifact hash

If the model is stale or the scorecard is older than the configured freshness window, the badge should say so.

---

## Implementation Roadmap

### Phase 1: Truth Artifacts

- Define `cliare.truth.v1`.
- Add synthetic fixture truth files.
- Add partial truth files for `cliare`, `rote`, `git`, `gh`, `docker`, and `supabase`.
- Add truth validation tests.

### Phase 2: Claim Comparison

- Implement shape-to-truth comparison.
- Emit per-claim true positive, false positive, false negative, and unknown records.
- Support scoped truth subsets so partial real CLI truth sets are valid.

### Phase 3: Metrics Engine

- Compute binary claim metrics.
- Compute categorical claim metrics.
- Compute confusion matrices.
- Compute false-safe rate.
- Emit `claim-metrics.json`.

### Phase 4: Stability Runs

- Run each target multiple times.
- Compare score, shape, findings, and side effects.
- Emit `score-stability.json`.

### Phase 5: Calibration Command

- Add `cliare calibrate`.
- Wire corpus, truth, benchmark execution, metrics, and stability reports.
- Produce `calibration.json` and `calibration.md`.

### Phase 6: Candidate Model Tuning

- Tune priors and likelihood weights against truth metrics.
- Track score movement against expected score bands.
- Publish candidate calibration reports.

### Phase 7: Certified Profile

- Define certified runtime profile.
- Add repeatability and provenance requirements.
- Require certified profile for public ranking.

### Phase 8: v1 Freeze

- Freeze `cliare-score-v1`.
- Publish calibration report.
- Publish schema versions.
- Start leaderboard ingestion for certified scorecards.

---

## Relationship To Existing Commands

`measure` remains the normal local command:

```sh
cliare measure ./mycli
```

`guard` remains the CI regression command:

```sh
cliare guard ./mycli --baseline .cliare/baseline.scorecard.json
```

`benchmark` remains the corpus execution command:

```sh
cliare benchmark --manifest benchmarks/local-corpus.json --out .cliare-bench
```

`calibrate` should compose benchmark results with truth sets and repeated-run stability:

```sh
cliare calibrate --corpus benchmarks/corpus.json --truth benchmarks/truth --out .cliare-calibration
```

`certify` should eventually compose measurement, policy, provenance, certified profile rules, and current model compatibility:

```sh
cliare certify ./mycli --profile certified --policy cliare.policy.json
```

---

## Certification Definition

CLIARE is ready for certified public scoring when:

1. A maintainer can run it locally and get the same score as CI for the same binary/profile.
2. The scorecard explains every score contribution.
3. The shape catalog is useful to agent harnesses.
4. The calibration report proves confidence values are meaningful.
5. Safety metrics prioritize avoiding false-safe classifications.
6. Public leaderboard entries are separated by profile, model, and verification level.
7. Model changes are governed, versioned, and reproducible.

At that point, CLIARE becomes shared infrastructure for the CLI-agent ecosystem rather than only a local measurement tool.
