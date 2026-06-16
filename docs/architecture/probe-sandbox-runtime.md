# 04 - Probe Sandbox Runtime

> **Scope:** How CLIARE executes target CLIs, controls environment state, captures output, and records filesystem side-effect evidence.
> **Status:** Current Implementation

---

## Summary

CLIARE measures a target CLI by running bounded probes and recording their runtime behavior. The execution runtime is part of the measurement contract because command shape, parseability, preconditions, and side effects are only meaningful when tied to a concrete environment.

Default rule:

```text
Run target commands through explicit argv execution, not shell interpolation.
```

The current runtime provides:

- explicit `argv` execution with `tokio::process::Command`
- stdin closed with `Stdio::null()`
- bounded stdout and stderr capture
- per-probe timeout
- isolated environment for default measurements
- optional host execution for authenticated or host-specific measurements
- filesystem snapshots for isolated probe regions
- evidence records for probe scheduling and completion

It does not provide network blocking, network event tracing, subprocess tracing, container execution, native OS sandbox policy, or a public cleanup policy option.

---

## Execution Modes

`measure` and `guard` expose the runtime mode as:

```sh
--execution-mode isolated
--execution-mode host
```

### `isolated`

`isolated` is the default. CLIARE creates an artifact-local sandbox directory and clears the inherited environment before running probes.

Use this mode for normal CLI audits, CI measurements, release checks, command-index generation, and security review where the target should not inherit credentials or host configuration by accident.

### `host`

`host` inherits the host environment and runs from the current directory or the provided context workdir.

Use this mode only when the measurement intentionally depends on authenticated state, local tools, or host-specific configuration. Host mode is useful for understanding a real agent operating context, but it is less isolated and currently does not collect filesystem side-effect diffs because no sandbox regions are registered for host execution.

---

## Traversal Profiles Are Not Sandbox Modes

CLIARE also has traversal profiles:

```sh
--profile quick
--profile standard
--profile deep
```

These profiles control scheduling budgets, not sandbox permissions.

| Profile | Default max depth | Default max probes | Default minimum expected value | Default concurrency |
|---------|-------------------|--------------------|--------------------------------|---------------------|
| `quick` | 3 | 64 | 300 | 2 |
| `standard` | 5 | 256 | 150 | 4 |
| `deep` | 8 | 1000 | 50 | 8 |

Runtime permissions are controlled separately by `--execution-mode` and runtime context flags such as `--context`, `--auth-state`, `--fixture-state`, and `--context-workdir`.

---

## Isolated Environment

In isolated mode, every probe is run with `env_clear()` and a small environment assembled by CLIARE.

The current environment includes:

```text
CI=1
CLIARE=1
HOME=<probe-root>/home
NO_COLOR=1
PWD=<probe-root>/cwd or <context-workdir>
TEMP=<probe-root>/tmp
TERM=dumb
TMP=<probe-root>/tmp
TMPDIR=<probe-root>/tmp
XDG_CACHE_HOME=<probe-root>/xdg-cache
XDG_CONFIG_HOME=<probe-root>/xdg-config
XDG_DATA_HOME=<probe-root>/xdg-data
PATH=<host PATH if present, otherwise /usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin>
LANG=<host LANG if present and non-empty>
LC_ALL=<host LC_ALL if present and non-empty>
```

This is a cleared environment with a narrow allowlist, not a general user-configurable allowlist file. There is no current `cliare.yaml` environment configuration surface.

Important detail: `PATH` is inherited when present so CLIARE can run installed CLIs and their normal helper binaries. Credentials and most host configuration variables are not inherited in isolated mode.

---

## Sandbox Layout

For isolated mode, CLIARE creates sandbox state under the measurement artifact directory:

```text
<artifact-dir>/
  sandbox/
    home/
    cwd/
    tmp/
    xdg-cache/
    xdg-config/
    xdg-data/
    probes/
      <probe-id>/
        home/
        cwd/
        tmp/
        xdg-cache/
        xdg-config/
        xdg-data/
```

The top-level sandbox root is recreated at the start of a fresh isolated measurement. Individual probes run in their own `sandbox/probes/<probe-id>/...` root.

When `--context-workdir <dir>` is supplied, the process working directory is that real directory. CLIARE still uses isolated `HOME`, `TMP`, and XDG paths, but the workdir region points at the provided directory. This mode is useful for measuring CLIs that require a repository or project context, and it should be chosen deliberately because the target can touch that real directory.

There is no `--keep-sandbox` option. Sandbox directories are artifact-local runtime evidence and may remain after a run.

---

## Probe Execution

Each probe becomes:

```text
<target> <probe args...>
```

The process runner:

1. builds the target argv
2. snapshots registered filesystem regions
3. starts the target with explicit args
4. closes stdin
5. pipes stdout and stderr
6. waits up to the configured timeout
7. kills the direct child process on timeout
8. drains stdout and stderr with a short drain timeout
9. snapshots registered filesystem regions again
10. records the diff in evidence

CLIARE records target nonzero exits as evidence. A target CLI returning exit code `1` for a missing operand, auth requirement, or unsupported flag is normally target behavior, not a CLIARE runtime failure.

---

## Bootstrap And Scheduled Probes

The bootstrap probes are intentionally generic:

```text
--help
-h
help
--version
version
<unknown command token>
<unknown flag token>
```

From those observations, the planner schedules additional probes for:

- direct command help, such as `<command> --help`
- alternate help compatibility, such as `help <command path>`
- invalid child probes for confirmed commands with child candidates
- invalid flag probes for confirmed commands
- advertised output-mode probes when required positionals are not unknown
- output-mode help probes

The current planner does not run arbitrary read commands just because their verb looks safe. It also skips output-mode probes when a command requires positional operands that CLIARE cannot safely fill.

---

## Filesystem Side-Effect Evidence

In isolated mode, CLIARE snapshots registered regions before and after each probe.

Tracked regions:

| Region | Meaning |
|--------|---------|
| `home` | isolated probe HOME |
| `workdir` | isolated probe cwd, or the provided context workdir |
| `xdg_config` | isolated XDG config home |
| `xdg_cache` | isolated XDG cache home |
| `xdg_data` | isolated XDG data home |
| `tmp` | isolated temporary directory |

Diff records include:

- change kind: `created`, `modified`, or `deleted`
- region
- path
- size when available
- SHA-256 hash when content hashing is enabled for that region

For the default isolated workdir, content hashes are recorded. For a provided external context workdir, CLIARE uses metadata-based snapshots and does not hash file contents. That avoids reading and hashing arbitrary user project contents while still detecting changes.

Host execution currently registers no snapshot regions, so host-mode side-effect totals are expected to be zero even if the target writes to the host filesystem.

---

## Output Capture

Every probe has separate stdout and stderr byte retention limits controlled by:

```sh
--output-limit-bytes <BYTES>
```

The default is `1048576` bytes per stream.

For each stream, CLIARE records:

- SHA-256 over the full stream bytes read
- total bytes read
- retained bytes
- whether the retained output was truncated
- retained UTF-8 text when the retained bytes are valid UTF-8

The retained text is used for help parsing, diagnostics, precondition classification, and output-contract validation. The hash and byte counts preserve evidence that more output existed even when retained text is truncated.

---

## Timeout Behavior

Every probe has a wall-clock timeout controlled by:

```sh
--timeout-ms <MS>
```

The default is `5000` milliseconds.

If the timeout fires, CLIARE calls `start_kill()` on the direct child process and waits for that child to exit. It does not currently implement process-tree tracking or a configurable kill grace period.

Timeouts are recorded as process status in `evidence.jsonl` and can influence downstream scoring and reports.

---

## Runtime Context Declarations

Runtime context flags do not grant sandbox permissions by themselves. They declare what kind of environment the measurement represents:

```sh
--context clean
--context authenticated
--context local-context
--context fixture
--context custom
--auth-state present|absent|unknown
--local-context-state present|absent|unknown
--fixture-state present|absent|unknown
--network-state present|absent|unknown
--runtime-dependency-state present|absent|unknown
--context-workdir <dir>
```

The context is written to `runtime-context.json` and copied into the run evidence. Reports use this information to explain whether a blocked command likely needs auth, local workspace state, fixture data, network availability, or local runtime dependencies.

---

## Network Behavior

CLIARE does not currently block network access or record network events at the runtime layer.

Network state can still appear in artifacts in two ways:

- the user declares the runtime context with `--network-state`
- the target output is classified as a network precondition, for example when a probe fails because a service is unreachable

These are diagnostic and context signals, not packet-level network traces.

---

## Process Behavior

CLIARE records the process status for the direct target process:

- exited with optional exit code
- timed out
- spawn failed

It does not currently trace subprocesses, record subprocess argv, detect editor/browser/pager launches as child-process events, or enforce OS-level process policy. Some of those behaviors may still be inferred indirectly from stdout, stderr, timeout, or side-effect evidence, but they are not first-class runtime observations today.

---

## Safety Model

CLIARE is conservative about what it schedules. It focuses on help, version, invalid-input diagnostics, command-path confirmation, and output-mode validation. It does not intentionally execute arbitrary mutating business commands.

Side effects from these safe probes are not automatically classified as malicious. They are evidence for maintainers and security reviewers. For example:

- help or version writing config/cache files may require review
- output-mode probes creating files may indicate surprising behavior
- credential-like paths in side-effect evidence are treated as more serious
- provided context workdir changes need explicit review because they touch real project state

The scoring and report layers convert this evidence into findings such as safe-probe side effects or credential-like side effects.

---

## Evidence Contract

`evidence.jsonl` uses schema `cliare.evidence.v1` and records these event kinds:

- `run_started`
- `probe_scheduled`
- `process_completed`
- `run_finished`

`run_started` includes target fingerprint, artifact directory, runtime context, and sandbox metadata.

`probe_scheduled` includes probe id, argv, command path, intent, and sandbox evidence.

`process_completed` includes probe id, argv, status, duration, stdout capture, stderr capture, and side-effect summary.

`run_finished` records the number of completed probes.

Higher-level artifacts such as `shape.json`, `command-index.json`, `scorecard.json`, `issues.json`, and persona reports should remain traceable back to these evidence records.

---

## Failure Model

| Failure | Current behavior |
|---------|------------------|
| Target cannot be spawned | Probe records a spawn failure when possible; measurement can fail if CLIARE cannot execute required runtime work. |
| Target exits nonzero | Recorded as target evidence. |
| Target hangs | Direct child is killed after timeout and the probe is recorded as timed out. |
| Target emits too much output | Full stream hash and byte count are retained; text is truncated to the per-stream limit. |
| Target writes inside isolated regions | Side-effect diff is recorded. |
| Target writes outside isolated regions | Not directly observed unless the path is the provided context workdir or another registered region. |
| Target writes in host mode | Not currently captured as side-effect diffs. |
| CLIARE cannot read/write artifacts | Command fails with a CLIARE error. |

---

## Current Non-Goals

The current runtime does not implement:

- Docker or Podman execution
- macOS `sandbox-exec`, EndpointSecurity, Linux namespace/seccomp, bubblewrap, or Windows Job Object/AppContainer enforcement
- network deny rules or network event capture
- subprocess tracing
- process-tree kill policy
- pseudo-TTY comparison probes
- completion generation probes
- user-configurable environment allowlists
- `--keep-sandbox`
- a public certification sandbox profile

If any of these are added later, update the implementation first, verify the resulting `cliare metadata --format json` and `evidence.jsonl` shape, and then update this document.
