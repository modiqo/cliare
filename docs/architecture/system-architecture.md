# 03 - System Architecture

> **Scope:** Current component model, data flow, storage layout, CLI surface, and execution lifecycle.
> **Status:** Current Implementation

---

## Architectural Goal

CLIARE inspects arbitrary command-line binaries without source access and produces evidence-backed artifacts for three jobs:

- Maintainers get a review queue for agent-readiness gaps, drift, preconditions, and side effects.
- Agent harnesses get a command index and generated artifact-review skill so they can use a CLI deliberately.
- Security and platform teams get reproducible evidence about filesystem side effects and policy posture.

The core architecture separates runtime probing from derived artifacts:

```text
Probe once. Derive command shape, command index, scorecards, reports, and issue ledgers from the same evidence.
```

When report formatting, issue grouping, or scoring changes, CLIARE can re-read preserved measurement artifacts. A fresh target probe run is only required when the target binary, runtime context, profile, or requested traversal budget changes enough that the existing artifacts should not be reused.

---

## Source Of Truth For The CLI Surface

The public binary is `cliare`. The checked-in source of truth for commands is the clap definition in `src/cli.rs`, and the machine-readable command contract is available through:

```sh
cliare metadata --format json
```

The human-readable command summary is:

```sh
cliare metadata --format text
```

As of this implementation, the public command families are:

```text
cliare measure <target>
cliare jobs status
cliare guard <target> --baseline <scorecard.json>
cliare benchmark
cliare context compare <context-dir>...
cliare report <persona>
cliare describe [artifact-dir]
cliare skills list
cliare skills install
cliare issues list
cliare issues mark <issue-id>
cliare playbook <maintainer|harness|security>
cliare metadata
```

Do not document additional CLI commands unless they are present in `cliare metadata --format json`.

---

## Major Components

```text
+--------------------+
| CLI Frontend       |
+---------+----------+
          |
          v
+--------------------+
| Target Resolver    |
+---------+----------+
          |
          v
+--------------------+
| Probe Planner      |
+---------+----------+
          |
          v
+--------------------+       +--------------------+
| Probe Scheduler    +------>+ Execution Runtime  |
+---------+----------+       +---------+----------+
          |                            |
          | observations               | argv exec
          v                            v
+--------------------+       +--------------------+
| Evidence Writer    |<------+ Target CLI Binary  |
+---------+----------+       +--------------------+
          |
          v
+--------------------+
| Shape Builder      |
+---------+----------+
          |
          v
+--------------------+
| Score And Issues   |
+---------+----------+
          |
          v
+--------------------+
| Reports And Skills |
+--------------------+
```

The implementation is intentionally single-binary and artifact-file oriented. There is no server, database, plugin runtime, or remote publishing service in the current architecture.

---

## CLI Frontend

The CLI frontend parses arguments, resolves runtime context options, and dispatches to the module that owns the command:

| Command | Primary module | Purpose |
|---------|----------------|---------|
| `measure` | `src/measure.rs` | Probe a target CLI and write the measurement artifact bundle. |
| `jobs status` | `src/jobs.rs` | Inspect progress for foreground or detached measurement jobs. |
| `guard` | `src/guard.rs` | Measure a target and fail if score or policy posture regresses against a baseline. |
| `benchmark` | `src/benchmark.rs` | Run a target corpus and write benchmark reports. |
| `context compare` | `src/context.rs` | Compare multiple measurement contexts. |
| `report` | `src/report.rs` | Print or write persona-specific outcome packets. |
| `describe` | `src/describe.rs` | Produce an artifact map for humans and agents. |
| `skills` | `src/skills.rs` | List or install CLIARE artifact-review skills. |
| `issues` | `src/issues.rs` | List issues and record maintainer dispositions. |
| `playbook` | `src/playbook.rs` | Print step-by-step workflows for maintainers, harnesses, and security reviewers. |
| `metadata` | `src/command_spec.rs` | Print implementation metadata and the command spec. |

The CLI frontend does not contain scoring or probing logic. It should remain a boundary layer that turns user input into typed command arguments and prints summaries or errors.

---

## Measurement Lifecycle

`cliare measure` is the main data-producing command.

```text
target resolution
  -> runtime context resolution
  -> cache compatibility check
  -> progress log initialization
  -> adaptive probe traversal
  -> evidence.jsonl
  -> shape.json and command-index.*
  -> scorecard.json and report.md
  -> CI artifacts
  -> persona reports and issues ledger
  -> artifact guides
  -> measure-cache.json
```

A measurement can run in the foreground or with `--detach`. Detached runs write `jobs/current` and progress logs under the artifact directory, and `cliare jobs status --out <artifact-dir>` reads that state.

The measurement profile controls default traversal budgets:

| Profile | Default max depth | Default max probes | Default minimum expected value | Default concurrency |
|---------|-------------------|--------------------|--------------------------------|---------------------|
| `quick` | 3 | 64 | 300 | 2 |
| `standard` | 5 | 256 | 150 | 4 |
| `deep` | 8 | 1000 | 50 | 8 |

Users can override those defaults with `--max-depth`, `--max-probes`, `--min-expected-value`, and `--concurrency`.

---

## Target Resolution And Caching

`measure` accepts either a filesystem path or a command name that can be resolved from `PATH`. Detached measurement preflights target resolution before writing job artifacts so a missing target fails early.

CLIARE writes `measure-cache.json` and reuses compatible artifacts unless `--refresh` is passed. Cache reuse is based on the target fingerprint and the effective probe profile. Reused artifacts are still completed with persona reports and artifact guides when needed.

The current target fingerprint records the requested and resolved target identity used by measurement artifacts. It is not a public cache command surface; users interact with reuse through `measure --refresh`.

---

## Probe Planner And Scheduler

The planner starts from generic safe probes such as direct help and version-style invocations, then schedules additional probes from evidence discovered during traversal.

The scheduler favors probes that improve command-shape confidence:

- direct help forms such as `<command> --help` and `<command> -h`
- optional compatibility help forms such as `help <command path>`
- command-path confirmation probes
- diagnostic probes for invalid or incomplete invocations
- output-mode probes when a command advertises parseable output and required operands are known or safely fixtureable
- side-effect observation for probes that CLIARE classifies as safe to execute

The scheduler is bounded by max depth, max probes, minimum expected value, concurrency, per-probe timeout, and per-probe output byte limits. Target CLI failures are usually evidence about the target, not CLIARE failures.

---

## Execution Runtime

CLIARE executes target commands as explicit argv arrays. It does not route target invocations through a shell.

`measure` and `guard` support these execution modes through `--execution-mode`:

- `isolated`: run probes with an isolated temporary home and working directory.
- `host`: run probes in the host environment when the user intentionally wants authenticated or host-specific behavior.

For each probe, the runtime records bounded stdout/stderr, exit status, duration, timeout status, and filesystem side-effect snapshots when supported by the selected runtime mode. The side-effect evidence is used by scorecards, issues, persona reports, and security review.

Network event tracing is not a current artifact contract. Network availability can be declared as part of runtime context state, and target diagnostics can be classified as network-precondition blocked when the evidence supports that classification.

---

## Evidence Writer

The measurement run writes `evidence.jsonl`. Each line is a structured observation for a scheduled or completed probe. Evidence entries include the probe id, intent, command path, argv, process result, bounded output metadata, classified diagnostics, precondition information, and side-effect information when present.

The evidence log is the lowest-level artifact users should inspect when they need to verify why a command, flag, issue, or score exists. Higher-level artifacts should point back to evidence identifiers instead of standing alone as unsupported claims.

---

## Shape Builder

The shape builder reads observations and produces `shape.json`. The shape artifact is an inference view over observed evidence, not a hand-authored CLI schema.

It contains the inferred command tree, flags, positionals, aliases, output contracts, preconditions, gaps, and confidence values. Help text creates candidates; runtime probes can confirm, weaken, or classify those candidates.

The current model is deterministic and local. It uses parser and runtime evidence with configured scoring/inference weights from the bundled score model. It does not call an external model service.

---

## Command Index

The command index is the agent-facing map of the target CLI:

- `command-index.json` is the machine-readable index.
- `command-index.md` is the human-readable companion.

The command index is derived from `shape.json` and evidence. It is designed for harnesses and skills that need to route through a CLI without rediscovering command syntax at runtime. It includes command paths, runtime state, agent suitability, parameters, output contracts, preconditions, and evidence pointers.

Generated agent-review material also includes `AGENT_SKILL.md`, and `cliare skills install` can install CLIARE artifact-review skills for supported local agent environments.

---

## Scoring, Issues, And Dispositions

The scoring engine consumes evidence and shape artifacts and writes:

- `scorecard.json`
- `report.md`
- `summary.md`
- `findings.sarif`
- `junit.xml`

The scorecard includes total score, subscores, coverage, traversal budget posture, findings, policy status, sandbox/runtime metadata, and bundled model provenance.

Persona reports and issue artifacts are written from the same measurement bundle:

- `issues.json`
- `issues.md`
- `persona-maintainer.{json,md}`
- `persona-harness.{json,md}`
- `persona-security.{json,md}`
- `persona-platform.{json,md}`
- `persona-oss.{json,md}`
- `persona-devrel.{json,md}`
- `persona-research.{json,md}`

Maintainers can record review decisions with:

```sh
cliare issues mark <issue-id> --out <artifact-dir> --status <status> --reason <reason>
```

Those decisions are stored in `issue-dispositions.json`. `cliare issues list` joins generated issues with dispositions so reviewed decisions can be separated from unresolved action items.

---

## Reports And Artifact Maps

`cliare report <persona>` prints or writes persona-specific packets from a measurement directory. Supported personas are `maintainer`, `harness`, `platform`, `security`, `oss`, `devrel`, and `research`.

`cliare describe [artifact-dir]` builds an artifact map for a measurement, benchmark, or context-suite directory. With `--write`, it writes:

- `artifact-map.json`
- `artifact-map.md`

The artifact map is the preferred navigation aid for humans and agents because it explains which files exist, what each file is for, and how to inspect the artifact bundle.

---

## Runtime Contexts

Runtime contexts let users measure the same CLI under different assumptions. For example:

```sh
cliare measure mycli --out .cliare/mycli --context clean --refresh
cliare measure mycli --out .cliare/mycli --context authenticated --execution-mode host --refresh
```

When `--context` is used, `--out` is treated as a suite root and the measurement is written under:

```text
<out>/contexts/<context-name>/
```

Context metadata is stored in `runtime-context.json`. Multiple context artifact directories can be compared with:

```sh
cliare context compare <context-dir>... --out .cliare-context --write
```

That writes `context-suite.json` and `context-compare.md`.

---

## Benchmark Lifecycle

`cliare benchmark` runs a corpus manifest and writes benchmark-level artifacts under the selected output directory. Each target gets its own measurement artifact bundle, and the benchmark root records aggregate status, score posture, and generated benchmark guides.

The default manifest is:

```text
benchmarks/local-corpus.json
```

Benchmarking is a local calibration workflow. It does not publish results to a hosted service in the current implementation.

---

## Storage Layout

A normal measurement directory looks like:

```text
.cliare/<target>/
  README.md
  AGENT_SKILL.md
  evidence.jsonl
  shape.json
  command-index.json
  command-index.md
  scorecard.json
  report.md
  summary.md
  findings.sarif
  junit.xml
  issues.json
  issues.md
  issue-dispositions.json        # present only after issues are marked
  measure-cache.json
  runtime-context.json
  persona-maintainer.json
  persona-maintainer.md
  persona-harness.json
  persona-harness.md
  persona-security.json
  persona-security.md
  persona-platform.json
  persona-platform.md
  persona-oss.json
  persona-oss.md
  persona-devrel.json
  persona-devrel.md
  persona-research.json
  persona-research.md
  jobs/
    current
    <job-id>.log
    <job-id>.stdout.log
    <job-id>.stderr.log
```

A context suite root looks like:

```text
.cliare/<target>/
  contexts/
    clean/
      scorecard.json
      command-index.json
      evidence.jsonl
      ...
    authenticated/
      scorecard.json
      command-index.json
      evidence.jsonl
      ...
```

`cliare report`, `cliare issues`, `cliare describe`, and `cliare jobs status` accept `--context <name>` when `--out` points at a context suite root.

---

## Configuration And Policy

There is no general project configuration file in the current implementation.

Measurement behavior is controlled through command-line options:

- traversal profile and budgets on `measure` and `guard`
- execution mode on `measure` and `guard`
- runtime context fields on `measure` and `guard`
- baseline and optional policy file on `guard`
- corpus manifest on `benchmark`

`guard --policy <file>` is the current policy entry point. Policy validation is not exposed as a separate public command.

---

## Failure Model

CLIARE distinguishes its own failures from target CLI behavior.

| Failure | Meaning | Artifact Behavior |
|---------|---------|-------------------|
| Missing target | The requested target cannot be resolved | Measurement fails before creating detached job artifacts. |
| Probe timeout | Target invocation exceeded the per-probe timeout | Evidence records the timeout. |
| Target nonzero exit | Target returned a failing status | Evidence records the exit and diagnostics may become findings or preconditions. |
| Output limit reached | Target output exceeded retention limits | Evidence keeps bounded output metadata. |
| Precondition blocked | Target indicates auth, local context, fixture data, network, or runtime dependency is missing | Evidence and issues classify the precondition instead of treating it as an ordinary parser failure. |
| CLIARE artifact read/write failure | CLIARE cannot read or write required artifacts | Command fails with an error. |
| Guard regression | Score or policy posture violates the requested baseline/policy | `guard` exits nonzero after writing current artifacts. |

CLIARE should preserve evidence already written by successful probes even when later artifact stages fail.

---

## Security Boundary

The target CLI is untrusted.

Assumptions:

- It may read environment variables.
- It may write files.
- It may spawn subprocesses.
- It may attempt network calls.
- It may behave differently under CI, host state, auth state, or local repository state.
- It may print sensitive values from inherited config.
- It may hang or emit large output.

Therefore CLIARE defaults to isolated execution, bounded output, timeouts, explicit argv execution, and side-effect observation. Host execution exists for intentional authenticated or host-specific measurement and should be chosen deliberately.

---

## Keeping This Document Honest

This document should describe the implementation that exists in this repository. Before adding a public command, artifact, configuration file, or integration point here:

1. Add or update the implementation.
2. Verify the public command surface with `cliare metadata --format json`.
3. Verify generated measurement artifacts from a real `cliare measure` run.
4. Update this document from those implementation facts.

Do not use this document to reserve future command names or describe planned subsystems as if they already exist.
