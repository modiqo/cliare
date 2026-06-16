# 23 - Agent Skill Installation

> **Scope:** Current installable CLIARE artifact-review skills for agent tools.
> **Status:** Current implementation reference.

---

## Summary

CLIARE emits artifacts for humans and agents: artifact maps, scorecards, persona reports, issue ledgers, command indexes, command shape, and runtime evidence.

The `cliare skills` command installs review instructions so supported agent tools inspect those artifacts consistently. The installed skills do not run measurement. They teach the agent how to read an existing CLIARE artifact directory without overclaiming from partial evidence.

The review discipline is:

- read `artifact-map.json` first when present, or generate it with `cliare describe <artifact-dir> --write`
- start with persona-specific Markdown/JSON tables
- separate severity from confidence
- avoid raw JSON dumps
- avoid speculative root-cause claims
- use `command-index.json` for command suitability, parameters, preconditions, output contracts, and evidence pointers
- drill into `issues.json`, `shape.json`, and `evidence.jsonl` only when needed
- preserve the distinction between `observed`, `blocked`, `needs_fixture`, `inferred`, and `advisory`

---

## Commands

List available integrations:

```sh
cliare skills list
cliare skills list --format json
```

Install all user-level integrations:

```sh
cliare skills install --agent all
```

Install one integration:

```sh
cliare skills install --agent claude
cliare skills install --agent codex
cliare skills install --agent cursor
```

Preview planned writes without changing files:

```sh
cliare skills install --agent all --dry-run
```

Install into project-level agent directories:

```sh
cliare skills install --agent all --scope project --project-dir .
```

Override user home for user-scope installation:

```sh
cliare skills install --agent codex --scope user --home /tmp/cliare-home
```

---

## Supported Agents

Current install targets:

| Agent | Installed Artifacts | User-Scope Location |
| --- | --- | --- |
| `claude` | Shared `cliare-artifact-review` skill plus `/cliare-<persona>` command wrappers | `~/.claude/skills`, `~/.claude/commands` |
| `codex` | Shared `cliare-artifact-review` skill | `~/.codex/skills` |
| `cursor` | CLIARE artifact-review rule | `~/.cursor/rules` |

Project-scope installation mirrors the same roots under the selected project directory:

```text
<project>/.claude/skills/...
<project>/.claude/commands/...
<project>/.codex/skills/...
<project>/.cursor/rules/...
```

`--agent all` expands to `claude`, `codex`, and `cursor`.

---

## Installed Files

Claude user-scope install writes:

```text
~/.claude/skills/cliare-artifact-review/SKILL.md
~/.claude/commands/cliare-maintainer.md
~/.claude/commands/cliare-harness.md
~/.claude/commands/cliare-platform.md
~/.claude/commands/cliare-security.md
~/.claude/commands/cliare-oss.md
~/.claude/commands/cliare-devrel.md
~/.claude/commands/cliare-research.md
```

Codex user-scope install writes:

```text
~/.codex/skills/cliare-artifact-review/SKILL.md
```

Cursor user-scope install writes:

```text
~/.cursor/rules/cliare-artifact-review.mdc
```

Install actions are reported as:

- `created`
- `updated`
- `unchanged`
- `would write` when `--dry-run` is used

---

## Claude Persona Commands

Claude receives command wrappers for:

- `/cliare-maintainer`
- `/cliare-harness`
- `/cliare-platform`
- `/cliare-security`
- `/cliare-oss`
- `/cliare-devrel`
- `/cliare-research`

Each wrapper instructs the agent to:

1. Resolve the CLIARE artifact directory from the command arguments or local `.cliare` directories.
2. Use the `cliare-artifact-review` skill.
3. Prefer `persona-<persona>.md`, `persona-<persona>.json`, and `issues.json`.
4. Start with the persona table.
5. Ask which row to drill into unless the user already named an issue.
6. Use `command-index.json`, `shape.json`, and `evidence.jsonl` only for drill-down.
7. Avoid exploratory scripts and speculative claims.

If no artifact directory exists, the wrapper tells the agent to ask for a measurement run instead of inventing findings.

---

## Source Of Truth

The shared skill source is:

```text
skills/cliare-artifact-review/SKILL.md
```

The Cursor rule source is:

```text
skills/cursor/cliare-artifact-review.mdc
```

Measurement artifact directories can also contain:

```text
AGENT_SKILL.md
```

For measurement directories, `AGENT_SKILL.md` is generated from `skills/cliare-artifact-review/SKILL.md`. This keeps local artifact guidance and installed agent skills aligned.

Benchmark directories use a benchmark-specific generated `AGENT_SKILL.md` because the review workflow is corpus-level rather than single-measurement-level.

---

## What The Skill Does Not Do

The installed skill does not:

- run `cliare measure`
- certify a score
- decide whether a CLI is safe by itself
- infer root causes from command names
- replace `issues.json`, `command-index.json`, or `evidence.jsonl`

It is a review guide. The measured artifacts remain the source of truth.

---

## Review Contract

These files should be reviewed like any other user-facing contract. They shape how agents explain CLIARE output and therefore directly affect whether reports are useful to maintainers, harness builders, platform teams, security reviewers, OSS maintainers, DevRel teams, and researchers.

When updating the skill, verify that it still:

- starts from artifact maps and persona reports
- treats `issues.json` as the canonical issue queue
- uses `command-index.json` for command routing decisions
- distinguishes observed, blocked, fixture-required, inferred, and advisory findings
- avoids raw JSON dumps unless the user asks for machine-readable detail
- avoids exploratory scripts for routine artifact inspection
