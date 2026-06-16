# 18 - Calibration And Leaderboard Authority

> **Scope:** Governance requirements before CLIARE scores can be used for certified public ranking.
> **Status:** Future authority bar. Current `cliare-score-v0` is useful for local CI and improvement tracking, not certified leaderboard authority.

---

## Summary

CLIARE can be useful before it is authoritative.

The current `cliare-score-v0` is intended for:

- local CLI quality measurement
- release-to-release drift detection
- CI regression gates
- internal scorecards
- benchmark corpus experimentation
- command indexes for agent harnesses

A public leaderboard has a higher bar. It needs calibrated, reproducible, comparable scores that are hard to game and tied to transparent evidence.

The intended boundary is:

```text
cliare-score-v0  evidence-backed experimental score for CI and improvement
cliare-score-v1  future calibrated score eligible for certification and public ranking
```

Until that boundary is crossed, public language should describe CLIARE scores as experimental, profile-specific, and evidence-backed rather than certified.

---

## Why Calibration Is Required

A maintainer, agent harness author, or leaderboard viewer should be able to ask:

- How accurate is CLIARE at command discovery?
- When CLIARE reports confidence `0.80`, is it right about 80% of the time?
- How often does it invent commands or flags?
- How often does it miss real commands or flags?
- How often does it classify unsafe behavior as safe?
- How stable is the score across repeated runs?
- Which model version produced this score?
- Which corpus calibrated that model?
- Can this score be reproduced locally?

Without those answers, a score is still useful engineering feedback, but it is not an authoritative public ranking.

---

## Authority Requirements

A CLIARE leaderboard score should be considered authoritative only when all of these are true:

1. The score uses a frozen calibrated model version, such as a future `cliare-score-v1`.
2. The inference model and scoring model are versioned in the scorecard.
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
14. Users can reproduce the score locally from the same binary, profile, and runtime context.

If any of these are missing, the score can still be displayed, but it should be labeled experimental, unverified, stale, or profile-specific.

---

## Truth Corpus Requirement

Certified scoring requires a versioned truth corpus with two layers:

| Layer | Purpose |
| --- | --- |
| Synthetic truth corpus | Exact black-box fixtures with known command surfaces, output modes, side effects, and adversarial cases. |
| Real CLI truth corpus | Human-reviewed labels from popular and long-tail CLIs to prove the model generalizes beyond fixtures. |

Synthetic CLIs are necessary because they make edge cases testable and repeatable. Real CLIs are necessary because agent-facing CLI behavior is messy: plugin systems, auth gates, local workspace preconditions, inconsistent help, and output drift are normal.

Truth sets should record:

- target id
- binary name and version
- OS and architecture
- installation source
- runtime context
- known command families
- known plugin dependencies
- known auth/profile/local-context preconditions
- safe probe boundaries
- expected output modes
- expected side-effect classes
- reviewer status

Truth sets should use the same vocabulary as `shape.json` and `command-index.json`, but with reviewer-grade labels.

---

## Negative Labels Are Mandatory

A calibration corpus that only lists true commands cannot measure false positives.

Truth sets must include negative labels for claims CLIARE could invent incorrectly:

- command exists
- flag exists
- flag arity
- positional requiredness
- output mode exists
- output mode parses
- runtime precondition class
- side-effect class
- safety property

Precondition-gated commands should not be labeled false just because they cannot execute in a clean runtime. They should be labeled conditional with an explicit precondition such as `auth_required`, `local_context_required`, `fixture_required`, `network_unavailable`, or `runtime_dependency_unavailable`.

---

## Required Metrics

Every certified model version should publish calibration metrics.

Binary claim metrics:

- accuracy
- precision
- recall
- F1
- Brier score
- log loss
- expected calibration error
- false positive rate
- false negative rate

Categorical claim metrics:

- top-1 accuracy
- macro F1
- categorical log loss
- expected calibration error
- confusion matrix

Discovery metrics:

- command precision
- command recall
- command F1
- false discovered command count
- missed command count
- depth-weighted recall
- budget-adjusted recall

Grammar metrics:

- flag existence precision, recall, and F1
- flag arity accuracy
- positional requiredness accuracy
- variadic detection accuracy
- required flag accuracy
- repeatable flag accuracy

Output metrics:

- machine-readable mode precision
- machine-readable mode recall
- parse success precision
- false advertised mode rate
- mixed-output detection recall
- stdout/stderr separation issue rate

Safety metrics:

- unsafe recall
- destructive-risk recall
- credential-like side-effect recall
- side-effect class accuracy
- dry-run detection accuracy
- false-safe rate

False-safe rate should be a headline metric. A model that misses commands is frustrating. A model that classifies unsafe behavior as safe is dangerous.

---

## Score Stability

Authoritative scores must be stable under repeated measurement of the same target in the same runtime context.

Stability reports should include:

- total score mean
- total score standard deviation
- subscore standard deviation
- shape diff rate
- finding diff rate
- timeout variance
- output parse variance
- side-effect variance
- guard pass/fail flaps

Nondeterministic CLIs can still be measured, but their scorecards should expose variance and uncertainty rather than presenting a single point score as fully stable.

---

## Certified Profiles

Leaderboard scores should not mix arbitrary budgets and profiles.

Current profiles such as `quick`, `standard`, and `deep` are useful for local feedback, CI, and release-readiness exploration. A future certified leaderboard profile should specify:

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

The leaderboard should rank only within the same score model and comparable profile.

---

## Provenance And Verification Levels

Every public scorecard should carry enough provenance for reviewers to understand what was measured.

Required provenance includes:

- scorecard schema version
- score model version and hash
- inference model version
- calibration corpus version, once calibrated
- CLIARE binary version
- target binary path and hash
- target version
- OS and architecture
- sandbox profile
- traversal profile
- runtime context
- run timestamp
- artifact hashes
- CI provider metadata when available
- repository and commit metadata when available

Suggested verification levels:

| Level | Meaning |
| --- | --- |
| `local_unverified` | User ran CLIARE locally; no external attestation. |
| `ci_attested` | Score came from a recognized CI provider with artifact hashes. |
| `repo_verified` | CI run is tied to the claimed repository and commit. |
| `release_verified` | Score is tied to a signed release artifact. |
| `certified` | Score uses certified profile, current calibrated model, required provenance, and repeatability checks. |

Leaderboards should display verification level beside every score.

---

## Leaderboard Display Rules

A public leaderboard should make trust legible.

Every entry should show:

- score
- score interval or variance once available
- score model
- profile
- verification level
- target version
- date measured
- traversal completion state
- budget exhaustion state
- precondition-blocked count
- safety findings count
- calibration corpus version, once calibrated

Entries should be grouped or filtered by:

- score model
- profile
- target category
- verification level

An experimental `quick` profile score should not be ranked against a certified profile score. It can be displayed, but it belongs in a different lane.

---

## Public Badge Rules

Badges should be strict about model and verification level.

Acceptable examples:

```text
CLIARE experimental: 86
CLIARE CI: 86
CLIARE certified: 91
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

## Anti-Gaming Requirements

Any public score can be gamed. CLIARE should assume maintainers may optimize for the score.

Calibration and certification should include adversarial cases such as:

- help output that lists commands that do not exist
- help output that hides commands that do exist
- prose that looks like command rows
- numeric menus that look like subcommands
- fake JSON modes
- JSON mixed with warnings or progress text
- CLIs that write token-like files during help
- CLIs that behave differently when `CLIARE` appears in the environment
- CLIs that pass root `--help` but hang on subcommand help
- CLIs that expose huge low-value surfaces to exhaust budgets
- destructive commands hidden behind friendly names
- commands that exit zero for invalid flags
- commands that print stack traces instead of diagnostics

Certification should preserve these rules:

- runtime confirmation outweighs static help claims
- side effects are observed, not inferred from names alone
- safety prefers unknown over false safe
- budget pressure remains visible in scorecards
- public scorecards expose enough metadata for reproducibility

---

## Relationship To Other Documents

This document defines the public-authority bar. It is not the implementation plan for calibration commands.

Related documents:

- [Computational Scoring Model](../model/computational-scoring-model.md): current score formulas, claim confidence, model artifact, and future mathematical direction.
- [CI, Publishing, and Calibrated Leaderboards](ci-leaderboard-and-publishing.md): current CI outputs, repository publishing, and future hosted publishing surfaces.
- [QA, Benchmarking, and Calibration](qa-benchmarking-and-calibration.md): current benchmark runner and QA boundary.
- [Calibration Workflow TODO](calibration-workflow-todo.md): concrete future implementation plan for `calibrate init`, `calibrate check`, and `calibrate evaluate`.

---

## Certification Definition

CLIARE is ready for certified public scoring only when:

1. Maintainers can reproduce scores locally and in CI for the same binary, profile, model, and runtime context.
2. Scorecards explain every material score contribution.
3. Command indexes are useful to agent harnesses.
4. Calibration reports prove confidence values are meaningful.
5. Safety metrics prioritize avoiding false-safe classifications.
6. Public leaderboard entries are separated by profile, model, and verification level.
7. Model changes are governed, versioned, and reproducible.

Until then, CLIARE should remain explicit: v0 scores are evidence-backed engineering signals, not certified public rankings.
