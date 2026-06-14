# 21 - Reviewable Issue Ledger and Persona Views

> **Scope:** Human-reviewable findings generated from measured CLIARE artifacts.
> **Status:** Product and Implementation Design

---

## Purpose

CLIARE helps teams improve a CLI, not merely describe it. A scorecard answers whether the measured release is more or less agent-ready than another release. An issue ledger answers what a maintainer, harness builder, security reviewer, or platform owner can do next.

The ledger is the canonical review artifact derived from one measurement run. Persona reports are views over that ledger. This keeps the reporting system consistent:

```text
artifact-map.*  -> directory navigation, required files, schemas, and job state
evidence.jsonl  -> raw runtime observations
shape.json      -> inferred command surface and gaps
command-index.* -> command-level suitability, parameters, preconditions, and evidence pointers
scorecard.json  -> numeric readiness model and top-level findings

issues.json     -> normalized issue ledger
issues.md       -> human review ledger

persona-*.json  -> persona-prioritized projection
persona-*.md    -> persona-prioritized human packet
```

The ledger exists because raw evidence IDs and grouped score findings are not sufficient for day-to-day work. A developer should be able to open a file, select an issue, inspect the affected commands, understand the source of the observation, and make a change that can be validated by rerunning CLIARE.

---

## Design Principles

### One Issue Per Fixable Pattern

The ledger should avoid duplicate rows for the same fix. If ten commands have the same failure pattern, the ledger should contain one issue with ten affected commands, not ten repeated action rows.

### Preserve the Evidence Trail

Each issue must retain references into `evidence.jsonl`, `command-index.json`, `shape.json`, or `scorecard.json`. Evidence references are not a user interface by themselves, but they make the issue auditable.

### Separate Observation From Judgment

The ledger should distinguish:

- an observed failure, such as invalid JSON from an advertised JSON mode
- an unmeasured contract, such as a JSON mode that requires positional fixtures
- a runtime precondition, such as clean-environment auth requirement
- a design recommendation, such as help precedence for `--json --help`

These states should not be collapsed into one vague "not working" category.

### Describe the Next Verification Run

Every issue should state how to verify improvement. A fix that cannot be remeasured is not operationally useful.

### Keep Persona Views Focused

Persona reports should not repeat the whole ledger. They should rank and explain the subset that matters to that persona, then link or refer to the complete ledger for the full issue inventory.

---

## Issue Ledger Contract

`issues.json` is a stable automation artifact. `issues.md` is a stable human artifact. Both are generated from the same in-memory ledger.

```json
{
  "schema_version": "cliare.issue-ledger.v1",
  "target": {
    "requested": "rote",
    "resolved": "/Users/example/.local/bin/rote",
    "binary_sha256": "..."
  },
  "source_artifacts": {
    "evidence": "evidence.jsonl",
    "shape": "shape.json",
    "scorecard": "scorecard.json"
  },
  "summary": {
    "issues_total": 12,
    "high": 2,
    "medium": 7,
    "low": 3,
    "affected_commands": 48,
    "requires_fixtures": 4,
    "blocked_by_preconditions": 3
  },
  "issues": []
}
```

Each issue uses this structure:

```json
{
  "id": "issue.output.unvalidated_json.adapter_set",
  "status": "open",
  "severity": "medium",
  "category": "output",
  "confidence": "observed",
  "title": "Advertised JSON output requires fixtures before validation",
  "impact": "Agents cannot rely on this output contract until it is validated with safe command operands.",
  "why_it_matters": "Harnesses need parseable machine output for command routing, state inspection, and recovery.",
  "recommendation": "Provide a safe fixture profile for the required operands or expose a read-only command variant that can be probed without mutation.",
  "verification": {
    "command": "cliare measure rote --out .cliare --profile deep --refresh",
    "expected_change": "The output contract moves from unprobed to parse_success=true or precondition_blocked=true with documented setup."
  },
  "affected_commands": [
    {
      "path": ["adapter", "set"],
      "argv": ["rote", "adapter", "set"],
      "state": "runtime_confirmed",
      "reason": "usage requires <ID> <KEY> <VALUE>; CLIARE did not synthesize operands"
    }
  ],
  "evidence": [
    {
      "kind": "shape",
      "reference": "shape.output_contracts[adapter set --json]",
      "detail": "advertised=true, probed=false"
    }
  ],
  "personas": ["maintainer", "harness", "platform"],
  "score_dimensions": ["output", "grammar"]
}
```

---

## Issue Confidence

Issue confidence should communicate how directly CLIARE observed the problem.

| Confidence | Meaning | Example |
|---|---|---|
| `observed` | The target runtime produced the behavior directly. | `--json` exited successfully but stdout did not parse as JSON. |
| `blocked` | The runtime reported a precondition before the contract could be validated. | Clean HOME produced `auth required` for a list command. |
| `inferred` | The issue is inferred from shape gaps and incomplete confirmation. | A command candidate appeared in help but no matching command-specific help was observed. |
| `needs_fixture` | The contract is plausible, but CLIARE avoided probing because safe operands were not available. | A mutation command advertises JSON but requires `<ID> <VALUE>`. |
| `advisory` | The behavior is not a failure, but it affects agent ergonomics or standard conformance. | `--help` takes precedence over `--json`, so machine-readable help is unavailable. |

This distinction prevents false positives from becoming developer noise.

---

## Issue Categories

### Discovery

Discovery issues concern command existence and help availability.

Examples:

- command candidate is unconfirmed
- command-specific help is missing
- help for a child path returns parent help
- clean-runtime preconditions block help

### Grammar

Grammar issues concern arguments and flags.

Examples:

- usage syntax does not identify required operands
- flag arity is unknown
- value domains are undocumented
- aliases are advertised but canonical help cannot be reconciled

### Output

Output issues concern machine-readable contracts.

Examples:

- advertised JSON/YAML does not parse
- advertised output mode was not probed because fixtures are required
- output contract is auth-gated
- `--help` precedence prevents machine-readable help

### Recovery

Recovery issues concern failure behavior.

Examples:

- unknown flags are accepted silently
- unknown subcommands fall through to parent behavior
- diagnostics omit suggestions or valid alternatives
- exit codes do not distinguish usage errors from runtime failures

### Safety

Safety issues concern side effects and sensitive paths during safe probes.

Examples:

- help writes persistent files
- version probes create caches
- command discovery touches credential-like paths
- safe probes mutate working directory state

### Coverage

Coverage issues concern the measurement itself.

Examples:

- traversal stopped before convergence
- depth budget was reached
- probe budget was exhausted
- output contracts require fixtures not supplied by the run

---

## Persona Projection

Every ledger issue can carry a persona relevance map. Persona reports sort by this relevance and then by severity.

| Persona | Primary Lens | Highest-Value Issues |
|---|---|---|
| `maintainer` | What should change in the CLI implementation? | output, discovery, grammar, recovery |
| `harness` | What can an agent safely and reliably use? | safety, output, blocked commands, recovery |
| `platform` | Can this pass policy in CI? | safety, coverage, score regression, guard policy |
| `security` | What runtime behavior needs approval? | side effects, credential-like paths, auth preconditions |
| `oss` | Is the public score credible? | traversal completeness, score status, publishable artifacts |
| `devrel` | What claims are supported? | public-safe summary, strengths, limitations |
| `research` | Is this useful calibration data? | reproducibility, labels, uncertainty, corpus metadata |

Persona files should include:

- a short purpose statement
- score and traversal summary
- the top issues for that persona
- a command review section when command-level action is needed
- exact rerun commands
- notes on interpretation boundaries

They should not include repeated issue rows or unexplained evidence IDs.

---

## Markdown Review Format

`issues.md` should be optimized for code-review style triage.

```markdown
# CLIARE Issue Ledger

Target: `rote`
Score: 76.5/100
Traversal: converged

## I-001 Runtime Preconditions Block Clean-Environment Discovery

Severity: medium
Category: discovery
Confidence: blocked
Affected commands: 7

Why it matters:
Clean CI and agent harnesses need to distinguish command existence from configured account state.

Affected command samples:
- `rote flow list`
- `rote vars`

Recommended fix:
Expose unauthenticated help and schema metadata where practical. If runtime auth is unavoidable, document the precondition in help without requiring the configured profile.

Verify:
`cliare measure rote --out .cliare --profile deep --refresh`
```

The Markdown should be readable without opening JSON. JSON remains the authoritative machine contract.

---

## Handling Unprobed Output Contracts

An unprobed output contract is not automatically a failure. It should become one of three issue types:

1. `needs_fixture`: the command requires operands or external state that CLIARE did not synthesize.
2. `blocked`: the runtime reported preconditions such as auth, local context, profile, dependency, or another runtime requirement.
3. `coverage`: traversal or probe budget ended before safe validation.

For maintainers, `needs_fixture` is actionable: provide a documented safe fixture, a dry-run command, or a read-only list/show command that exercises the same output contract.

For harness builders, `needs_fixture` means the command should not be exposed as confidently machine-readable until validated.

For leaderboard scoring, unprobed contracts should affect coverage confidence, not be counted as parse failures.

---

## Evidence References

Evidence references should be human-facing, not only raw IDs.

Preferred evidence record:

```json
{
  "kind": "process",
  "reference": "e_000832",
  "probe_id": "p_000416",
  "intent": "output_json",
  "argv": ["rote", "adapter", "list", "--json"],
  "status": "exited:0",
  "summary": "stdout parsed as JSON"
}
```

Raw evidence IDs remain present, but the issue should carry enough context to understand the observation without manually searching `evidence.jsonl`.

---

## Acceptance Criteria

The implementation is ready when:

1. `cliare report <persona> --write` writes `issues.json` and `issues.md` if they do not already exist for the artifact directory.
2. The JSON persona packet includes a `top_issues` view derived from the ledger.
3. The Markdown persona packet starts with a persona-specific priority table and uses drill-down sections for selected issues.
4. Unprobed output contracts are represented as `needs_fixture`, `blocked`, or `coverage`, never as parse failures.
5. Auth preconditions include affected command paths and evidence context.
6. Side-effect issues include sample paths and probe argv.
7. The ledger can be regenerated deterministically from `evidence.jsonl`, `command-index.json`, `shape.json`, and `scorecard.json`.
8. The artifact directory can be described independently with `cliare describe <folder> --write`, producing `artifact-map.json` and `artifact-map.md` for agents that need a stable navigation manifest before opening large artifacts.
9. Tests cover grouped gaps, side effects, preconditions, unprobed outputs, parse failures, and parent-help false positives.
