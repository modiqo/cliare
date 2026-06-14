# Agent Skill Installation

CLIARE emits artifacts that are meant to be read by people and agents. The `skills/` directory packages that review procedure as installable agent context so tools such as Claude, Codex, and Cursor can inspect artifact maps, scorecards, persona reports, issue ledgers, the command index, command shape, and runtime evidence consistently.

The goal is not to make the agent more verbose. The goal is to make the agent more disciplined:

- read `artifact-map.json` first when present, or generate it with `cliare describe <folder> --write`
- start with persona-specific tables
- separate severity from confidence
- avoid raw JSON dumps
- avoid speculative root-cause claims
- use `command-index.json` for command suitability, parameters, preconditions, and evidence pointers
- drill into `issues.json`, `shape.json`, and `evidence.jsonl` only when needed
- preserve the distinction between `observed`, `blocked`, `needs_fixture`, `inferred`, and `advisory`

## Commands

List available integrations:

```sh
cliare skills list
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

Preview writes without changing files:

```sh
cliare skills install --agent all --dry-run
```

Install into a project instead of user-level agent directories:

```sh
cliare skills install --agent all --scope project --project-dir .
```

## Installed Artifacts

| Agent | Installed artifacts | User-scope location |
|---|---|---|
| `claude` | Shared `cliare-artifact-review` skill plus `/cliare-<persona>` command wrappers | `~/.claude/skills`, `~/.claude/commands` |
| `codex` | Shared `cliare-artifact-review` skill | `~/.codex/skills` |
| `cursor` | CLIARE artifact-review rule | `~/.cursor/rules` |

Project-scope installation mirrors the same layout under `.claude`, `.codex`, and `.cursor` inside the selected project directory.

## Persona Command Behavior

Claude receives persona commands for:

- `/cliare-maintainer`
- `/cliare-harness`
- `/cliare-platform`
- `/cliare-security`
- `/cliare-oss`
- `/cliare-devrel`
- `/cliare-research`

Each command resolves a CLIARE artifact directory, refreshes the requested persona packet when necessary, starts with the persona table, and drills into a selected issue only after the user asks or names an issue. The command wrappers intentionally forbid exploratory scripts, Python snippets, directory-changing shell flows, and speculative claims from command names.

## Skill Source of Truth

The canonical skill package is:

```text
skills/cliare-artifact-review/SKILL.md
```

The measurement artifact guide generated as `AGENT_SKILL.md` is compiled from the same package. This keeps local artifact directories and installed agent skills aligned.

The Cursor integration uses:

```text
skills/cursor/cliare-artifact-review.mdc
```

These files should be reviewed like any other user-facing contract. They shape how agents explain CLIARE output and therefore directly affect whether reports are useful to maintainers, harness builders, platform teams, security reviewers, OSS maintainers, DevRel teams, and researchers.
