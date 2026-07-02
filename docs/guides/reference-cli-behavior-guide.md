# 12 - Reference CLI Behavior Guide

> **Scope:** Practical behavior guidance for CLI maintainers who want better CLIARE results and more reliable agent/harness usage.
> **Status:** Current maintainer guide.

---

## Summary

An agent-ready CLI is not a CLI with AI-specific labels. It is a CLI whose released binary is discoverable, explicit, parseable, safe to inspect, and recoverable when a command fails.

CLIARE currently measures that behavior by running bounded runtime probes and producing:

- `evidence.jsonl`
- `shape.json`
- `command-index.json`
- `command-index.md`
- `scorecard.json`
- `issues.json`
- persona reports

The maintainer goal is simple:

> Make the CLI easy for an agent to discover, invoke, parse, and recover from without guessing.

Agents can attempt to explore almost any command-line interface. That does not
make every interface design acceptable for automation. The useful question is
whether the released CLI gives agents evidence they can rely on: command
existence, invocation grammar, parseable outputs, recovery diagnostics,
preconditions, and safe discovery behavior. CLIARE reports those evidence-backed
navigation capabilities and turns missing evidence into developer feedback.

---

## How CLIARE Sees A CLI

CLIARE treats the target CLI as a black box. It does not require a specific parser, framework, language, or source-code layout.

Current measurement primarily observes:

- root help and version behavior
- command-specific `--help` and `-h`
- optional `help <command path>` compatibility
- unknown command and unknown flag diagnostics
- usage strings, arguments, flags, aliases, and enum-like values in help
- advertised JSON/YAML/table/plain output modes
- safe output-mode probes when operands are not required
- fixture-needed situations when required operands are missing
- exit codes, stdout, stderr, timeouts, and spawn failures
- persistent filesystem side effects from safe probes
- runtime context such as clean, authenticated, local-context, or fixture runs

Current CLIARE does not use shell completion as its primary command discovery source. Completion is still useful CLI design, but the current command index is built from runtime help, diagnostics, output probes, and evidence.

## Agent Navigation Evidence

CLIARE reports agent navigation evidence as separate scorecard dimensions so a
maintainer can see which capabilities are backed by runtime proof:

| Capability | Developer Question |
|---|---|
| `canonical_help_coverage` | Can an agent ask each discovered command for direct `<command> --help`? |
| `usage_coverage` | Does help expose stable usage syntax for invocation planning? |
| `subcommand_table_clarity` | Can command groups be traversed from parseable command tables? |
| `positional_operand_coverage` | Do runtime-recognized commands reveal required operands? |
| `output_contract_parse_coverage` | Are advertised machine-readable outputs actually parseable? |
| `invalid_input_recovery` | Do mistakes fail nonzero and tell the agent what to do next? |
| `discovery_side_effect_safety` | Can discovery probes run without persistent side effects? |
| `precondition_clarity` | Are auth, fixture, local context, network, and dependency blockers explicit? |
| `example_validity` | Not measured yet; examples are not scored until CLIARE can validate them safely. |

Treat these as evidence labels, not style preferences. A CLI can be usable by a
human and still give an agent weak navigation evidence if command-specific help
is missing, usage is vague, output contracts are prose-only, or diagnostics
require human context.

---

## Golden Path

A strong CLI supports a workflow like this:

```sh
mycli --version
mycli --help
mycli project list --format json
mycli project show <project-id> --format json
mycli deploy --env staging --dry-run --format json
mycli deploy --env staging --yes --format json
```

It provides:

- stable root help and command-specific help
- visible usage grammar
- named positionals and flag value names
- clean machine-readable output
- explicit runtime preconditions
- noninteractive behavior for CI and agents
- dry-run or plan modes for mutation
- clear diagnostics and exit codes
- safe discovery paths with no hidden durable side effects

---

## Help Contract

Help is the main discovery API for CLIARE and for agent harnesses.

Canonical help forms:

```sh
mycli --help
mycli -h
mycli project list --help
mycli project list -h
```

Optional compatibility form:

```sh
mycli help project list
```

Current CLIARE treats direct `<command> --help` and `<command> -h` as canonical. If `help <command path>` fails but direct help succeeds, that should be treated as lower-severity compatibility feedback, not as proof that command help is unavailable.

Good help:

```text
mycli project list - List projects

Usage:
  mycli project list [OPTIONS]

Options:
  --format <FORMAT>  Output format [possible values: table, json, yaml]
  --limit <N>        Maximum projects to return [default: 100]
  --profile <NAME>   Profile to use
  -h, --help         Print help

Preconditions:
  Runtime data requires authentication.
  Help does not require authentication.
```

Poor help:

```text
Usage: mycli project [args]
```

Recommendations:

- Make root help available without auth, network, project checkout, or config writes.
- Make every public command reachable through command-specific help.
- Show the full command path in usage.
- Keep command tables under `Commands` or `Subcommands`.
- Keep fields, enum values, and environment variables out of command tables.
- Document aliases without hiding the canonical command.
- Mark deprecated or hidden commands explicitly.

---

## Command Discovery

Good:

```text
mycli --help lists top-level commands.
mycli project --help lists child commands.
mycli project list --help prints command-specific usage.
Unknown commands exit nonzero and suggest close matches.
```

Poor:

```text
Only external docs list commands.
Subcommands are discoverable only by guessing.
Unknown child commands print parent help and exit 0.
Help exits with a stack trace.
```

Recommendations:

- Generate help from the same command definitions used by the parser.
- Reject unknown commands with a nonzero exit code.
- Include a short correction when the match is obvious.
- Avoid interactive menus as the only path to subcommands.
- Do not require login to discover command shape.

---

## Flag Grammar

Good:

```text
--format <FORMAT>  Output format [possible values: table, json, yaml]
--limit <N>        Maximum number of results
--yes              Bypass confirmation prompts
--dry-run          Show planned changes without applying them
```

Poor:

```text
--format is mentioned only in prose.
--format sometimes takes a value and sometimes acts as a boolean.
Invalid values print "error".
Unknown flags are silently ignored.
```

Recommendations:

- Prefer stable long flags for scripts and agents.
- Use short flags only as aliases for common human workflows.
- Declare value names such as `<FORMAT>`, `<FILE>`, `<ID>`, and `<N>`.
- List enum values in help and invalid-value errors.
- Reject unknown flags.
- Return nonzero for missing flag values.
- Avoid context-sensitive arity.
- Support `--` when positionals can begin with `-`.

---

## Positional Arguments

Good:

```text
Usage:
  mycli project show <PROJECT_ID>
```

Good error:

```text
error: missing required argument <PROJECT_ID>

Usage:
  mycli project show <PROJECT_ID>
```

Poor:

```text
Usage:
  mycli project show [args]
```

Recommendations:

- Name every positional.
- Mark required and optional operands clearly.
- Mark variadic operands clearly.
- Prefer flags for optional resource selectors.
- Avoid overloaded positional order.
- Provide examples for commands with multiple required operands.

---

## Machine-Readable Output

Agents need machine-readable output to update state without parsing prose.

Good:

```sh
mycli project list --format json
mycli project list --json
```

Output:

```json
{
  "projects": [
    { "id": "p_123", "name": "Demo" }
  ],
  "next_cursor": null
}
```

Poor:

```text
Fetching projects...
{"projects":[{"id":"p_123","name":"Demo"}]}
Done!
```

Recommendations:

- Support `--format json` or `--json` for list/show/read commands.
- Keep machine-readable stdout clean.
- Send warnings, progress, and diagnostics to stderr.
- Disable progress under `CI=1` or when stdout is not a terminal.
- Honor `NO_COLOR=1` for human output.
- Keep top-level JSON shape stable across releases.
- Include schema versions or documented examples when practical.

CLIARE can safely validate output modes when the command can run without required operands. If required operands are needed, CLIARE will avoid guessing and may report `needs_fixture`.

---

## Fixtures For Required Operands

Some commands cannot be safely validated without sample IDs, URLs, paths, or account state. That is not automatically a CLI defect. It means the maintainer should provide a safe fixture path or disposition the issue.

Example problem:

```text
mycli adapter new-from-mcp <id> <endpoint-url> --dry-run --format json
```

CLIARE should not invent `<id>` or `<endpoint-url>`.

Mitigation options:

- document safe fixture operands in examples
- provide a `fixtures` command that prints safe IDs or endpoints
- make `--dry-run` accept obviously fake operands without writes
- provide a read-only sample mode
- record a `needs_fixture` disposition until a safe fixture exists

Example:

```sh
mycli fixtures print --format json
mycli adapter new-from-mcp fixture-adapter https://example.invalid/mcp --dry-run --format json
```

Disposition example:

```sh
cliare issues mark <issue-id> \
  --out .cliare/mycli \
  --status needs-fixture \
  --reason "Requires safe fixture operands for <id> and <endpoint-url>."
```

---

## Runtime Preconditions

Preconditions should be explicit and separable from command existence.

Common preconditions:

- authentication
- local project or repository context
- daemon running
- network access
- runtime dependency installed
- fixture data available

Good:

```text
auth required

Fix:
  mycli auth login

Machine output:
  mycli auth status --format json
```

Poor:

```text
failed
```

Recommendations:

- Help and version should not require auth.
- Command shape should be visible without satisfying runtime state.
- Auth errors should be distinguishable from not-found, network, and usage errors.
- Put actionable remediation on stderr.
- Avoid using the same exit code and message for every failure.
- Provide machine-readable status commands for auth/config where practical.

---

## Safety And Side Effects

CLIARE runs safe discovery probes. Those probes should not mutate durable user or project state.

Safe discovery paths include:

```text
--help
-h
help
--version
version
invalid command
invalid flag
safe output-mode help probes
```

Avoid during safe discovery:

- creating config files
- creating telemetry files
- writing credentials
- acquiring durable locks
- modifying project files
- opening editors, browsers, pagers, or interactive prompts
- deleting or syncing remote resources

If a harmless cache must be created, document it and keep it away from credential-like paths.

Mutating commands should provide:

```sh
mycli deploy --env staging --dry-run --format json
mycli deploy --env staging --yes --format json
```

Recommendations:

- Provide `--dry-run`, `--plan`, or `check` for mutation.
- Make dry-run side-effect free.
- Require explicit confirmation bypass for destructive commands.
- Put side-effect warnings in command help.
- Never make destructive behavior the default of an ambiguous command.

---

## Noninteractive Behavior

Agents and CI cannot safely handle hidden prompts.

Good:

```text
CI=1 disables prompts.
--non-interactive fails instead of prompting.
--yes explicitly bypasses confirmation.
```

Poor:

```text
Command waits forever for input.
Command opens an editor.
Command opens a browser.
Command starts a pager.
```

Recommendations:

- Detect non-TTY stdin and fail with an actionable error.
- Support `--non-interactive` where prompts are common.
- Support `--yes` or equivalent only for explicit confirmation bypass.
- Disable pagers, browsers, and editors in CI.
- Keep primary output on stdout and diagnostics on stderr.

---

## Error Output And Exit Codes

Agents recover by reading exit status, stderr, and suggestions.

Good:

```text
error: invalid value 'jsn' for '--format'

valid values:
  table
  json
  yaml

did you mean 'json'?
```

Poor:

```text
bad format
```

Recommended exit-code pattern:

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General runtime failure |
| 2 | Usage or validation error |
| 3 | Authentication or authorization error |
| 4 | Not found |
| 5 | Network or remote service error |

The exact mapping can vary, but it should be stable and documented.

Recommendations:

- Identify the bad input.
- Name the flag, argument, command, or precondition.
- List valid values.
- Suggest likely correction when confidence is high.
- Use stable nonzero exits for failure classes.
- Avoid stack traces for ordinary user errors.

---

## Config And Auth

Good:

```sh
mycli auth status --format json
mycli config get --format json
mycli config path
```

Poor:

```text
Any command may create config files without disclosure.
Auth failures look like generic network failures.
```

Recommendations:

- Use XDG paths where practical.
- Document config writes.
- Make auth status machine-readable.
- Keep login interactive if needed, but make auth errors actionable.
- Support scoped test credentials for CI when feasible.
- Do not touch credential stores during help and version probes.

---

## Completion

Completion is useful for humans and some harnesses, but it is not the primary current CLIARE discovery path.

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

Recommendations:

- Generate completion from the same definitions as help and parser behavior.
- Keep completion deterministic.
- Avoid network calls in completion unless explicitly documented.
- Treat stale completion as a product issue even when CLIARE can still infer shape from help.

---

## Versioning And Drift

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
  "schema_version": "mycli.version.v1"
}
```

Recommendations:

- Version the CLI.
- Version output schemas when practical.
- Avoid breaking command shape in patch releases.
- Deprecate before removal.
- Keep help, parser behavior, examples, and output contracts aligned in each release.

---

## CLIARE Feedback Loop

Use CLIARE as a maintainer loop:

```sh
cliare measure mycli --out .cliare/mycli --profile standard --refresh
cliare report maintainer --out .cliare/mycli --format markdown
cliare issues list --out .cliare/mycli --format markdown
```

After fixes:

```sh
cliare measure mycli --out .cliare/mycli --profile deep --refresh
cliare report maintainer --out .cliare/mycli --write
cliare describe .cliare/mycli --write
```

When a baseline exists:

```sh
cliare guard mycli \
  --baseline .cliare-baseline/mycli/scorecard.json \
  --out .cliare/mycli \
  --profile deep \
  --allowed-drop 2
```

For authenticated behavior:

```sh
cliare measure mycli \
  --out .cliare/mycli \
  --context authenticated \
  --auth-state present \
  --execution-mode host \
  --profile deep \
  --refresh
```

For long runs:

```sh
cliare measure mycli --out .cliare/mycli --profile deep --detach
cliare jobs status --out .cliare/mycli
```

Use `cliare playbook maintainer --target mycli` for the full step-by-step maintainer workflow.

---

## Improvement Checklist

High-impact fixes:

- [ ] Root `--help`, `-h`, `help`, `--version`, and `version` work without auth.
- [ ] Every public command has command-specific `--help`.
- [ ] Usage strings show the full command path.
- [ ] Positionals have names and required/optional/variadic shape.
- [ ] Flags declare arity and enum values.
- [ ] Unknown commands and flags fail clearly.
- [ ] List/show/read commands support JSON or YAML.
- [ ] Machine-readable stdout is clean.
- [ ] Required operands have safe examples or fixtures.
- [ ] Mutating commands support dry-run or plan output.
- [ ] Destructive commands require explicit confirmation bypass.
- [ ] Safe discovery probes do not leave durable side effects.
- [ ] Auth, config, network, daemon, and local-context preconditions are explicit.
- [ ] CI runs `cliare measure` or `cliare guard`.
- [ ] Agent-facing artifacts are published or attached.

---

## Before And After

Before:

```sh
mycli deploy prod
```

Problems:

- ambiguous positional
- mutating command
- no dry-run
- no machine-readable output
- unclear environment selection

After:

```sh
mycli deploy --env prod --dry-run --format json
mycli deploy --env prod --yes --format json
```

Improvements:

- explicit environment flag
- dry-run evidence
- parseable output
- confirmation bypass is explicit
- easier for agents to plan and verify

---

## What CLIARE Currently Rewards

CLIARE currently rewards evidence that a CLI is:

- discoverable through help and diagnostics
- explicit about flags, arguments, and usage
- consistent between help and runtime behavior
- parseable through JSON or YAML output modes
- safe during discovery probes
- recoverable through useful errors
- bounded and noninteractive under probes

It does not reward documentation that contradicts runtime behavior.

---

## What CLIARE Currently Flags

CLIARE may flag:

- missing machine-readable output modes
- advertised output modes that cannot be safely validated
- required operands that need fixtures
- parse failures in JSON/YAML output probes
- help/runtime mismatch
- incomplete command-specific help
- ambiguous or missing flag/argument grammar
- poor invalid command or invalid flag diagnostics
- safe-probe filesystem side effects
- credential-like file writes
- incomplete traversal because depth or probe budget was exhausted

Maintainers do not have to treat every issue as a defect. Use dispositions for intentional, not-applicable, accepted-risk, false-positive, deferred, or fixture-gated cases.

---

## Maintainer Contract

The practical contract is:

```text
Anything a human can discover through help should work at runtime.
Anything an agent needs to parse should have machine-readable output.
Anything that mutates state should have a preview or dry-run path.
Anything that needs auth, fixtures, a local repo, a daemon, or network should say so directly.
Anything that fails should fail with enough structure to recover.
```

That is agent-ready CLI design.
