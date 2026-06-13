# 12 - Reference CLI Behavior Guide

> **Scope:** Practical guidance for CLI maintainers who want to improve their CLIARE score and make their tools agent-ready.
> **Status:** Draft

---

## Summary

This guide describes what an agent-ready CLI looks like. It is not only a scoring guide. It should become the public reference for maintainers who ask:

> How do I make my CLI better for agents and automation?

The answer is not "add AI." The answer is to make the CLI discoverable, typed, deterministic, parseable, safe, and recoverable.

---

## Golden Path

An agent-ready CLI supports this workflow:

```sh
mycli --version
mycli --help
mycli completion bash
mycli project list --format json
mycli project show <id> --format json
mycli deploy --env staging --dry-run --format json
mycli deploy --env staging --yes --format json
```

It has:

- stable command tree
- accurate help
- shell completion
- machine-readable output
- clear exit codes
- dry-run for mutation
- noninteractive mode
- parseable errors
- no hidden side effects in metadata commands

---

## Command Discovery

Good:

```text
mycli --help lists top-level commands.
mycli help <command> works.
mycli <command> --help works.
mycli completion <shell> works.
Unknown commands suggest close matches.
```

Poor:

```text
Only docs list commands.
Subcommands are hidden unless guessed.
Help exits with stack trace.
Completion exists but is stale.
```

Recommendation:

- Make every public command reachable through help traversal.
- Keep completion and help generated from the same source when possible.
- Mark hidden/deprecated commands explicitly.

---

## Flag Grammar

Good:

```text
--format json
--format=json
-f json
```

Help:

```text
--format <FORMAT>  Output format [possible values: json, table, yaml]
```

Poor:

```text
--format is mentioned in prose but not usage.
--format sometimes takes a value and sometimes does not.
Invalid values produce "error".
```

Recommendation:

- Declare value names.
- List enum values.
- Use consistent long flags.
- Avoid context-sensitive arity.
- Reject unknown flags.
- Give precise missing-value errors.

---

## Positional Arguments

Good:

```text
Usage: mycli project show <PROJECT_ID>
```

Error:

```text
missing required argument <PROJECT_ID>
```

Poor:

```text
Usage: mycli project show [args]
```

Recommendation:

- Name every positional.
- Mark optional and variadic args clearly.
- Avoid ambiguous positional order.
- Prefer flags for optional resource selectors.

---

## Machine-Readable Output

Good:

```sh
mycli project list --format json
mycli project list --json
```

Output:

```json
[
  { "id": "p_123", "name": "Demo" }
]
```

Poor:

```text
Fetching...
ID      NAME
p_123   Demo
Done!
```

Recommendation:

- Add `--json` or `--format json`.
- Keep JSON on stdout clean.
- Send progress/warnings to stderr.
- Disable progress under `CI=1`.
- Honor `NO_COLOR=1`.
- Document output schema or keep it stable.

---

## Error Output

Good:

```text
error: invalid value 'jsn' for '--format'

valid values:
  json
  table
  yaml

did you mean 'json'?
```

Poor:

```text
bad format
```

Recommendation:

- Identify the bad input.
- Name the flag or argument.
- List valid values.
- Suggest likely correction.
- Use stable exit codes.
- Avoid stack traces for user errors.

---

## Exit Codes

Recommended:

| Code | Meaning |
|------|---------|
| 0 | success |
| 1 | general runtime failure |
| 2 | usage or validation error |
| 3 | auth error |
| 4 | not found |
| 5 | network or remote service error |

The exact mapping can vary, but it should be stable and documented.

---

## Safety

Mutating commands should be obvious.

Good:

```sh
mycli deploy --dry-run --format json
mycli delete project p_123 --dry-run
mycli delete project p_123 --yes
```

Poor:

```sh
mycli sync
# silently deletes remote resources
```

Recommendation:

- Provide dry-run/plan/check for mutating commands.
- Require explicit confirmation bypass for destructive commands.
- Do not make destructive defaults.
- Put side-effect warnings in help.
- Make dry-run side-effect free.

---

## Noninteractive Behavior

Good:

```text
CI=1 disables prompts.
--non-interactive fails instead of asking.
--yes explicitly bypasses confirmation.
```

Poor:

```text
Command waits forever for input.
Command opens editor.
Command opens browser.
Command starts pager.
```

Recommendation:

- Detect non-TTY and fail with actionable error.
- Support `--non-interactive`.
- Support env vars for CI mode.
- Avoid pagers, editors, and browser openers in CI.

---

## Config and Auth

Good:

```text
mycli auth status --format json
mycli config get --format json
mycli config path
```

Poor:

```text
Any command may create config files without disclosure.
Auth failures look like generic network failures.
```

Recommendation:

- Use XDG paths where possible.
- Document config writes.
- Make auth status machine-readable.
- Keep login interactive, but make auth errors actionable.
- Support scoped test credentials for CI if feasible.

---

## Completion

Good:

```sh
mycli completion bash
mycli completion zsh
mycli completion fish
```

Completion should include:

- commands
- flags
- enum values
- file/path hints where appropriate

Recommendation:

- Generate completion from the same command definitions as the parser.
- Keep completion deterministic.
- Avoid network calls in completion unless explicitly documented.

---

## Versioning and Drift

Good:

```sh
mycli --version
mycli version --format json
```

Output:

```json
{
  "version": "1.2.3",
  "commit": "abc123",
  "schema_version": "2026-06"
}
```

Recommendation:

- Version the CLI.
- Version output schemas when possible.
- Avoid breaking command shape in patch releases.
- Document deprecations before removal.

---

## CLIARE Improvement Checklist

High-impact fixes:

- [ ] Add `--help` and `--version`.
- [ ] Add help for every subcommand.
- [ ] Add shell completion.
- [ ] Reject unknown flags.
- [ ] Add suggestions for unknown commands/flags.
- [ ] List enum values in errors.
- [ ] Add `--json` or `--format json`.
- [ ] Keep JSON stdout clean.
- [ ] Add `--dry-run` for mutating commands.
- [ ] Require confirmation for destructive commands.
- [ ] Disable color/spinners/pagers in CI.
- [ ] Use stable exit codes.
- [ ] Make auth errors actionable.

---

## Example Before and After

Before:

```sh
mycli deploy prod
```

Problems:

- unclear positional
- mutating command
- no dry-run
- no output format
- ambiguous environment

After:

```sh
mycli deploy --env prod --dry-run --format json
mycli deploy --env prod --yes --format json
```

Improvements:

- explicit flag
- dry-run evidence
- machine-readable output
- confirmation bypass is explicit
- easier for agents to plan and verify

---

## What CLIARE Rewards

CLIARE rewards CLIs that are:

- discoverable
- explicit
- consistent
- parseable
- noninteractive
- safe by default
- helpful on error
- stable over time

It does not reward superficial documentation if runtime behavior disagrees.

---

## What CLIARE Penalizes

CLIARE penalizes:

- undocumented public commands
- stale completion
- help/runtime mismatch
- ambiguous flags
- human-only output
- prompts in CI
- destructive commands without dry-run
- noisy stdout
- hidden network calls
- unstable exit codes
- generic errors

---

## The Maintainer Contract

The ideal maintainer contract:

```text
Anything a human can discover through help or completion should work at runtime.
Anything an agent needs to parse should have machine-readable output.
Anything that mutates state should have a preview or dry-run path.
Anything that fails should fail with enough structure to recover.
```

That is agent-ready CLI design.

