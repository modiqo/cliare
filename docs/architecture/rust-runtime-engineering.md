# 13 - Rust Runtime Engineering

> **Scope:** Current Rust implementation architecture, bounded runtime behavior, deterministic probing, artifact writing, and future engineering boundaries.
> **Status:** Current implementation reference plus future direction.

---

## Summary

CLIARE is currently implemented as a single Rust crate with a Tokio-based CLI runtime. It is not a multi-crate workspace, not a generic graph engine, and not a distributed executor.

The current measurement path is:

```text
CLI args
  -> target fingerprint
  -> runtime context
  -> cache check
  -> sandbox setup
  -> deterministic planner
  -> bounded probe rounds
  -> evidence.jsonl
  -> claims and shape
  -> command index
  -> scorecard
  -> reports, issues, SARIF, JUnit, guides
  -> measure-cache.json
```

The implementation favors explicit data structures, deterministic ordering, typed errors, bounded subprocess execution, and reviewable JSON/Markdown artifacts.

---

## Actual Dependency Set

The current crate dependencies are intentionally small:

| Need | Current dependency |
|------|--------------------|
| CLI parsing | `clap` |
| CLI diagnostics boundary | `miette` |
| Serialization | `serde`, `serde_json` |
| Hashing | `sha2` |
| Typed errors | `thiserror` |
| Time formatting | `time` |
| Async runtime, process, filesystem, timers | `tokio` |

The current crate does not depend on:

- `async-trait`
- `bytes`
- `camino`
- `criterion`
- `futures`
- `indexmap`
- `insta`
- `proptest`
- `schemars`
- `smallvec`
- `tempfile`
- `tokio-util`
- `tracing`
- platform sandbox crates such as `nix`
- generic graph libraries
- actor frameworks
- embedded databases
- ML runtimes

Those libraries may become useful later, but they are not part of the current implementation and should not be documented as current runtime foundation.

---

## Crate Layout

The repository currently uses one crate with focused modules:

| Area | Modules |
|------|---------|
| CLI surface | `cli`, `command_spec`, `main` |
| Measurement loop | `measure`, `planner`, `process`, `sandbox`, `fingerprint`, `evidence` |
| Inference and shape | `layout`, `layout_tokens`, `layout_usage`, `claims`, `observation`, `shape`, `output`, `diagnostic`, `precondition` |
| Scoring | `score`, `score_model`, `belief`, `policy` |
| Reports and issues | `report`, `report_model`, `report_markdown`, `report_evidence`, `issues`, `issue_disposition`, `ci` |
| Operations | `jobs`, `benchmark`, `context`, `describe`, `artifact_guide`, `skills`, `playbook` |
| Shared contracts | `artifacts`, `error`, `markdown`, `path_classification` |

This is factual for the current implementation. A future split into `cliare-core`, `cliare-sandbox`, or similar crates should be treated as a refactor decision, not as shipped architecture.

---

## Current Measurement Lifecycle

`measure::measure` owns the end-to-end measurement flow:

1. Build a `RuntimeContext` from CLI options.
2. Resolve the artifact directory.
3. Fingerprint the target executable.
4. Resolve profile values: depth, probe budget, expected-value threshold, and concurrency.
5. Build a `ProbeProfile` for cache identity.
6. Reuse `measure-cache.json` when `--refresh` is not set and all cache checks pass.
7. Create a progress log under `<artifact-dir>/jobs`.
8. Write `runtime-context.json`.
9. Create an isolated or host sandbox.
10. Create a fresh `evidence.jsonl`.
11. Seed a deterministic planner with bootstrap probes.
12. Run bounded traversal rounds.
13. Build `shape.json` and command-index artifacts from observations.
14. Score the measurement.
15. Write reports, issues, SARIF, JUnit, persona packets, and guides.
16. Write `measure-cache.json`.

Target command failures are usually evidence. CLIARE treats process exit codes, stderr, timeouts, parse failures, and side effects as observations unless CLIARE itself failed to run or persist the measurement.

---

## Planner

The current planner is `planner::DeterministicPlanner`.

It uses:

- `VecDeque<ProbePlan>` for the frontier
- `BTreeSet<Vec<String>>` to deduplicate scheduled argv suffixes
- `BTreeMap` when joining output contracts to command claims
- deterministic sorting before queue insertion
- depth limits
- expected-value thresholding through `ConvergencePolicy`

The planner seeds bootstrap probes, then extends from the current `ClaimSet`.

Probe categories are ordered:

1. help confirmation
2. diagnostics
3. output probes

Ranking uses deterministic fields:

- category
- expected value
- uncertainty
- confidence
- depth
- intent order
- argv suffix as a final stable tie-breaker

Current deduplication is argv-suffix based. It does not yet hash stdin, environment policy, sandbox policy, fixture state, or TTY mode into a richer probe key.

---

## Traversal Loop

`measure::run_traversal` runs the current adaptive traversal.

The loop is intentionally bounded:

- no unbounded task spawning
- no unbounded channel
- no recursive task tree
- no per-command worker pool
- no background inference task

Per round:

1. Pull up to `concurrency_limit` probes from the planner.
2. Create a probe-specific sandbox execution directory.
3. Append `probe_scheduled` evidence.
4. Spawn one Tokio task per probe with `tokio::spawn`.
5. Await the handles in the order they were scheduled.
6. Append `process_completed` evidence.
7. Convert observations into a fresh `ClaimSet`.
8. Extend the planner from claims.
9. Continue until the frontier is empty or `max_probes` is reached.

This model is simpler than a wave/checkpoint scheduler. It gives bounded concurrency and deterministic evidence commit order, while still allowing process execution inside a round to happen concurrently.

Current limitation:

- If an earlier scheduled probe in a round is slow, later completed handles are not committed until the loop awaits them in schedule order.
- This favors deterministic event order over earliest-completion streaming.
- If a probe task returns a CLIARE execution error, the traversal records a progress failure after the round and returns that error. Successful probes from the same round can still be committed before the failure is returned.

---

## Process Execution

`process::TargetProcess` owns target process execution.

Current behavior:

- `tokio::process::Command`
- explicit target executable plus argv suffix
- `stdin` set to null
- `stdout` and `stderr` piped
- `env_clear`
- sandbox-controlled environment
- sandbox-controlled cwd
- per-probe timeout through `tokio::time::timeout`
- best-effort child kill on timeout through `start_kill`
- before/after sandbox snapshots for side-effect detection

Output capture:

- stdout and stderr are drained concurrently in separate Tokio tasks
- retained bytes are capped by `--output-limit-bytes`
- total bytes and retained bytes are recorded separately
- SHA-256 is computed over all bytes read
- truncated output is marked
- retained bytes are decoded as UTF-8 only when possible
- stream-drain collection has a one-second fallback timeout; abandoned output is marked truncated

Current process-completed evidence supports:

- exited with optional code
- timed out

The evidence data model includes a `spawn_failed` status variant, but the current process runner returns `CliareError::Spawn` when `Command::spawn` itself fails, before a `process_completed` event is constructed. The current implementation also does not record Unix signals, policy-killed status, process-tree cleanup details, or retry events.

---

## Sandbox Runtime

`sandbox::Sandbox` supports two execution profiles:

| Profile | Behavior |
|---------|----------|
| `isolated` | Creates sandbox directories under the artifact directory and clears the target environment to a CLIARE-controlled allowlist |
| `host` | Uses host home/config/cache/data/temp and optional context workdir for authenticated or local-context measurements |

In isolated mode CLIARE creates:

- sandbox root
- HOME
- cwd
- XDG config/cache/data
- temp

Each probe receives its own execution directories under:

```text
<artifact-dir>/sandbox/probes/<probe-id>/
```

This prevents concurrent probes from contaminating each other's side-effect snapshots.

Side-effect detection:

- snapshots configured sandbox regions before and after each probe
- records created, modified, and deleted files
- hashes files where possible
- classifies credential-like paths later for safety reporting and policy checks

Current limitations:

- No OS-level syscall tracing.
- No portable network-deny enforcement.
- No Linux namespace, macOS sandbox, Windows Job Object, Docker, or container backend.
- Host mode intentionally exposes host context and should be used only when that is the measurement goal.

---

## Evidence Writer

`evidence::EvidenceWriter` writes:

```text
<artifact-dir>/evidence.jsonl
```

Current behavior:

- creates the artifact directory
- opens an in-progress evidence file for a fresh run
- appends to the checkpointed in-progress evidence file for a compatible resumed run
- starts event IDs at `e_000001`
- writes line-delimited JSON
- flushes after each event
- atomically commits the completed evidence log to `evidence.jsonl`

Current event kinds:

- `run_started`
- `probe_scheduled`
- `process_completed`
- `run_finished`

Current limitation:

- Resume is internal; there is no public `measure --resume` checkpoint selector.
- In-flight probes from an interrupted run are not trusted as completed evidence and may be re-run.
- There is no public replay command.
- If a later report-writing step fails, evidence written so far remains on disk, but replaying downstream artifacts from that evidence is not a public command.

---

## Artifact Writing

Measurement writes a set of JSON, Markdown, SARIF, XML, and guide artifacts after traversal.

Important current behavior:

- `measure-cache.json` is written after a successful fresh measurement.
- Cache hits regenerate lightweight derivative artifacts such as runtime context, persona reports, measurement guides, and context-suite metadata.
- Some modules use atomic temp-file-plus-rename writes, including benchmark aggregate outputs and measurement guides.
- Many measurement artifacts are currently written directly with `tokio::fs::write` or equivalent module writers.

Do not claim that every artifact is atomically written today.

---

## Measurement Cache

Current cache file:

```text
<artifact-dir>/measure-cache.json
```

Current cache identity includes:

- cache schema version
- CLIARE package version
- measurement engine label
- run ID
- target fingerprint
- probe profile
- runtime context embedded in the profile
- SHA-256 digests and sizes for required measurement artifacts

Cache reuse also requires the required measurement artifacts to exist and match the manifest digests:

- `evidence.jsonl`
- `shape.json`
- `command-index.json`
- `command-index.md`
- `scorecard.json`
- `report.md`
- `summary.md`
- `findings.sarif`
- `junit.xml`

`--refresh` bypasses the cache.

This is completed-artifact reuse. It is not probe-level memoization across different runs.

---

## Detached Jobs

`jobs::spawn_detached_measure` implements `cliare measure --detach`.

Current behavior:

- resolves runtime context and artifact directory
- preflights the target executable before spawning
- creates `<artifact-dir>/jobs`
- rejects a new detached run when `jobs/current` points to an active detached job
- writes initial progress log, stdout log, stderr log, and `jobs/current`
- spawns the current CLIARE executable with internal detached-worker flags
- starts the worker in a separate process group on Unix

`cliare jobs status` reads `jobs/current`, progress logs, and stderr logs. It reports:

- `not_started`
- `starting`
- `running`
- `complete`
- `failed`

It does not inspect process state directly.

Detached jobs are background execution, not resume.

---

## Benchmark Runtime

`benchmark::benchmark` adds a target-level concurrency layer above normal measurement.

Current behavior:

- reads a corpus manifest
- validates schema and target options
- uses a bounded target-concurrency setting
- runs target measurements into per-target artifact directories
- treats required target failures as benchmark failures
- skips optional targets only when binaries are missing
- writes aggregate `benchmark.json` and `benchmark.md`
- uses a `.benchmark.lock` file to avoid concurrent benchmark writers for one output directory
- writes aggregate benchmark outputs with temp-file-plus-rename

The benchmark coordinator owns aggregate writes. Target workers write only their own measurement directories.

---

## Scoring Determinism

The current score path is deterministic for the same artifacts, score model, target behavior, and runtime context.

Current scoring facts:

- bundled model JSON is loaded through `score_model`
- score model schema is validated
- scoring dimensions include discovery, grammar, execution, output, safety, and recovery
- inference weights and score weights are model-backed rather than scattered constants
- scorecards include coverage and model metadata
- findings are generated from observed evidence and score facts

Runtime probing can still vary when the target CLI itself is nondeterministic, slow, auth-dependent, or network-backed. CLIARE exposes traversal completion, probe counts, budget exhaustion, preconditions, output parse results, and side effects so operators can judge the run.

---

## Error Handling

The crate uses one typed `CliareError` enum with `thiserror`.

Current error handling style:

- expected CLIARE failures have typed variants
- variants preserve source errors where useful
- CLI boundary converts errors through `miette::IntoDiagnostic`
- `guard` and `benchmark` convert failed checks into user-facing `miette` errors
- target process nonzero exits are evidence, not `CliareError`
- target timeouts are captured as process evidence
- target process spawn errors currently fail traversal as `CliareError::Spawn` after the probe has been scheduled
- inability to fingerprint, create artifacts, write evidence, parse required artifacts, or spawn detached jobs is a CLIARE error

The implementation does not currently use a layered error enum per subsystem. That may be useful later, but the current single-error enum is factual and auditable.

---

## Bounded Resource Rules

Current bounded behavior:

- `--max-probes` limits total scheduled probes.
- `--max-depth` limits recursive command-path exploration.
- `--min-expected-value` prunes low-value dynamic probes.
- `--concurrency` limits simultaneous probes in a traversal round.
- `--timeout-ms` limits each target process.
- `--output-limit-bytes` limits retained stdout and stderr.
- sandbox snapshots are scoped to known sandbox regions.
- benchmark target concurrency is separately bounded.

Current gaps:

- no global wall-clock `--max-time`
- no bounded evidence channel because evidence is written synchronously from the traversal loop
- no process-tree kill guarantee beyond best-effort child kill
- no memory-mapped or spilled large-output artifact store; retained output is bounded inline

---

## Testing Coverage

The current test suite includes unit and integration-style coverage across:

- CLI parsing and metadata command spec
- target fingerprinting
- sandbox isolation and side-effect snapshots
- bounded output capture and hashing
- deterministic planner ordering, depth limits, convergence thresholding, and output probe gating
- evidence serialization
- scoring behavior and issue generation
- policy checks
- guard pass/fail behavior
- job status classification and detached active-job guard
- context-suite resolution
- benchmark reporting and locking
- issue dispositions
- persona reports and playbooks

Current test dependencies are standard Rust test support plus Tokio async tests. The crate does not currently use property-testing, snapshot-testing, or criterion benchmark crates.

---

## Current Engineering Invariants

The implementation should preserve these invariants:

- Target CLI failures are evidence unless CLIARE itself cannot execute or persist the run.
- Probe scheduling is deterministic for the same claims and profile.
- Probe execution is bounded by depth, probe budget, concurrency, timeout, and output-retention limits.
- Evidence event IDs are stable within one fresh run.
- Process evidence is committed in scheduled order for each round.
- Safe output probes are not run when required positionals would require guessed operands.
- Isolated probes use separate per-probe sandbox directories.
- Cache hits require matching target fingerprint, profile, package version, engine label, and required artifacts.
- Detached runs preflight the target and refuse another active detached job in the same artifact directory.
- Public reports should not imply certification or leaderboard authority.

---

## Future Engineering Direction

These are future improvements, not current implementation:

- split the crate into smaller internal crates once module boundaries stabilize
- richer probe keys that include env policy, sandbox policy, fixture state, stdin hash, and TTY mode
- public `measure --resume` controls
- replay existing evidence into shape, command index, reports, and scorecards
- rescore existing artifacts with a new model version
- append-only evidence replay with stronger validation
- OS-specific sandbox backends
- network policy enforcement
- process-tree cleanup guarantees
- structured tracing
- property tests for planner invariants
- snapshot tests for reports
- criterion benchmarks for hot paths

Each future dependency or abstraction should have a measured need and a short design note before being added.

---

## Review Checklist

Before changing runtime code, check:

- Does this keep probe count, depth, timeout, output, and concurrency bounded?
- Does this preserve deterministic planner ordering?
- Does this treat target failures as evidence?
- Does this avoid guessing required operands?
- Does this avoid adding unbounded queues or task spawning?
- Does this keep artifact formats versioned?
- Does this preserve source errors in `CliareError`?
- Does this update docs when behavior or command output changes?
- Does this add tests for planner, sandbox, evidence, scoring, or report behavior as appropriate?
