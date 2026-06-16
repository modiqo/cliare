# 27 - Maintainer Playbook

> **Scope:** End-to-end command sequence for maintainers adopting CLIARE.
> **Status:** Current implementation.

---

## Purpose

The maintainer playbook removes setup and parameter guessing. It gives maintainers one role-specific sequence:

```text
measure -> view -> act or disposition -> remeasure -> gate in CI -> publish agent surface
```

The command form is:

```sh
cliare playbook maintainer --target mycli
```

The generated playbook uses schema `cliare.playbook.v1` in JSON mode. The default output is `human`: a short terminal walkthrough that tells you exactly what to run next. Use `--format markdown` for a full document and `--format json` for automation.

```sh
cliare playbook maintainer --target mycli --format human
```

By default, the playbook uses `.cliare/<target-cli>` as the artifact directory. This is a project-local folder relative to the directory where you run CLIARE, not a global database. Do not use bare `.cliare` when it contains multiple target folders. If the target is not known yet, the playbook prints placeholder commands with `<target-cli>`.

Use `--out` only to override the artifact directory:

```sh
cliare playbook maintainer --target mycli --out /tmp/cliare-mycli
```

For context runs, CLIARE writes under:

```text
.cliare/<target-cli>/contexts/<context>
```

`cliare playbook maintainer --context <name>` selects an existing context for generated `describe`, `report`, `issues`, and `jobs status` commands. It does not create that context. Create context artifacts first with `cliare measure --context <profile> ...`, then use the same artifact root and context name for review.

---

## Maintainer Lifecycle

### 1. Measure

Start with `standard` for normal development:

```sh
cliare measure mycli --out .cliare/mycli --profile standard --refresh
```

Use `quick` during tight local edits:

```sh
cliare measure mycli --out .cliare/mycli --profile quick --refresh
```

Use `deep` before CI baselines, releases, and agent-surface publishing:

```sh
cliare measure mycli --out .cliare/mycli --profile deep --refresh
```

For large CLIs with deep command trees:

```sh
cliare measure mycli --out .cliare/mycli --profile deep --max-depth 12 --max-probes 5000 --concurrency 8 --refresh
```

For long-running measurements:

```sh
cliare measure mycli --out .cliare/mycli --profile deep --max-depth 12 --max-probes 5000 --concurrency 8 --refresh --detach
cliare jobs status --out .cliare/mycli
```

For authenticated or host-context behavior:

```sh
cliare measure mycli --out .cliare/mycli --context authenticated --auth-state present --execution-mode host --profile deep --refresh
```

After that run, review the authenticated context with:

```sh
cliare playbook maintainer --target mycli --out .cliare/mycli --context authenticated
```

### 2. View

Open the artifact map, maintainer report, issue ledger, and focused drilldowns:

```sh
cliare issues list --out .cliare/mycli --format human
cliare describe .cliare/mycli --format markdown
cliare report maintainer --out .cliare/mycli --format markdown
cliare issues list --out .cliare/mycli --format markdown
cliare report maintainer --out .cliare/mycli --area output-contracts --format markdown
cliare report maintainer --out .cliare/mycli --issue <issue-id> --with-evidence --format bundle
```

If the measurement was detached, wait until `cliare jobs status --out .cliare/mycli` reports `complete` before running report or issue commands.

### 3. Act

Fix concrete contract gaps before advisory compatibility work:

1. Output contracts: parseable JSON/YAML, safe dry-run behavior, fixture paths.
2. Preconditions: auth, local context, daemon, network, runtime dependency, fixture requirements.
3. Command discovery: command-specific `--help` and stable usage syntax.
4. Diagnostics: invalid command and invalid flag recovery.
5. Safety: discovery-time side effects and credential-like paths.
6. Compatibility advisories: optional conventions such as `help <path>`.

### 4. Disposition

Record maintainer decisions when an issue is intentional, not applicable, accepted risk, deferred, a false positive, or fixture-gated:

```sh
cliare issues mark <issue-id> --out .cliare/mycli --status intentional --reason "Direct <command> --help is canonical for this CLI."
cliare issues mark <issue-id> --out .cliare/mycli --status needs-fixture --reason "Requires safe fixture operands for <id> and <endpoint-url>."
cliare issues list --out .cliare/mycli --format markdown
```

Disposition status values are passed as kebab-case on the CLI, for example `needs-fixture`, `accepted-risk`, and `false-positive`. The stored `issue-dispositions.json` uses snake_case values such as `needs_fixture`, `accepted_risk`, and `false_positive`.

### 5. Remeasure

After fixes or dispositions:

```sh
cliare measure mycli --out .cliare/mycli --profile deep --refresh
cliare report maintainer --out .cliare/mycli --write
cliare issues list --out .cliare/mycli --format markdown
```

### 6. Gate in CI

Use `guard` once a baseline exists:

```sh
cliare guard mycli --baseline .cliare-baseline/mycli/scorecard.json --out .cliare/mycli --profile deep --allowed-drop 2
```

### 7. Publish Agent Surface

Publish or attach the files an agent harness should read before invoking the target CLI:

```sh
cliare describe .cliare/mycli --write
cliare report harness --out .cliare/mycli --write
cliare skills install --agent all --scope project
cliare metadata --format json
```

`cliare report harness --write` also refreshes shared report artifacts such as `issues.json`, `issues.md`, and the local artifact guide files. `cliare skills install --agent all --scope project` installs CLIARE artifact-review skills into project-level agent directories for supported agents. Add `--project-dir <dir>` when running from outside the project root.

Agent-facing artifacts:

- `command-index.json`
- `command-index.md`
- `issues.json`
- `issues.md`
- `issue-dispositions.json`
- `persona-harness.json`
- `persona-harness.md`
- `AGENT_SKILL.md`
- `metadata --format json` command spec

---

## Parameter Guide

Most maintainers should choose only `quick`, `standard`, or `deep`. Change advanced parameters only when the report points to traversal pressure.

| Parameter | Meaning | Change When |
|---|---|---|
| `--profile quick` | Small local smoke pass. | Editing help, diagnostics, or one output contract. |
| `--profile standard` | Balanced default pass. | Normal maintainer loop. |
| `--profile deep` | Broader release-quality pass. | CI baseline, release, or publishing agent surface. |
| `--max-depth` | Recursive command-path depth. | Nested command families are missing or `observed_max_depth == max_depth`. |
| `--max-probes` | Maximum runtime probes. | `budget_exhausted=true`, `frontier_remaining > 0`, or too many candidate commands remain. |
| `--concurrency` | Probes run at the same time. | Lower for rate limits, shared state, daemons, or flaky CLIs; raise only for stable local CLIs. |
| `--timeout-ms` | Per-probe timeout. | The CLI is slow, network-backed, daemon-backed, or package-manager-like. |
| `--output-limit-bytes` | Retained stdout/stderr bytes per probe. | Help or machine output is legitimately large. |
| `--execution-mode isolated` | Default sandboxed profile. | Use for safe local probing. |
| `--execution-mode host` | Host config, auth, plugins, and local state are visible. | Measuring authenticated or host-specific behavior. |
| `--context` | Runtime context profile for measurement, or context selector for view/report/issue commands. | Compare clean, authenticated, local-context, or fixture-backed behavior. |
| `--context-workdir` | Working directory that supplies local project/repository context. | Measuring commands that require a repo, project, workspace, or config files. |

Increase depth or probes when:

- `budget_exhausted` is true.
- `frontier_remaining` is greater than zero.
- `observed_max_depth` equals `max_depth` and nested command families are missing.
- Many commands remain `candidate` instead of `runtime_confirmed`.
- Many machine-readable output contracts are unprobed and the missing condition is traversal budget, not a fixture.

Do not increase depth or probes when the report shows a real precondition such as auth, fixture, daemon, local repo, network, or runtime dependency. In that case, provide the context or mark the issue with a disposition.

---

## Completion Criteria

A maintainer pass is complete when:

- high severity issues are fixed, fixture-gated, dispositioned, or accepted risk
- output contracts are parse-success, documented precondition, or `needs_fixture`
- optional compatibility advisories are fixed or marked intentional/not applicable
- `command-index.json` reflects the intended agent routing surface
- `issues list` shows reviewed decisions instead of repeated noise
- CI runs `measure` or `guard`
- agent-facing artifacts are published or attached
