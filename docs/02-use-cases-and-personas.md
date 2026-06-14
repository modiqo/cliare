# 02 - Use Cases and Personas

> **Scope:** User groups, jobs-to-be-done, workflows, and concrete scenarios supported by CLIARE.
> **Status:** Reference Design

---

## Overview

CLI usability has become a cross-cutting engineering concern. It affects maintainers, agent harnesses, CI pipelines, internal platform teams, security reviewers, DevRel teams, and tool vendors.

CLIARE serves these personas through one evidence model and persona-specific report views:

1. CLI maintainers
2. Agent harness builders
3. Platform engineering teams
4. Security and governance teams
5. Open-source project maintainers
6. Developer relations and ecosystem teams
7. Benchmark and research users

---

## Persona 1: CLI Maintainer

### Situation

A maintainer owns a CLI used by developers and increasingly by agents. The CLI has grown organically. It may have:

- many subcommands
- inconsistent flag grammar
- old aliases
- hidden commands
- text-only output
- some commands with JSON output
- weak error messages
- interactive prompts
- networked and local operations mixed together
- plugin extensions

The maintainer wants to make the CLI more automation-friendly without rewriting it.

### Jobs

- Measure current CLI quality.
- Identify contradictions between help, completion, and runtime behavior.
- Track improvement over releases.
- Add a README badge.
- Prevent regressions in CI.
- Prioritize fixes that help agents most.

### CLIARE Workflow

```sh
cliare measure ./target/release/mycli
open .cliare/report.md
```

Then later:

```sh
cliare guard ./target/release/mycli --baseline .cliare/baseline.scorecard.json
```

### Desired Report

The maintainer wants the report to say:

```text
Score: 72.4

Top improvements:
1. Add --json or --format json to 14 high-importance list/show commands.
2. Add --dry-run to 5 mutating commands.
3. Fix 3 help/runtime contradictions.
4. Make unknown flag errors include suggestions.
5. Disable color and spinner output when CI=1 or NO_COLOR=1.
```

### What Success Looks Like

The maintainer ships a release and sees:

```text
Output Contract: 58 -> 78
Safety: 61 -> 83
Total: 72 -> 84
```

That is the product loop.

---

## Persona 2: Agent Harness Builder

### Situation

An agent platform needs to decide whether a CLI is safe and useful enough to expose to agents. The harness can run shell commands, but raw shell access is broad and risky.

The platform wants a structured catalog:

- commands
- flags
- arguments
- output shape
- safety class
- confidence
- examples
- known failure modes

### Jobs

- Build tool definitions from a black-box CLI.
- Decide which commands are safe to expose.
- Prefer high-confidence invocations.
- Detect when a CLI upgrade invalidates learned skills.
- Generate agent guidance from observed behavior.

### CLIARE Workflow

```sh
cliare infer ./vendor-cli --profile safe --out vendor.shape.json
cliare tools ./vendor.shape.json --min-confidence 0.85 --safe-only
```

### Desired Artifact

The harness wants a `shape.json` with enough structure to generate tool schemas:

```json
{
  "command_id": "vendor.projects.list",
  "argv": ["vendor", "projects", "list"],
  "safety": { "class": "read", "confidence": 0.94 },
  "output": { "kind": "json", "confidence": 0.87 },
  "parameters": {
    "type": "object",
    "properties": {
      "org": { "type": "string" },
      "format": { "type": "string", "enum": ["json", "table"] }
    }
  }
}
```

### What Success Looks Like

The harness uses CLIARE output to expose a restricted, typed, high-confidence subset of the CLI rather than arbitrary shell access.

---

## Persona 3: Platform Engineering Team

### Situation

A platform team owns internal CLIs used to deploy services, rotate secrets, update environments, inspect production state, and run data jobs.

Agents and automation increasingly call these CLIs. The team needs governance.

### Jobs

- Enforce minimum CLI quality before adoption.
- Catch regressions in release pipelines.
- Require `--dry-run` for mutating commands.
- Require machine-readable output for read commands.
- Track readiness across dozens of internal tools.

### CLIARE Workflow

```sh
cliare certify ./platformctl --policy platform-policy.yaml
```

Policy example:

```yaml
minimum_score: 80
minimum_subscores:
  safety: 90
  output: 75
rules:
  - mutating_commands_require_dry_run: true
  - read_commands_require_machine_output: true
  - no_color_when_no_color_set: true
  - no_interactive_prompt_in_ci: true
```

### What Success Looks Like

CLIARE becomes a quality gate:

```text
platformctl v2.4.1 failed CLIARE guard:
- delete environment lacks dry-run or confirmation evidence
- deploy emits spinner bytes with CI=1
- list services no longer accepts --format json
```

---

## Persona 4: Security and Governance Team

### Situation

Security teams need to understand what a CLI can do before approving automated use. Source code may not be available. Vendor docs may be incomplete. The binary may have auth side effects or network behavior.

### Jobs

- Classify commands by side effect.
- Detect filesystem writes.
- Detect network attempts.
- Identify commands that require credentials.
- Verify noninteractive safeguards.
- Produce audit evidence.

### CLIARE Workflow

```sh
cliare probe ./vendorctl --profile safe --trace fs,network,process
cliare report --security .cliare/evidence.jsonl
```

### Desired Report

Security wants findings like:

```text
Command: vendorctl login
Class: auth-mutating
Evidence:
- writes $HOME/.vendor/config.json
- attempts outbound TLS to auth.vendor.example
- prints browser login URL
Recommendation:
- exclude from agent-exposed command set unless auth fixture is explicitly configured
```

### What Success Looks Like

Security can approve a subset of commands based on observed evidence, not trust in documentation.

---

## Persona 5: Open-Source Project Maintainer

### Situation

An OSS maintainer wants their CLI to be considered agent-friendly. They want a badge and a public score, but they do not want to upload binaries or secrets.

### Jobs

- Add a GitHub Action.
- Publish a scorecard.
- Improve public credibility.
- Compare with similar tools.

### CLIARE Workflow

```yaml
name: CLIARE
on:
  pull_request:
  push:
    tags: ["v*"]

jobs:
  cliare:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: make build
      - uses: modiqo/cliare-action@v1
        with:
          binary: ./dist/mycli
          profile: certify
          publish: true
```

### What Success Looks Like

The project README shows:

```text
CLIARE 86 | Agent-ready CLI | CI-attested
```

---

## Persona 6: Developer Relations and Ecosystem Team

### Situation

A company ships a developer platform and wants its CLI to be visible as agent-ready. It wants a public scorecard, a leaderboard entry, and a benchmark narrative that is supported by evidence.

### Jobs

- Publish a strong score.
- Show historical improvement.
- Demonstrate readiness against competitors.
- Use CLIARE findings to prioritize DevEx work.

### What Success Looks Like

CLIARE becomes part of release communication:

```text
Our CLI is CLIARE-certified with a 91 agent-readiness score.
```

The standard creates a vocabulary companies can use publicly without inventing their own readiness claims.

---

## Persona 7: Benchmark and Research User

### Situation

A researcher wants to evaluate how well agents use CLIs or how CLIs expose machine-operable interfaces.

### Jobs

- Use a reproducible corpus.
- Compare agent performance to CLIARE score.
- Study which CLI properties predict task success.
- Re-score old evidence with new models.

### What Success Looks Like

CLIARE provides:

- benchmark fixtures
- ground-truth command shapes
- evidence logs
- scoring model versions
- evaluation harnesses

Researchers can cite CLIARE as a measurement substrate, not just a tool.

---

## Use Case Matrix

| Use Case | Primary User | Required CLIARE Feature |
|----------|--------------|--------------------------|
| First-time audit | CLI maintainer | `cliare measure`, report |
| Release gate | CI owner | baseline diff, fail thresholds |
| Agent tool generation | Agent harness | shape catalog, safe subset export |
| Security review | Security team | sandbox traces, side-effect classification |
| Public badge | OSS maintainer | scorecard, publish, verification level |
| Leaderboard | Developer relations | hosted scorecard registry |
| Research benchmark | Researchers | ground truth, calibration, replayable evidence |
| Regression analysis | Platform team | historical score trend |
| Vendor evaluation | Enterprise buyer | comparable public scorecards |

---

## Required Modes

CLIARE should support distinct modes because users have different risk tolerance.

### `measure`

Local measurement, no CI failure.

```sh
cliare measure ./mycli
```

### `guard`

Compare to a baseline and fail on regressions.

```sh
cliare guard ./mycli --baseline .cliare/baseline.scorecard.json
```

### `certify`

Strict, reproducible profile intended for badges and leaderboard.

```sh
cliare certify ./mycli
```

### `infer`

Generate only the command shape catalog.

```sh
cliare infer ./mycli --out shape.json
```

### `rescore`

Recompute score from old evidence under a new model version.

```sh
cliare rescore .cliare/evidence.jsonl --model cliare-score-v2
```

### `publish`

Submit a scorecard to a hosted registry.

```sh
cliare publish .cliare/scorecard.json
```

---

## Anti-Use Cases

CLIARE should explicitly avoid these traps:

### Arbitrary Shell Benchmark

CLIARE is about a target CLI, not arbitrary shell command success.

### Prompt Benchmark

CLIARE measures the CLI surface, not whether one specific LLM can solve a task.

### Documentation Beauty Contest

Good docs help, but runtime behavior is decisive.

### Unsafe Fuzzer

CLIARE should not randomly run destructive commands. It should use profiles, side-effect classification, fixtures, dry-run discovery, and explicit opt-in.

### Cloud Binary Execution By Default

Cloud execution of arbitrary binaries should not be the primary path. It is expensive, risky, and unnecessary for adoption.
