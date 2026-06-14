# 22 - Agent-Ready CLI Standard Template

> **Scope:** Reference behavior template for CLIs that want strong CLIARE scores and reliable agent integration.
> **Status:** Standard Draft

---

## Purpose

This document defines a practical template for command-line interfaces that are easy for humans, automation, and agent harnesses to discover and use. It is not a replacement for POSIX, GNU, or project-specific conventions. It is a compatibility layer for modern CLIs that need to be measured, routed, and operated by agents.

The template has two jobs:

1. Give maintainers a clear target when improving an existing CLI.
2. Give new projects a starting point that will measure well under CLIARE without needing special-case inference.

CLIARE does not require a project to use a specific parser, framework, programming language, or command structure. A CLI can be written with Clap, Cobra, Click, argparse, shell scripts, or a custom parser and still conform. The contract is runtime behavior.

---

## Source Lineage

The template is informed by established CLI conventions:

- GNU command-line standards recommend following POSIX option guidelines, providing long options, and supporting `--help` and `--version`: https://www.gnu.org/prep/standards/html_node/Command_002dLine-Interfaces.html
- The Command Line Interface Guidelines emphasize parser libraries, zero/nonzero exit codes, stdout for primary output, stderr for diagnostics, discoverable help, robust errors, subcommands, and composability: https://clig.dev/

CLIARE extends those conventions for agent readiness:

- every command should be discoverable from the released binary
- help should be parseable enough to infer command shape
- output contracts should be machine-readable and stable
- safe discovery should avoid durable side effects
- runtime preconditions should be explicit and separable from command existence
- CI should publish scorecards and issue ledgers

---

## Required Runtime Surface

Every CLI should support these root-level commands or flags:

```text
mycli --help
mycli -h
mycli help
mycli --version
mycli version
```

Recommended behavior:

| Invocation | Exit | stdout | stderr | Side effects |
|---|---:|---|---|---|
| `mycli --help` | `0` | root help | empty unless warning is needed | none |
| `mycli -h` | `0` | root help | empty unless warning is needed | none |
| `mycli help` | `0` | root help | empty unless warning is needed | none |
| `mycli --version` | `0` | version string or object | empty | none |
| `mycli version` | `0` | version string or object | empty | none |

`--help` and `--version` should never require authentication, profile setup, network access, a writable config directory, or a project checkout.

---

## Help Contract

Help is the primary discovery API for both humans and agents. A command-specific help screen should identify the command path, usage grammar, arguments, flags, examples, output contracts, preconditions, and exit codes.

Recommended structure:

```text
mycli project list - List projects

USAGE
  mycli project list [OPTIONS]

DESCRIPTION
  Lists projects visible to the current profile.

ARGUMENTS
  <ORG>        Organization slug. Optional when MYCLI_ORG is set.

OPTIONS
  --format <FORMAT>    Output format: text, json, yaml. [default: text]
  --limit <N>          Maximum projects to return. [default: 100]
  --profile <NAME>     Profile to use.
  -h, --help           Print help.

OUTPUT
  --format json        Prints a JSON object to stdout.
  --format yaml        Prints a YAML document to stdout.

PRECONDITIONS
  Requires an authenticated profile for runtime data.
  Help and schema metadata do not require authentication.

EXIT CODES
  0  success
  2  usage error
  3  authentication required
  4  permission denied
  5  network or service error

EXAMPLES
  mycli project list --format json
  mycli project list --profile prod --limit 20
```

Rules:

- Use a stable `USAGE` section for every command.
- Show the full command path in usage, not only the leaf command.
- Mark required operands with `<NAME>`.
- Mark optional operands with `[NAME]`.
- Mark variadic operands with `...`.
- Keep command tables under headings such as `COMMANDS` or `SUBCOMMANDS`.
- Keep data tables under headings such as `FIELDS`, `KEYS`, `VALUES`, `OUTPUT`, or `EXIT CODES`.
- Do not put environment variable tables or enum values under `COMMANDS`.
- If a command has aliases, document the canonical command and aliases.
- If help for an alias prints the canonical usage, keep the alias listed in the parent command table.

---

## Command Catalog Contract

For multi-command CLIs, the root help should expose the command tree in a predictable format:

```text
COMMANDS
  project        Manage projects
  project list   List projects
  project show   Show one project
  auth           Manage authentication
```

Acceptable alternative:

```text
COMMANDS
  project        Manage projects
  auth           Manage authentication

Run `mycli help <command>` for details.
```

If the root lists only first-level commands, each parent command must list its children.

Anti-patterns:

- hiding subcommands behind interactive menus
- requiring authentication to print a command list
- printing parent help for unknown child commands while exiting `0`
- mixing commands, enum values, environment variables, and config keys in one unlabelled table

---

## Argument and Flag Contract

Flags should be explicit and stable.

Recommended:

```text
--format <FORMAT>      Output format: text, json, yaml. [default: text]
--output <FILE>        Write primary output to a file.
--profile <NAME>       Profile to use.
--yes                  Disable confirmation prompts.
--no-color             Disable color output.
--quiet                Suppress non-essential human messages.
--verbose              Print additional diagnostic detail to stderr.
```

Rules:

- Prefer long options for scripts and agents.
- Use short options only as aliases for common human workflows.
- Use the same flag names consistently across commands.
- Use `--output <FILE>` or `-o <FILE>` for output files.
- Use `--format <FORMAT>` for output representation.
- Use `--json` only as a boolean shorthand for JSON output.
- Do not use names such as `--config-json <JSON>` to mean output format; that is an input payload.
- Document valid values for enum-like flags.
- Reject unknown flags with a nonzero exit code.
- Reject missing required flag values with a nonzero exit code.
- Support `--` to terminate option parsing when positional operands can begin with `-`.

---

## Output Contract

Human output and machine output are separate contracts.

Human-readable mode:

- optimized for scanning
- may include colors when stdout is a terminal
- should avoid color when `NO_COLOR` is set or `--no-color` is passed
- should not be parsed by agents unless no machine mode exists

Machine-readable mode:

- written to stdout
- valid JSON, YAML, NDJSON, or another documented format
- no progress spinners, banners, warnings, or prose mixed into stdout
- diagnostics and progress go to stderr
- stable top-level shape across releases
- includes explicit nulls or omitted fields consistently
- documented with examples

Recommended:

```sh
mycli project list --format json
mycli project list --json
```

For JSON:

```json
{
  "projects": [],
  "next_cursor": null
}
```

Do not emit:

```text
Loading projects...
{"projects":[]}
```

If progress is required, write progress to stderr.

---

## Help and Output Precedence

Both of these are acceptable:

```sh
mycli project list --json --help
```

Option A: help takes precedence and prints prose help.

Option B: output mode applies and prints machine-readable help metadata.

The behavior must be consistent and documented. CLIARE treats Option B as stronger for agent harnesses because it enables schema discovery, but Option A is not a runtime output failure. The direct runtime contract is measured with:

```sh
mycli project list --json
```

---

## Runtime Preconditions

A command may require authentication, a project directory, network access, a profile, or external service state. Those preconditions should be visible without satisfying them.

Recommended:

```text
PRECONDITIONS
  Runtime data requires authentication.
  Configure with: mycli auth login
  Help and schema output do not require authentication.
```

Rules:

- Help should not require auth.
- Version should not require auth.
- Command shape/schema should not require auth when practical.
- If runtime data requires auth, return a specific nonzero exit code.
- Put remediation on stderr.
- Keep the error short and structured.

Example:

```text
auth required

Fix:
  mycli auth login
```

---

## Safe Discovery Contract

Agents and CI will run discovery probes. Safe discovery invocations should be read-only.

Safe invocations include:

```text
--help
-h
help
--version
version
invalid child probe
invalid flag probe
machine-readable metadata probe
```

Rules:

- Do not write config files during help.
- Do not create telemetry files during help unless telemetry is explicitly enabled.
- Do not acquire durable locks during help.
- Do not touch credential stores during help.
- If a cache must be created, document it and keep it outside credential-like paths.
- Respect `CI=1`, `NO_COLOR=1`, and non-interactive stdin.

---

## Diagnostics and Recovery

Agents recover from errors by reading exit codes, stderr, and suggestions.

Recommended invalid flag behavior:

```text
error: unknown flag `--jsno`

Did you mean `--json`?

Usage:
  mycli project list [OPTIONS]
```

Rules:

- Return nonzero for usage errors.
- Put errors on stderr.
- Include the offending token.
- Include a short correction when confidence is high.
- Include usage or a help pointer.
- Do not silently ignore unknown flags or unknown subcommands.
- Do not fall through to parent command behavior on unknown child commands.

---

## Agent Metadata Command

Projects that want strong harness integration should expose a metadata command:

```sh
mycli metadata --format json
```

Minimum shape:

```json
{
  "schema_version": "mycli.metadata.v1",
  "name": "mycli",
  "version": "1.2.3",
  "commands": [
    {
      "path": ["project", "list"],
      "summary": "List projects",
      "usage": "mycli project list [OPTIONS]",
      "output": [
        {
          "format": "json",
          "argv": ["--format", "json"],
          "schema": "https://example.com/schemas/project-list.v1.json"
        }
      ],
      "preconditions": ["auth"]
    }
  ]
}
```

This command is optional. CLIARE must work without it. When present, it gives agents and CLIARE stronger evidence than prose help alone.

---

## Fixture Contract

Some commands cannot be safely exercised without operands. To make those contracts measurable, provide a fixture profile.

Recommended:

```sh
mycli fixtures print --format json
mycli project show --id fixture-project --format json
mycli project update --id fixture-project --name "New name" --dry-run --format json
```

Rules:

- Fixtures must be safe for CI.
- Dry-run output should match real output shape where practical.
- Fixture IDs should be documented.
- Mutating commands should expose `--dry-run` or `--plan`.
- Fixture setup should be explicit, not hidden in help execution.

---

## CI Template

Recommended GitHub Actions shape:

```yaml
name: cliare

on:
  pull_request:
  push:
    branches: [main]

jobs:
  cliare:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --release
      - run: cliare measure ./target/release/mycli --out .cliare --profile deep --refresh
      - run: cliare report maintainer --out .cliare --write
      - run: cliare guard ./target/release/mycli --baseline .cliare/baseline.scorecard.json --out .cliare
      - uses: actions/upload-artifact@v4
        with:
          name: cliare
          path: .cliare
```

For public projects, publish:

- `scorecard.json`
- `issues.md`
- `persona-oss.md`
- badge data when stable

---

## Minimum Conformance Checklist

A CLI is CLIARE-ready when:

- root `--help`, `-h`, `help`, `--version`, and `version` work without auth
- every command has command-specific help
- every command help includes full usage
- required and optional operands are visible
- flags document arity and valid values
- JSON/YAML modes produce parseable stdout
- diagnostics use nonzero exits and stderr
- safe discovery has no durable side effects
- preconditions are documented separately from command existence
- CI publishes CLIARE artifacts

---

## High-Score Checklist

A CLI is strongly agent-ready when:

- command metadata is available as JSON
- output schemas are documented or linked
- list/show/read commands all support JSON
- mutating commands support dry-run or plan output
- fixture profiles allow safe validation of important contracts
- aliases resolve to canonical help
- unknown child commands and flags are rejected clearly
- all help and metadata paths are deterministic in clean CI
- public scorecards include binary fingerprints and traversal profile
