# 21 - Reviewable Issue Ledger And Persona Views

> **Scope:** Current issue ledger, maintainer dispositions, and persona projections generated from CLIARE measurement artifacts.
> **Status:** Current implementation reference.

---

## Summary

CLIARE produces two related issue artifacts:

- `issues.json` and `issues.md`: the canonical issue ledger generated from measured artifacts.
- `issue-dispositions.json`: maintainer review state recorded with `cliare issues mark`.

Persona packets are views over the same ledger. This keeps maintainer, harness, platform, security, OSS, DevRel, and research reports consistent: every persona sees the same underlying evidence, but sorted and explained for that role.

The current artifact relationship is:

```text
evidence.jsonl
shape.json
command-index.json
scorecard.json
        |
        v
issues.json / issues.md
        |
        +--> persona-maintainer.*
        +--> persona-harness.*
        +--> persona-security.*
        +--> ...

issue-dispositions.json
        |
        v
issues list/report views show action-required vs reviewed decisions
```

The ledger exists because raw evidence IDs and score findings are not enough for day-to-day engineering work. A maintainer should be able to identify the issue, see affected commands, understand why it matters, decide whether to fix or disposition it, and verify the next run.

---

## Current Commands

List issues from a measurement artifact directory:

```sh
cliare issues list --out .cliare/mycli
cliare issues list --out .cliare/mycli --format human
cliare issues list --out .cliare/mycli --format markdown
cliare issues list --out .cliare/mycli --format json
```

Record a maintainer disposition:

```sh
cliare issues mark issue.output_mode_unprobed \
  --out .cliare/mycli \
  --status needs-fixture \
  --reason "Requires safe sample operands."
```

For context-suite roots, select the context:

```sh
cliare issues list --out .cliare/mycli --context workspace --format human
cliare issues mark issue.output_mode_unprobed --out .cliare/mycli --context workspace --status intentional --reason "Documented behavior."
```

`cliare issues list` reads both `issues.json` and `issue-dispositions.json` when present. It does not rerun the target CLI.

---

## Ledger Contract

`issues.json` uses:

```text
schema_version: cliare.issue-ledger.v1
```

The top-level ledger contains:

```text
schema_version
target
source_artifacts
summary
issues
```

The summary contains:

- total issue count
- high, medium, and low counts
- affected command count
- fixture-required issue count
- precondition-blocked issue count
- dispositioned issue count
- action-required issue count
- reviewed decision count

Each issue contains:

- `id`
- `status`
- `severity`
- `category`
- `agent_readiness_area`
- `confidence`
- `title`
- `impact`
- `why_it_matters`
- `recommendation`
- `verification`
- `affected_commands`
- `evidence`
- optional `disposition`
- `personas`
- `score_dimensions`

`issues.md` is rendered from the same in-memory ledger and is optimized for human review.

---

## Issue Fields

### Identity

Issue IDs are stable enough for dispositions and review workflows. They are not evidence IDs.

Example:

```text
issue.output_mode_unprobed
issue.safety.safe_probe_side_effects
issue.discovery.precondition_blocked
```

### Severity

Current severities:

```text
high
medium
low
```

Severity is persona-independent. Persona reports may reprioritize the same issue differently, but they do not change the underlying severity.

### Category

Current categories:

```text
discovery
grammar
execution
output
safety
recovery
coverage
policy
publishing
calibration
```

### Agent-Readiness Area

`agent_readiness_area` is the user-facing issue area used by focused reports and `--area` filtering:

```text
output-contracts
preconditions
command-discovery
help-coverage
compatibility
diagnostics
execution
safety
coverage
policy
publishing
calibration
```

### Confidence

Issue confidence communicates how directly CLIARE observed the problem.

| Confidence | Meaning | Example |
| --- | --- | --- |
| `observed` | The target runtime produced the behavior directly. | Advertised JSON exited successfully but did not parse. |
| `blocked` | The runtime reported a precondition before validation could continue. | Clean HOME produced an auth-required diagnostic. |
| `inferred` | The issue is inferred from shape gaps or incomplete confirmation. | A command candidate appeared in help but was not confirmed. |
| `needs_fixture` | CLIARE avoided probing because safe operands were not available. | A command advertises JSON but requires `<id>` or `<file>`. |
| `advisory` | The behavior is not necessarily a failure, but affects ergonomics or public claims. | Optional `help <path>` compatibility is unavailable while direct help works. |

This distinction is important. A `needs_fixture` issue is not a parse failure. A `blocked` issue is not command absence. An `advisory` issue should not block release unless local policy says so.

---

## Affected Commands

Issues can carry affected command samples. Each sample can include:

- command path
- argv form
- runtime state
- confidence
- summary
- required positionals
- output contracts
- reason

The ledger groups fixable patterns. If many commands share the same issue, CLIARE emits one issue with affected command samples instead of repeated duplicate issues.

---

## Evidence References

Each issue may include evidence entries with:

- kind
- reference
- detail
- probe id
- probe intent
- scope
- argv
- process status
- interpretation
- side effects when relevant

The goal is to make the issue understandable without forcing the reader to manually search `evidence.jsonl`, while still preserving enough pointers for audit.

Raw evidence stays in `evidence.jsonl`. The issue ledger is an index and interpretation layer, not a raw transcript.

---

## Verification

Every issue includes a verification command and expected change.

Example:

```json
{
  "verification": {
    "command": "cliare measure rote --out .cliare/rote --profile deep --refresh",
    "expected_change": "The contract moves from unprobed to parse_success=true, blocked with a documented precondition, or explicitly fixture-required."
  }
}
```

The verification command is intentionally operational. It tells the maintainer how to check whether a fix or disposition changed the measured result.

---

## Dispositions

Maintainers can record decisions without deleting evidence.

Disposition file:

```text
issue-dispositions.json
schema_version: cliare.issue-dispositions.v1
```

Current disposition statuses:

| Status | Action Required? | Meaning |
| --- | --- | --- |
| `open` | yes | No decision yet. |
| `accepted` | yes | The issue is accepted and should be fixed. |
| `needs_fixture` | yes | The issue requires safe operands or fixture data before validation. |
| `intentional` | no | The behavior is deliberate and documented. |
| `not_applicable` | no | The issue does not apply to this CLI/context. |
| `false_positive` | no | The finding is not valid for this target. |
| `accepted_risk` | no | The team accepts the risk. |
| `deferred` | no | The team defers action deliberately. |

Example:

```sh
cliare issues mark issue.safety.safe_probe_side_effects \
  --out .cliare/mycli \
  --status accepted-risk \
  --reason "The write is a documented cache under isolated XDG cache."
```

Dispositions are applied when rendering `issues list` and persona reports. Reviewed or muted issues remain in the ledger but move out of the action-required queue.

---

## Issues List Views

`cliare issues list` produces a disposition-aware projection.

JSON schema:

```text
cliare.issue-list.v2
```

The projection includes:

- artifact directory
- disposition file path
- source `issues.json` path when present
- total issues
- dispositioned count
- action-required count
- reviewed-decision count
- issue rows with title, severity, category, readiness area, confidence, recommendation, command samples, verification, disposition, and action-required state

Use formats by audience:

| Format | Use |
| --- | --- |
| `human` | Fast maintainer triage in the terminal. |
| `markdown` | Pull requests, issue comments, and review notes. |
| `json` | Automation and custom dashboards. |

The `human` view separates action-required issues from reviewed or muted issues and prints disposition examples.

---

## Persona Projection

Persona packets are projections over the ledger.

An issue can be relevant to multiple personas. For example:

- output issues usually matter to maintainers, harness builders, platform teams, OSS, and DevRel
- safety issues usually matter to security, harness builders, and platform teams
- calibration issues usually matter to research
- publishing issues usually matter to OSS and DevRel

Persona reports sort by persona priority, severity, and issue ID. They include:

- `top_issues`: action-required issues for that persona
- `reviewed_issues`: dispositioned or muted issues for that persona
- `action_items`: flattened work items derived from issues

This keeps the full ledger canonical while giving each role a focused work queue.

---

## Unprobed Output Contracts

An unprobed output contract is not automatically a failure.

The ledger distinguishes:

| State | Meaning |
| --- | --- |
| `needs_fixture` | The command requires operands or external state that CLIARE did not synthesize. |
| `blocked` | The runtime reported auth, local-context, network, dependency, or fixture preconditions. |
| `coverage` | Traversal or probe budget ended before safe validation. |

For maintainers, `needs_fixture` is actionable: provide a safe fixture invocation, a dry-run/sample mode, or a read-only list/show command that exercises the same output contract.

For harness builders, `needs_fixture` means the command should not be exposed as confidently machine-readable until validated.

For scoring, unprobed contracts are not counted as parse failures.

---

## Current Quality Bar

The issue ledger should remain:

- deterministic for the same measurement artifacts and dispositions
- grounded in `evidence.jsonl`, `shape.json`, `command-index.json`, and `scorecard.json`
- grouped by fixable pattern rather than duplicated per command
- explicit about observed, blocked, inferred, fixture-required, and advisory states
- actionable through verification commands
- disposition-aware without deleting evidence
- suitable for both human review and automation

The ledger is the canonical review surface. Persona packets and `issues list` are projections over it.
