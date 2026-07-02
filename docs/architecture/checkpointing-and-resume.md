# 10 - Cache, Jobs, And Resume Direction

> **Scope:** Measurement reuse, detached execution, progress reporting, artifact lifecycle, crash behavior, and checkpoint/resume behavior.
> **Status:** Current implementation plus remaining direction.

---

## Summary

CLIARE currently supports:

- artifact-level measurement reuse through `measure-cache.json`
- internal probe-level checkpoint/resume for compatible interrupted measurements
- progress logs under the measurement artifact directory
- foreground and detached measurement job tracking
- an active-job guard for detached measurements
- benchmark output locking and atomic benchmark report writes

CLIARE does not currently expose a public `--resume`, `replay`, or `rescore` command. Resume is internal and conservative: a fresh non-`--refresh` measurement may resume from `<artifact-dir>/measure-checkpoint.json` only when the target fingerprint, probe profile, runtime context, CLIARE version, engine, checkpoint counters, and in-progress evidence log are compatible.

The current rule is:

> Completed measurement artifacts may be reused exactly. Compatible partial probe execution may resume from the last completed probe checkpoint; incompatible or incomplete checkpoint state is treated as a fresh cache miss.

---

## Current Measurement Lifecycle

The implemented `cliare measure` lifecycle is:

```text
resolve runtime context
  |
  v
resolve and fingerprint target binary
  |
  v
build probe profile from CLI options and runtime context
  |
  v
if --refresh is not set, check measure-cache.json
  |
  +--> cache hit: regenerate lightweight guides/reports and return summary
  |
  v
cache miss: read compatible measure-checkpoint.json or clean abandoned in-progress evidence
  |
  v
create progress log and runtime-context.json
  |
  v
create sandbox and evidence writer, or append to checkpointed in-progress evidence
  |
  v
run deterministic traversal until frontier exhaustion or probe budget, writing a checkpoint after each completed probe
  |
  v
write evidence, shape, command index, scorecard, reports, issues, and guides
  |
  v
write measure-cache.json and remove measure-checkpoint.json
```

The cache check happens before probes are executed. `--refresh` bypasses the cache and forces a new probe run.

---

## Artifact Directory

`--out` controls the measurement artifact directory.

For a single-context run:

```sh
cliare measure mise --out .cliare/mise
```

Artifacts are written directly under `.cliare/mise`.

For a context-suite run:

```sh
cliare measure rote --out .cliare/rote --context authenticated
```

`--out` is treated as the suite root, and artifacts are written under the selected context directory. Commands that read the result need the same artifact directory, or the same suite root plus `--context` when supported.

---

## Current Cache Manifest

The current cache file is:

```text
<artifact-dir>/measure-cache.json
```

Its schema version is:

```text
cliare.measure-cache.v1
```

It records:

- `schema_version`
- `cliare_version`
- `engine`
- `run_id`
- target fingerprint
- probe profile
- per-artifact SHA-256 digests and sizes for required measurement artifacts
- measurement summary facts

The current engine label is:

```text
cliare-measure-v0
```

A cache hit requires all of the following:

- the cache schema matches `cliare.measure-cache.v1`
- the cached CLIARE package version matches the running binary
- the cached engine matches `cliare-measure-v0`
- the target fingerprint matches the current target
- the probe profile matches the current run options and runtime context
- all required measurement artifacts still exist
- all recorded required-artifact digests and sizes match the files on disk

If any check fails, CLIARE treats the cache as a miss and runs probes again.

---

## Required Cache Artifacts

The current cache is only reused when these files exist:

| Artifact | Purpose |
|----------|---------|
| `evidence.jsonl` | Runtime evidence events from the completed measurement |
| `shape.json` | Inferred command shape |
| `command-index.json` | Machine-readable command index |
| `command-index.md` | Human-readable command index |
| `condition-dictionary.csv` | CSV decoder for report labels and condition meanings |
| `scorecard.json` | Scorecard and scoring facts |
| `report.md` | Measurement report |
| `summary.md` | CI-oriented summary |
| `findings.sarif` | SARIF output |
| `junit.xml` | JUnit output |

Other files may also be present, including `issues.json`, `issues.md`, `runtime-context.json`, `README.md`, `AGENT_SKILL.md`, persona reports, and job logs. Those are useful artifacts, but they are not currently part of the required cache-hit file list.

---

## Cache Hit Behavior

On a cache hit, CLIARE does not execute probes again. It reconstructs the measurement summary from `measure-cache.json` and refreshes lightweight derivative artifacts that can depend on current report generation code.

The current cache-hit path refreshes:

- `runtime-context.json`
- persona reports
- measurement guides
- context-suite metadata when the run is part of a context suite

This is intentionally conservative. The cache reuses a completed measurement for the same binary, profile, and context. It does not try to reuse individual probes across changed profiles or changed targets.

---

## Evidence Log

The current evidence log is:

```text
<artifact-dir>/evidence.jsonl
```

It is line-delimited JSON with schema version:

```text
cliare.evidence.v1
```

Current event kinds are:

- `run_started`
- `probe_scheduled`
- `process_completed`
- `run_finished`

The writer flushes each event after writing it. A fresh run opens `evidence.jsonl` with truncate semantics, so an earlier evidence log is replaced when a new measurement is executed in the same artifact directory.
During an active measurement, evidence is first written to an in-progress file named `.evidence.jsonl.in-progress.<pid>`. Successful completion atomically commits that file to `evidence.jsonl`. A compatible checkpoint resumes by appending to the recorded in-progress evidence file.

Current implication:

- completed evidence is useful for inspection and downstream artifacts
- compatible partial evidence may be resumed by the next non-`--refresh` measurement
- abandoned in-progress evidence files are removed before a fresh run starts
- there is no public replay command that regenerates shape, scorecard, and reports from an existing evidence log

---

## Progress Logs

Every measurement creates a progress log under:

```text
<artifact-dir>/jobs/<job-id>.log
```

The latest job pointer is:

```text
<artifact-dir>/jobs/current
```

Foreground measurements print the job ID and progress-log path when stdout is a terminal. Detached measurements print the job ID, process ID, log paths, status command, and tail command immediately.

The progress log header records the current progress formula:

```text
shown_percent = min(completed / max_probes * 100, 99.0) until complete
```

Example:

```text
529 / 5000 * 100 = 10.58%, logged as 10.6%
```

The final completed line is logged as `100.0% complete ...`.

This means the progress percentage is budget progress, not discovered-command progress. A large CLI can still be actively discovering commands at 10% if `--max-probes` is large.

---

## Detached Jobs

`cliare measure --detach` starts the measurement in the background and returns immediately.

Example:

```sh
cliare measure supabase --out .cliare/supabase --profile deep --max-depth 12 --max-probes 5000 --detach
```

Detached execution currently does the following before spawning the worker:

- resolves the runtime context and artifact directory
- preflights that the target executable exists
- creates `<artifact-dir>/jobs`
- rejects the run if another detached job is active for the same artifact directory
- writes an initial progress log
- creates stdout and stderr log files
- writes `jobs/current`

Detached job files are:

```text
<artifact-dir>/jobs/<job-id>.log
<artifact-dir>/jobs/<job-id>.stdout.log
<artifact-dir>/jobs/<job-id>.stderr.log
<artifact-dir>/jobs/current
```

Detached execution is not resume. It is background execution for a currently running measurement.

---

## Job Status

The implemented status command is:

```sh
cliare jobs status --out <artifact-dir>
```

For context-suite measurements:

```sh
cliare jobs status --out <suite-root> --context authenticated
```

The current status labels are:

| Status | Meaning |
|--------|---------|
| `not_started` | No `jobs/current` pointer exists |
| `starting` | A pointer exists, but no progress or error line is available yet |
| `running` | A progress line exists and the final completion line has not been seen |
| `complete` | The latest progress line contains `100.0% complete` |
| `failed` | The latest progress line contains `failed error=`, or stderr indicates failure before progress |

`jobs status` reads log files and the current pointer. It does not inspect process state directly.

---

## Benchmark Output Lifecycle

The benchmark command has its own output protection.

Current behavior:

- a `.benchmark.lock` file prevents concurrent benchmark writers for the same benchmark output directory
- benchmark aggregate outputs are written through a temporary file and renamed into place
- individual target measurements still use the normal measurement artifact lifecycle

Benchmark locking does not imply general measurement locking for every foreground `cliare measure` invocation. Detached measurement has an active-job guard; foreground users should still avoid running two measurements into the same artifact directory at the same time.

---

## Current Limitations

These capabilities are not implemented today:

- `.cliare/run.json` run manifests
- `.cliare/checkpoints/` multi-phase scheduler, sandbox, inference, or scoring checkpoints
- `cliare measure --resume`
- `cliare measure --new-run`
- `cliare measure --no-cache`
- `cliare measure --max-time`
- `cliare replay`
- `cliare rescore`
- probe-level reuse across runs
- confidence intervals for partial measurements
- public control over checkpoint selection or checkpoint retention
- hosted checkpoint lifecycle

The absence of these public controls is important for operators. If a run is interrupted, rerun `cliare measure` and let it either hit the completed cache, resume a compatible internal checkpoint, or start a fresh measurement. Use `--refresh` when you explicitly want to ignore reusable artifacts and any internal checkpoint.

---

## Remaining Checkpointing Direction

The desired broader model is:

> Evidence remains durable, probe scheduling is checkpointed, and downstream inference/scoring/reporting can be replayed deterministically.

The internal checkpoint implementation covers completed-probe resume for the measurement traversal. Remaining work should add:

- a run manifest with run ID, target fingerprint, profile, budget, status, and artifact references
- pending-frontier and budget-state checkpoints for richer scheduler replay
- stronger evidence-log validation before reuse
- explicit public resume semantics and UX
- replay from evidence into shape, scorecard, reports, and issues
- rescore from existing shape or evidence when only the scoring model changes
- crash-conscious writes for checkpoint files and derived artifacts

Resume behavior should remain conservative:

- resume only when the target fingerprint still matches
- resume only when the traversal profile and runtime context still match
- treat corrupted or incomplete checkpoint state as a cache miss, not as valid evidence
- never silently mix evidence from incompatible binaries, contexts, or probe policies

---

## Operational Guidance

Use the current implementation this way:

| Goal | Command pattern |
|------|-----------------|
| Fast repeated read of a completed result | `cliare measure <target> --out <dir>` |
| Force a fresh probe run | `cliare measure <target> --out <dir> --refresh` |
| Run a large measurement in the background | `cliare measure <target> --out <dir> --detach` |
| Check latest job status | `cliare jobs status --out <dir>` |
| Follow progress | `tail -f <dir>/jobs/<job-id>.log` |

For long-running measurements:

- choose a dedicated artifact directory per target and context
- use `--detach` when the run may outlive the terminal session
- use `cliare jobs status` before starting another detached run into the same directory
- wait for the `100.0% complete` progress line before treating reports as final
- use `--refresh` after changing the target binary, probe profile, or runtime context

For CI:

- use `--refresh` when CI should always measure the current build
- preserve artifacts when you want cache reuse across repeated jobs
- avoid sharing one artifact directory between concurrent jobs
- upload `summary.md`, `scorecard.json`, `issues.json`, `findings.sarif`, and `junit.xml` as CI artifacts
