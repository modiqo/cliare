# 07 - Scoring And Improvement Tracking

> **Scope:** Current scorecard artifacts, findings, issue dispositions, guard comparisons, policy gates, CI outputs, and the practical improvement loop for CLI maintainers.
> **Status:** Current Implementation And Future Tracking Direction

---

## Summary

CLIARE turns runtime measurement into an evidence-backed improvement loop:

```text
measure -> inspect -> fix or disposition -> remeasure -> guard in CI
```

The score is the headline. The durable value is the path from runtime evidence to a maintainer decision:

- what CLIARE observed
- which command shape or safety issue it found
- which artifact proves it
- whether the maintainer fixed it, accepted it, or marked it as needing a fixture
- whether a later run regressed against a known baseline

CLIARE currently supports this loop with:

- `scorecard.json`
- `report.md`
- `issues.json` and `issues.md`
- `issue-dispositions.json`
- `summary.md`
- `findings.sarif`
- `junit.xml`
- `cliare guard`
- JSON policy checks

CLIARE does not currently ship separate `baseline`, `trend`, `certify`, or `publish` commands.

---

## Current Workflow

For a maintainer:

```sh
cliare measure ./mycli --out .cliare/mycli --profile standard --refresh
cliare issues list --out .cliare/mycli --format human
```

Then either fix the CLI or record an explicit disposition:

```sh
cliare issues mark finding.output.no_machine_readable_mode \
  --out .cliare/mycli \
  --status intentional \
  --reason "This CLI is intentionally human-only for this release."
```

Remeasure after changes:

```sh
cliare measure ./mycli --out .cliare/mycli --profile standard --refresh
```

Once the baseline is understood, preserve a scorecard and gate future runs:

```sh
mkdir -p .cliare-baseline/mycli
cp .cliare/mycli/scorecard.json .cliare-baseline/mycli/scorecard.json

cliare guard ./mycli \
  --baseline .cliare-baseline/mycli/scorecard.json \
  --out .cliare/mycli \
  --profile deep \
  --refresh
```

`cliare guard` performs a fresh measurement, compares the new total score against the supplied baseline scorecard, writes CI artifacts, and exits with failure when the regression exceeds `--allowed-drop` or when policy checks fail.

---

## Artifact Directory

The artifact directory is the `--out` directory passed to `cliare measure` or `cliare guard`.

Example:

```sh
cliare measure mise --out .cliare/mise --profile deep --refresh
```

Primary artifacts:

| Artifact | Purpose |
|---|---|
| `scorecard.json` | Numeric score, subscores, coverage counters, findings, runtime context, and model provenance. |
| `report.md` | Human-readable scorecard generated from the same scorecard data. |
| `issues.json` | Reviewable work queue with issue details, affected commands, evidence, and verification hints. |
| `issues.md` | Markdown issue ledger. |
| `issue-dispositions.json` | Maintainer decisions recorded by `cliare issues mark`. |
| `shape.json` | Evidence-derived command shape model. |
| `command-index.json` | Harness-oriented command map for agent routing. |
| `command-index.md` | Markdown rendering of the command index. |
| `evidence.jsonl` | Runtime evidence log. |
| `summary.md` | CI summary emitted by measurement and guard flows. |
| `findings.sarif` | SARIF rendering of scorecard findings. |
| `junit.xml` | JUnit rendering of findings and guard/policy failures. |
| `README.md` | Artifact navigation guide. |
| `AGENT_SKILL.md` | Agent-facing review skill for the artifact directory. |

When a runtime context is used, `--out` can represent a suite root and CLIARE resolves the selected context artifact directory through the context flags. Commands that read artifacts accept `--context` where applicable.

---

## Scorecard

The scorecard schema is:

```text
cliare.scorecard.v1
```

Current top-level fields:

- `schema_version`
- `target`
- `runtime_context`
- `score`
- `subscores`
- `coverage`
- `findings`
- `model`

The current score status is:

```text
experimental_partial
```

The scorecard total is a whole-point deterministic score from the bundled `cliare-score-v0` model. It does not currently include public confidence intervals, estimated unseen command counts, verification-level badges, or certified leaderboard metadata.

Current score summary shape:

```json
{
  "total": 84,
  "measured_weight": 1.0,
  "max_weight": 1.0,
  "model": "cliare-score-v0",
  "status": "experimental_partial"
}
```

Current dimension score shape:

```json
{
  "score": 84,
  "weight": 0.35,
  "status": "measured",
  "rationale": "confirmed command coverage plus average command confidence"
}
```

For exact score formulas, see [Computational Scoring Model](computational-scoring-model.md).

---

## Current Subscores

CLIARE currently emits six measured dimensions:

| Dimension | Current Meaning |
|---|---|
| Discovery | How much of the command surface was discovered, runtime-confirmed, or recognized through precondition-blocked probes. |
| Grammar | Whether confirmed commands have usable flag and usage grammar. |
| Execution | Whether probes completed without timeout or spawn failure. |
| Recovery | Whether invalid probes reject cleanly and precondition diagnostics are actionable. |
| Output | Whether JSON or YAML contracts were discovered and parse successfully when safely probed. |
| Safety | Whether safe probes left persistent filesystem side effects or credential-like paths. |

The scorecard is best read together with `coverage`. A score of `0` in a dimension may mean a real weakness, but it may also mean there was no measurable evidence for that dimension in the current context.

---

## Findings

Scorecard findings are compact explanations derived from coverage metrics and model thresholds.

Current finding shape:

```json
{
  "id": "finding.output.no_machine_readable_mode",
  "dimension": "output",
  "severity": "medium",
  "title": "No machine-readable output mode was discovered",
  "detail": "No JSON or YAML output contract was found in runtime help evidence.",
  "recommendation": "Advertise a stable JSON or YAML output mode in command help."
}
```

Current scorecard severities are:

- `low`
- `medium`
- `high`

Findings do not currently contain estimated score impact, Shapley attribution, command importance, or public verification tier metadata.

The richer maintainer work queue lives in `issues.json`.

---

## Issue Ledger And Dispositions

Use the issue commands for the maintainer feedback loop:

```sh
cliare issues list --out .cliare/mycli --format human
cliare issues list --out .cliare/mycli --format markdown
cliare issues list --out .cliare/mycli --format json
```

Record a maintainer decision:

```sh
cliare issues mark <issue-id> \
  --out .cliare/mycli \
  --status intentional \
  --reason "Documented and expected product behavior."
```

Current disposition statuses:

- `open`
- `accepted`
- `intentional`
- `not-applicable`
- `false-positive`
- `accepted-risk`
- `needs-fixture`
- `deferred`

The disposition file is:

```text
issue-dispositions.json
```

Dispositions are not hidden fixes. They are explicit review state that lets maintainers separate real defects from known, documented, or fixture-blocked behavior.

---

## Baselines

CLIARE does not currently provide a `cliare baseline accept` command. A baseline is a scorecard file that you preserve and pass to `cliare guard`.

Example:

```sh
mkdir -p .cliare-baseline/mycli
cp .cliare/mycli/scorecard.json .cliare-baseline/mycli/scorecard.json
```

The guard only requires that the baseline file contains:

```json
{
  "score": {
    "total": 84
  }
}
```

In practice, use the full `scorecard.json` as the baseline because it also preserves target fingerprint, model, context, subscores, coverage, and findings for human review.

Baseline hygiene:

- keep baselines per target CLI
- keep baselines per runtime context when contexts differ
- avoid comparing authenticated and unauthenticated runs as if they were the same context
- avoid comparing host-mode and isolated-mode safety as if they had identical side-effect coverage
- preserve the score model id and hash with the baseline

---

## Guard

Command:

```sh
cliare guard ./mycli \
  --baseline .cliare-baseline/mycli/scorecard.json \
  --out .cliare/mycli \
  --profile deep \
  --allowed-drop 0 \
  --refresh
```

`cliare guard`:

1. Reads the baseline scorecard.
2. Runs a fresh measurement with the same measurement options supported by `cliare measure`.
3. Computes:

```text
delta = current_total - baseline_total
```

4. Passes the score-regression gate when:

```text
delta + allowed_drop >= 0
```

5. Optionally evaluates a policy file.
6. Writes CI artifacts into the measurement artifact directory.

Terminal output includes:

- result
- score regression pass/fail
- baseline path
- baseline score
- current score
- delta
- allowed drop
- policy result and failures when a policy is supplied

---

## Policy Gates

`cliare guard --policy <file>` currently reads a JSON policy with schema:

```text
cliare.policy.v1
```

Example:

```json
{
  "schema_version": "cliare.policy.v1",
  "min_total_score": 80,
  "min_subscores": {
    "discovery": 75,
    "grammar": 70,
    "execution": 80,
    "output": 50,
    "safety": 90,
    "recovery": 70
  },
  "side_effects": {
    "allow_paths": [
      "xdg-cache/mycli/**"
    ],
    "max_unapproved": 0,
    "deny_credential_like": true
  }
}
```

Current policy checks:

| Rule | Behavior |
|---|---|
| `min_total_score` | Fails when `score.total` is below the threshold. |
| `min_subscores` | Fails when a named subscore is missing or below the threshold. |
| `side_effects.allow_paths` | Treats matching side-effect paths as approved. Supports `*` and `**` path globs. |
| `side_effects.max_unapproved` | Fails when more than this many unapproved side-effect paths are observed. |
| `side_effects.deny_credential_like` | Fails when side-effect paths look credential-related. |

The policy parser currently expects JSON, not YAML.

---

## CI Outputs

Both `cliare measure` and `cliare guard` write CI-oriented artifacts.

`summary.md` includes:

- target and resolved path
- score and status
- finding count
- discovered and runtime-confirmed command counts
- concurrency, scheduler rounds, and scheduled probes
- machine-readable output and parse-success counts
- side-effect and credential-like side-effect counts
- traversal completion
- guard result when guard was used
- policy result when policy was supplied
- subscore table
- finding table
- artifact list

`findings.sarif` represents scorecard findings as SARIF rules/results. Current SARIF severity mapping:

| Finding Severity | SARIF Level |
|---|---|
| `high` | `error` |
| `medium` | `warning` |
| `low` | `note` |

`junit.xml` represents:

- scorecard findings as failed test cases
- guard score-regression failure as a failed test case
- policy failures as failed test cases

These artifacts are intended for GitHub Actions, CI summaries, code scanning, and test-report surfaces.

---

## Improvement Semantics

A score increase is useful, but it should not hide remaining negative findings.

Read improvements by dimension:

- output improves when JSON/YAML contracts are discovered and parse successfully
- safety improves when safe probes stop writing persistent or credential-like files
- recovery improves when invalid commands/flags reject cleanly and preconditions are actionable
- grammar improves when flags and usage syntax become clearer
- discovery improves when command candidates are runtime-confirmed or explicitly precondition-blocked
- execution improves when timeouts and spawn failures disappear

Read regressions with context:

- a score drop may be a real product regression
- a score drop may be newly discovered risk from deeper probing
- a dimension can remain low because the current context lacks auth, fixtures, local project state, network, or runtime dependencies
- host mode and isolated mode are not equivalent for side-effect interpretation

Use `issues.json` and `evidence.jsonl` when deciding whether to fix, disposition, or rerun with a richer context.

---

## What Is Future Work

This document should not imply the following are shipped today:

- `cliare baseline accept`
- `cliare trend`
- `cliare certify`
- `cliare publish`
- confidence intervals in `scorecard.json`
- estimated total command surface in `scorecard.json`
- score-impact attribution per finding
- command-importance weighted score deltas
- Shapley-style change attribution
- hosted score publishing
- certified public leaderboard normalization

Those remain useful product directions. The current reliable loop is measurement artifacts, issue dispositions, scorecard baselines, guard comparisons, JSON policy checks, and CI outputs.

---

## Public Badge Semantics

Before calibration, badges should describe provenance instead of claiming universal safety:

Good:

```text
Evidence-backed command index
```

Good when CI generated the artifacts:

```text
Evidence-backed command index, CI-measured
```

Acceptable with explicit status:

```text
CLIARE score 84 | experimental_partial | cliare-score-v0
```

Avoid:

```text
Agent-safe
```

The current model is useful for maintainers and harnesses, but it is still `experimental_partial`.
