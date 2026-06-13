# 07 - Scoring and Improvement Tracking

> **Scope:** Scorecard structure, report findings, baselines, regressions, improvement tracking, score deltas, and policy gates.
> **Status:** Draft

---

## Summary

CLIARE should not only produce a score. It should create an improvement loop.

The core workflow:

```text
measure -> report -> improve -> guard -> certify -> publish
```

A maintainer should be able to answer:

- What is our current score?
- Why is it that score?
- What changed since last release?
- Which fixes improve it most?
- Did this PR regress agent readiness?
- Are we better than last quarter?

---

## Scorecard Artifact

File:

```
.cliare/scorecard.json
```

Example:

```json
{
  "schema_version": "cliare.scorecard.v1",
  "target": {
    "name": "mycli",
    "version": "1.2.3",
    "binary_sha256": "..."
  },
  "score": {
    "total": 84.2,
    "interval": { "p05": 80.1, "p95": 87.6 },
    "model": "cliare-score-v1"
  },
  "subscores": {
    "discovery": 91.0,
    "grammar": 86.5,
    "execution": 81.2,
    "output": 76.4,
    "safety": 88.1,
    "recovery": 82.0
  },
  "coverage": {
    "commands_observed": 128,
    "commands_estimated": 142,
    "coverage_estimate": 0.90
  },
  "findings": [],
  "verification": {
    "level": "ci_attested",
    "provider": "github_actions"
  }
}
```

---

## Subscores

The current implementation emits an experimental partial scorecard. It computes a deterministic total over dimensions that currently have direct evidence and keeps the model status explicit as `experimental_partial`.

Score v0 measures:

- discovery from command confidence and runtime confirmation rate
- grammar from flag confidence and unresolved grammar gaps
- execution from completed probes, timeouts, and spawn failures
- recovery from invalid command, invalid child, and invalid flag rejection behavior
- output from advertised machine-readable modes and safe parse probes
- safety from persistent filesystem side effects observed during safe probes

Score v0 does not yet include the full certified model: calibrated confidence intervals, destructive-action classification, workload-specific task weighting, public truth-set calibration, or profile-normalized leaderboard scoring.

For the exact implemented formulas and Bayesian confidence layer, see [Scoring Model and Bayesian Confidence](17-scoring-model-and-bayesian-confidence.md).

### Discovery

Measures whether agents can find the CLI surface.

Signals:

- root help lists commands
- subcommand help is reachable
- completion exposes commands and flags
- invalid command errors suggest alternatives
- hidden commands are intentionally hidden
- estimated unseen surface is low

Improves when:

- commands become discoverable
- completion is added
- help traversal becomes consistent
- command aliases are documented

Regresses when:

- commands disappear from help
- completion breaks
- help exits nonzero unexpectedly
- command tree becomes inconsistent

### Grammar

Measures whether agents can construct valid invocations.

Signals:

- flags are known
- arity is known
- positionals are known
- enum values are known
- duplicate/repeat behavior is clear
- `--flag=value` vs `--flag value` behavior is clear

Improves when:

- usage syntax becomes precise
- invalid value errors list valid values
- missing value errors identify the flag
- flags consistently accept standard forms

Regresses when:

- help advertises unsupported flags
- required args are not documented
- arity is ambiguous
- parser behavior changes without help change

### Execution

Measures whether valid-looking commands run reliably.

Signals:

- valid probes succeed
- invalid probes fail cleanly
- exit codes are stable
- commands do not hang
- noninteractive mode works
- required state is discoverable

Improves when:

- commands fail early with usage errors
- hidden required config is explained
- timeouts disappear
- CI behavior stabilizes

Regresses when:

- valid probes start failing
- commands hang waiting for input
- exit codes become inconsistent
- commands require undisclosed state

### Output

Measures whether agents can parse results.

Signals:

- JSON/NDJSON/YAML/CSV output
- stable output schema
- clean stdout/stderr separation
- no colors/spinners in machine mode
- output format flags are documented and validated

Improves when:

- `--json` or `--format json` is added
- errors become structured
- progress moves to stderr
- non-TTY output becomes clean

Regresses when:

- JSON output becomes mixed with prose
- fields disappear without schema/versioning
- stdout includes warnings
- pager/color appears in CI

### Safety

Measures whether agents can avoid unwanted side effects.

Signals:

- mutating commands are identifiable
- dry-run/plan/check exists
- destructive commands require confirmation
- side effects are contained
- network behavior is explicit
- auth behavior is clear

Improves when:

- dry-run is added
- destructive verbs require explicit `--yes`
- config writes move to XDG paths
- help states side effects

Regresses when:

- mutating command lacks dry-run
- help command writes config
- command calls network unexpectedly
- destructive command accepts defaults silently

### Recovery

Measures whether agents can fix mistakes.

Signals:

- unknown command suggestions
- unknown flag suggestions
- valid enum values listed
- auth errors explain remediation
- missing required args are named
- error output is parseable

Improves when:

- errors include "did you mean"
- errors list valid values
- auth errors include exact command to login
- parse errors avoid stack traces

Regresses when:

- errors become generic
- stack traces leak to users
- valid values disappear
- auth failures look like network failures

---

## Findings

Findings are ranked, actionable explanations.

Finding shape:

```json
{
  "id": "finding.output.no_json.high_importance",
  "severity": "medium",
  "dimension": "output",
  "title": "High-importance read commands lack machine-readable output",
  "impact": -4.8,
  "commands": ["mycli project list", "mycli project show"],
  "evidence": ["e_00012", "e_00018"],
  "recommendation": "Add --json or --format json and keep stdout parseable when CI=1."
}
```

Severity:

- `info`
- `low`
- `medium`
- `high`
- `critical`

Impact is estimated score impact.

---

## Baselines

Baseline file:

```
.cliare/baseline.scorecard.json
```

Create:

```sh
cliare baseline accept .cliare/scorecard.json
```

Guard:

```sh
cliare guard ./mycli --baseline .cliare/baseline.scorecard.json
```

Baseline should include:

- target version
- binary fingerprint
- score model
- shape hash
- evidence hash if available
- accepted date
- accepted by

---

## Regression Types

CLIARE should distinguish:

### Breaking Shape Regression

Public command or flag changed incompatibly.

Examples:

- command removed
- flag removed
- flag arity changed
- positional requiredness changed
- enum value removed

### Runtime Regression

Shape still appears valid, but execution fails.

Examples:

- valid probe now exits nonzero
- command hangs
- exit code changed

### Output Regression

Machine-readable contract worsened.

Examples:

- JSON output removed
- stdout polluted
- schema stability drops

### Safety Regression

Risk increased or mitigation disappeared.

Examples:

- dry-run removed
- destructive command no longer requires confirmation
- new write command undisclosed

### Discovery Regression

Surface became harder to find.

Examples:

- completion missing commands
- help traversal broken
- unknown command suggestions removed

### Confidence Regression

Behavior may not have changed, but evidence became less certain.

Examples:

- probes timeout intermittently
- output nondeterministic
- completion inconsistent

---

## Improvement Tracking

Track historical scores:

```
.cliare/history/
  2026-06-13T120000Z.scorecard.json
  2026-06-20T120000Z.scorecard.json
```

Trend report:

```sh
cliare trend .cliare/history
```

Output:

```text
Total:    71.2 -> 84.0 (+12.8)
Output:   43.0 -> 76.5 (+33.5)
Safety:   68.2 -> 86.1 (+17.9)
Recovery: 79.0 -> 81.3 (+2.3)
```

The report should list the concrete changes behind the trend.

---

## Policy Gates

Policy file:

```yaml
minimum_score: 80
minimum_subscores:
  discovery: 75
  grammar: 75
  execution: 75
  output: 70
  safety: 85
  recovery: 70

fail_on:
  public_command_removed: true
  flag_arity_changed: true
  json_output_removed: true
  safety_score_drop_gt: 5
  new_destructive_command_without_dry_run: true

warn_on:
  confidence_interval_width_gt: 15
  hidden_command_discovered: true
```

Command:

```sh
cliare guard ./mycli --policy cliare.yaml
```

---

## Score Delta Semantics

Score delta should be clear:

```text
Total: 78.4 -> 82.9 (+4.5)

Positive:
+3.1 Output: added JSON output to 6 list/show commands
+1.8 Safety: deploy supports --dry-run
+0.9 Recovery: invalid enum errors now list valid values

Negative:
-0.8 Grammar: new export command has unknown variadic positional
-0.5 Discovery: completion missing new config subcommands
```

Do not hide negatives under a positive total.

---

## Improvement Recommendations

Recommendation ranking should optimize expected score gain per effort.

Inputs:

- affected command importance
- current confidence
- score impact
- likely implementation difficulty
- common fix pattern

Example recommendations:

| Recommendation | Expected Gain | Effort |
|----------------|---------------|--------|
| Add `--json` to read commands | +8.2 | Medium |
| Add `--dry-run` to deploy/delete | +5.7 | Medium |
| Fix help/runtime flag mismatch | +3.1 | Low |
| Add unknown flag suggestions | +2.4 | Medium |
| Disable color under `NO_COLOR` | +1.9 | Low |

---

## Public Badge Semantics

Badge should show:

```text
CLIARE 84
```

Optional qualifiers:

```text
CLIARE 84 | CI-attested
CLIARE 84 | certified profile
CLIARE 84 | score model v1
```

Avoid overclaiming:

Bad:

```text
Agent-safe
```

Better:

```text
Agent-ready CLI
```

Even better with verification:

```text
Agent-ready CLI, CI-attested
```

---

## MVP Scoring Implementation

MVP should include:

- total score
- six subscores
- confidence interval
- top findings
- baseline diff
- Markdown report
- JSON scorecard
- guard thresholds

MVP can skip:

- Shapley attribution
- advanced trend dashboards
- hosted score publishing
- workload learning

But the scorecard schema should leave room for them.
