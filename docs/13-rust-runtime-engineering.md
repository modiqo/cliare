# 13 - Rust Runtime Engineering

> **Scope:** Rust implementation strategy for async recursive probing, bounded parallelism, divergence/convergence, memory discipline, deterministic scoring, and world-class error handling.
> **Status:** Draft

---

## Summary

CLIARE should be implemented as a serious Rust runtime, not a loose collection of subprocess calls. The core problem is an adaptive exploration engine:

1. Run probes.
2. Extract new command candidates.
3. Recursively expand promising paths.
4. Confirm grammar with runtime probes.
5. Classify outputs and side effects.
6. Converge evidence into claims.
7. Score deterministically.

This is not a generic graph problem that needs a graph library. The command surface has a specific structure: mostly a rooted command tree with lateral evidence links, contradiction links, and probe dependencies. A purpose-built scheduler over stable IDs and compact arenas is clearer, faster, easier to checkpoint, and easier to audit.

Recommended foundation:

- `tokio` for async process execution, timers, cancellation, and filesystem coordination.
- `tokio::task::JoinSet` for bounded owned task groups.
- `futures::stream::FuturesUnordered` where stream-style completion is more natural than task ownership.
- `indexmap` for deterministic insertion-ordered maps.
- `smallvec` for short argv paths and small evidence reference lists.
- `bytes` for bounded output buffers.
- `serde` plus `schemars` for artifact schemas.
- `thiserror` for library errors.
- `miette` for polished CLI diagnostics.
- `tracing` plus `tracing-error` for structured observability.
- `camino` for UTF-8 paths in artifacts.
- `tempfile` for sandbox roots.
- `nix` or platform-specific crates only behind backend traits.

Avoid dependency on generic graph algorithms in the core runtime. The scheduler should encode CLIARE's own invariants directly.

---

## Design Principle: Domain Scheduler, Not Generic Graph

The probe frontier is a dynamic tree-plus-worklist:

```text
root help
  -> discovered command candidates
      -> subcommand help
          -> flag probes
          -> positional probes
          -> output probes
          -> safety probes
```

The runtime needs:

- stable probe IDs
- dependency tracking
- bounded fanout
- prioritization
- cancellation
- retry policy
- checkpointing
- convergence barriers
- deterministic replay

A generic graph library does not give those semantics. Build a compact domain model:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ProbeId(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CandidateId(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct WaveId(u32);
```

Use arenas:

```rust
pub struct ProbeStore {
    probes: Vec<ProbeNode>,
    by_key: IndexMap<ProbeKey, ProbeId>,
}

pub struct CandidateStore {
    candidates: Vec<CommandCandidate>,
    by_path: IndexMap<ArgvPath, CandidateId>,
}
```

This gives deterministic IDs, compact memory, and simple serialization.

---

## Runtime Pipeline

```
RunPlanner
    |
    v
ProbeScheduler
    |
    | waves of ProbeSpec
    v
ProbeExecutor
    |
    | Observation
    v
EvidenceSink
    |
    | append-only evidence events
    v
OnlineAnnotators
    |
    | discovered candidates / claims
    v
ConvergenceBarriers
    |
    v
InferenceEngine
    |
    v
ScoringEngine
```

The split matters:

- executor runs processes
- evidence sink records facts
- annotators derive candidates
- inference builds posterior claims
- scoring consumes a stable snapshot

Do not let process execution directly mutate final score state.

The inference portion of this pipeline must stay generic. A framework-specific recognizer may emit evidence annotations or priors, but the core runtime should operate on layout blocks, candidate claims, belief updates, and confirmation probes. CLIARE's own use of Clap is a dogfood target, not an internal shortcut.

---

## Async Execution Model

Use bounded parallelism. Deep recursive probing can explode if every discovered command spawns its own subtree immediately.

Core parameters:

```rust
pub struct ProbeBudget {
    pub max_total_probes: usize,
    pub max_in_flight: usize,
    pub max_per_command: usize,
    pub max_depth: usize,
    pub max_duration: Duration,
    pub max_output_bytes: usize,
}
```

Execution loop:

```rust
while !frontier.is_empty() && budget.has_capacity() {
    while join_set.len() < budget.max_in_flight {
        let Some(probe) = frontier.pop_next() else { break };
        join_set.spawn(executor.run(probe));
    }

    let Some(result) = join_set.join_next().await else { break };
    let observation = result??;

    evidence.append_observation(&observation).await?;
    scheduler.ingest(&observation)?;
    scheduler.enqueue_new_work(&mut frontier)?;
}
```

Use `JoinSet` when CLIARE owns the tasks and wants clear cancellation. On shutdown or failure, abort remaining probes and record cancellation events.

Use `FuturesUnordered` for short-lived internal classifier jobs when task spawning is unnecessary.

---

## Divergence and Convergence

CLIARE should explore divergent branches concurrently but converge at explicit barriers.

Recommended waves:

1. `Bootstrap`
2. `CompletionDiscovery`
3. `HelpTraversal`
4. `NegativeSyntax`
5. `GrammarConfirmation`
6. `OutputClassification`
7. `SafetyClassification`
8. `DeterminismRepeats`
9. `FinalInference`
10. `Scoring`

Within a wave, probes run concurrently.

Between waves, converge:

- flush evidence
- update candidate store
- deduplicate probes
- recompute priorities
- checkpoint scheduler
- enforce budget

This gives high concurrency without nondeterministic chaos.

```rust
pub enum ProbeWave {
    Bootstrap,
    CompletionDiscovery,
    HelpTraversal,
    NegativeSyntax,
    GrammarConfirmation,
    OutputClassification,
    SafetyClassification,
    DeterminismRepeats,
}
```

Each wave implements:

```rust
pub trait WavePlanner {
    fn plan(
        &self,
        state: &SchedulerState,
        budget: &RemainingBudget,
    ) -> Result<Vec<ProbeSpec>, PlanError>;
}
```

---

## Frontier Algorithm

Use a priority queue with deterministic tie-breaking.

Priority dimensions:

- expected information gain
- command importance
- risk class
- depth
- confidence gap
- dependency readiness
- budget cost

```rust
pub struct FrontierItem {
    pub priority: OrderedScore,
    pub sequence: u64,
    pub probe_id: ProbeId,
}
```

Use `BinaryHeap` with a custom ordering. Include `sequence` to make tie-breaking deterministic.

Expected information gain can start simple:

```text
priority =
  uncertainty_weight
+ command_importance
+ contradiction_resolution_value
- risk_penalty
- cost_penalty
- depth_penalty
```

No exotic algorithm is needed initially. A well-instrumented, deterministic, budget-aware best-first scheduler is better than a clever opaque planner.

---

## Probe Deduplication

Dedup before scheduling:

```rust
#[derive(Hash, Eq, PartialEq)]
pub struct ProbeKey {
    pub argv: SmallVec<[String; 8]>,
    pub stdin_sha256: Option<[u8; 32]>,
    pub env_policy_hash: [u8; 32],
    pub sandbox_policy_hash: [u8; 32],
    pub fixture_hash: Option<[u8; 32]>,
    pub tty_mode: TtyMode,
}
```

If a probe key exists:

- do not run it again unless it is part of a determinism repeat group
- link new claim requests to existing evidence

This is a major compute saver.

---

## Memory Discipline

Large CLIs can produce huge help output and command trees. Keep the runtime bounded.

Rules:

1. Store raw stdout/stderr as bounded buffers.
2. Spill large output to artifact files by hash.
3. Keep in-memory observations compact.
4. Store evidence as append-only JSONL.
5. Use stable IDs instead of cloning large structures.
6. Use `Arc<str>` or interning for repeated command tokens and flag names if needed.
7. Keep shape inference in a separate replay pass for deep runs.

Recommended structures:

```rust
pub struct ObservationSummary {
    pub probe_id: ProbeId,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub stdout_ref: OutputRef,
    pub stderr_ref: OutputRef,
    pub fs_diff_ref: Option<ArtifactRef>,
}
```

Avoid holding full output strings across the whole run.

---

## Output Capture

Use bounded async reads from child stdout/stderr.

Requirements:

- avoid deadlocks when both stdout and stderr produce data
- enforce byte limits
- preserve truncation markers
- hash full captured bytes
- decode UTF-8 lossy only for display, not hashing

Represent output as bytes first:

```rust
pub struct CapturedOutput {
    pub bytes: Bytes,
    pub sha256: [u8; 32],
    pub truncated: bool,
    pub utf8: Utf8Status,
}
```

Only parse text after capture.

---

## Process Execution Trait

Put process execution behind a trait so sandbox backends are swappable.

```rust
#[async_trait::async_trait]
pub trait ProbeExecutor: Send + Sync {
    async fn execute(&self, probe: ProbeSpec) -> Result<Observation, ExecuteError>;
}
```

Concrete executors:

- `PortableExecutor`
- `ContainerExecutor`
- `LinuxNamespaceExecutor`
- `MacSandboxExecutor`
- `WindowsJobExecutor`

Certified profiles can require specific executor capabilities:

```rust
bitflags::bitflags! {
    pub struct SandboxCapabilities: u32 {
        const TEMP_HOME = 1 << 0;
        const ENV_ISOLATION = 1 << 1;
        const FS_DIFF = 1 << 2;
        const NETWORK_DENY = 1 << 3;
        const PROCESS_TREE_KILL = 1 << 4;
    }
}
```

---

## Error Handling Philosophy

World-class error handling means:

- typed errors in every library crate
- no stringly "other" errors for expected failures
- preserve source chains
- classify target failures separately from CLIARE failures
- record failed probes as evidence when the target failed
- use polished diagnostics at the CLI boundary
- never lose evidence because report generation failed

Core distinction:

```rust
pub enum RunOutcome {
    Complete(Scorecard),
    Partial(PartialScorecard),
    ToolFailed(CliareError),
}
```

A target command exiting nonzero is usually evidence, not a CLIARE error.

---

## Error Taxonomy

Top-level:

```rust
#[derive(Debug, thiserror::Error)]
pub enum CliareError {
    #[error("configuration error")]
    Config(#[from] ConfigError),

    #[error("target fingerprint error")]
    Fingerprint(#[from] FingerprintError),

    #[error("sandbox error")]
    Sandbox(#[from] SandboxError),

    #[error("probe planning error")]
    Planning(#[from] PlanError),

    #[error("evidence storage error")]
    Evidence(#[from] EvidenceError),

    #[error("inference error")]
    Inference(#[from] InferenceError),

    #[error("scoring error")]
    Scoring(#[from] ScoringError),

    #[error("reporting error")]
    Reporting(#[from] ReportingError),
}
```

Target execution result:

```rust
pub enum TargetStatus {
    Exited { code: i32 },
    Signaled { signal: i32 },
    TimedOut,
    SpawnFailed { kind: SpawnFailureKind },
    KilledByPolicy { policy: PolicyViolation },
}
```

`SpawnFailed` can be a CLIARE execution issue or target packaging issue depending on cause. Preserve classification.

---

## Diagnostic Quality

Use `miette` or equivalent rich diagnostics at the CLI boundary.

Example:

```text
CLIARE could not fingerprint target binary

  target: ./dist/mycli
  reason: file is not executable

Help:
  Build the CLI first or pass the path to the executable artifact.
```

For guard failures:

```text
CLIARE guard failed: safety regression exceeds threshold

  total score: 82.1 -> 76.4 (-5.7)
  safety:      88.0 -> 71.2 (-16.8)

Primary finding:
  `mycli env delete` appears destructive and no dry-run evidence was found.

Evidence:
  e_000421: help advertises delete command
  e_000438: command accepts --yes
  e_000447: no --dry-run variant accepted
```

---

## Evidence Durability Under Failure

Every probe should record one of:

- completed
- timed out
- spawn failed
- policy killed
- cancelled

If inference crashes after 900 probes, the 900 observations remain valid.

Use a write-ahead style:

1. append `probe_started`
2. run probe
3. append terminal event
4. flush at wave barrier

At resume, unfinished started probes are marked:

```text
abandoned_due_to_runner_exit
```

Then they can be rescheduled.

---

## Cancellation

Use `CancellationToken` from `tokio-util` for coordinated shutdown.

Scopes:

- whole run
- wave
- individual probe timeout

On Ctrl-C:

1. stop scheduling new probes
2. cancel in-flight probes
3. record cancellation events
4. flush evidence
5. write checkpoint
6. exit with clear message

---

## Retry Policy

Retries should be conservative.

Retry CLIARE infrastructure failures:

- transient artifact write failure
- sandbox setup race
- process kill race

Do not blindly retry target failures:

- usage error
- auth error
- nonzero exit

For nondeterminism measurement, use explicit repeat probes, not hidden retries.

```rust
pub struct RetryPolicy {
    pub max_infra_retries: u8,
    pub backoff: BackoffPolicy,
}
```

Keep retry events visible.

---

## Determinism

Scoring must be deterministic for the same evidence and model version.

Runtime probing can be concurrent and nondeterministic in completion order. Artifacts should normalize this:

- stable event IDs assigned by recorder
- stable probe IDs from scheduler
- deterministic frontier tie-breaking
- sort final commands by argv path
- sort flags by canonical name
- sort findings by severity, impact, stable ID
- include model versions

The score function should be pure:

```rust
pub trait ScoreModel {
    fn score(&self, shape: &CommandShape, evidence: &EvidenceIndex) -> Result<Scorecard, ScoringError>;
}
```

No network, no clock, no random sampling without stored seed.

If posterior sampling is used for confidence intervals, store the RNG seed in the scorecard.

---

## Trait Boundaries

Suggested core traits:

```rust
#[async_trait::async_trait]
pub trait EvidenceSink {
    async fn append(&self, event: EvidenceEvent) -> Result<(), EvidenceError>;
    async fn checkpoint(&self, checkpoint: RunCheckpoint) -> Result<(), EvidenceError>;
}

pub trait EvidenceReader {
    fn events(&self) -> Box<dyn Iterator<Item = Result<EvidenceEvent, EvidenceError>> + '_>;
}

pub trait CandidateExtractor {
    fn extract(&self, observation: &Observation) -> Result<Vec<CandidateDelta>, InferError>;
}

pub trait ProbePlanner {
    fn plan_wave(&self, state: &SchedulerState, wave: ProbeWave) -> Result<Vec<ProbeSpec>, PlanError>;
}

pub trait ClaimModel {
    fn update(&mut self, evidence: &EvidenceEvent) -> Result<(), InferError>;
    fn finalize(self) -> Result<CommandShape, InferError>;
}

pub trait Reporter {
    fn render(&self, scorecard: &Scorecard, shape: &CommandShape) -> Result<ReportArtifact, ReportingError>;
}
```

Keep traits narrow. Avoid a single giant runtime trait.

---

## Recommended Libraries

Use mature, boring libraries where they map directly to the problem:

| Need | Recommendation |
|------|----------------|
| Async runtime | `tokio` |
| Async trait boundary | `async-trait` initially; native async traits later where practical |
| Task cancellation | `tokio-util` `CancellationToken` |
| Concurrent completion stream | `tokio::task::JoinSet`, `futures::stream::FuturesUnordered` |
| Errors | `thiserror` in libraries, `miette` at CLI boundary |
| Structured logs | `tracing`, `tracing-subscriber`, `tracing-error` |
| Serialization | `serde`, `serde_json` |
| JSON Schema generation | `schemars` |
| CLI parser | `clap` |
| Deterministic maps | `indexmap` |
| Small vectors | `smallvec` |
| Bytes | `bytes` |
| Temp dirs | `tempfile` |
| Hashing | `sha2`, possibly `blake3` for internal cache speed |
| Paths | `camino` for artifact-facing UTF-8 paths |
| Time | `time` |
| Regex | `regex` |
| JSON parsing | `serde_json`, `simd-json` only if profiling proves need |
| Property tests | `proptest` |
| Snapshot tests | `insta` |
| Benchmarks | `criterion` |

Avoid early dependency on:

- generic graph libraries
- distributed execution frameworks
- embedded databases
- ML runtimes
- heavyweight actor frameworks

Those can be introduced only after measured need.

---

## Data Structures

Use explicit types:

```rust
pub struct ArgvPath(SmallVec<[Arc<str>; 8]>);
pub struct FlagName(Arc<str>);
pub struct CommandId(u64);
pub struct EvidenceId(u64);
```

Use newtypes for:

- probe IDs
- candidate IDs
- command IDs
- evidence IDs
- score model version
- schema version
- fingerprint hash
- artifact hash

This prevents mixing IDs across stores.

---

## Checkpoint Serialization

Checkpoint state should be plain data:

```rust
#[derive(Serialize, Deserialize)]
pub struct SchedulerCheckpoint {
    pub run_id: RunId,
    pub wave: ProbeWave,
    pub next_probe_id: u64,
    pub completed: Vec<ProbeId>,
    pub pending: Vec<ProbeId>,
    pub candidate_store: CandidateStoreSnapshot,
    pub budget: RemainingBudget,
}
```

Do not serialize live tasks, channels, file handles, or executor internals.

Resume reconstructs runtime from data.

---

## Channels and Backpressure

Use bounded channels only.

```rust
tokio::sync::mpsc::channel::<EvidenceEvent>(1024)
```

Backpressure is good. If evidence writing cannot keep up, probe scheduling should slow down rather than consuming unbounded memory.

Architecture:

```text
probe workers -> bounded evidence channel -> single evidence writer
```

Single writer advantages:

- stable event ordering
- simpler fsync/checkpoint
- no interleaved JSONL writes
- easier corruption handling

---

## Scoring Convergence

Scoring should run after evidence is stable.

For long runs, CLIARE can show progressive estimates:

```text
current estimate: 73.1 [61.0, 82.4]
```

But final score should be computed after a convergence barrier:

1. all in-flight probes finished or cancelled
2. evidence writer flushed
3. shape inference finalized
4. score model reads immutable inputs

Do not update official score concurrently with probing.

---

## Performance Strategy

Initial performance wins:

- bounded parallel subprocesses
- dedup probes
- parse help incrementally
- spill outputs
- avoid cloning strings
- deterministic priority frontier
- skip high-risk low-value branches
- cache by fingerprint
- converge by waves

Only after profiling:

- interning
- compact binary evidence index
- SIMD JSON parse
- parallel inference
- memory-mapped evidence

Use `criterion` for benchmark hot paths.

---

## Testing Runtime Correctness

Critical tests:

- scheduler never exceeds `max_in_flight`
- budget exhaustion yields partial score
- cancelled probes are recorded
- timeout kills process
- stdout/stderr cannot deadlock
- repeated same probe dedups
- deterministic order across runs with same evidence
- checkpoint resume does not duplicate completed probes
- score replay from evidence is identical
- target nonzero exit is evidence, not fatal CLIARE error
- evidence writer survives report failure

Use `proptest` for scheduler invariants:

```text
For any sequence of observations and discoveries:
- scheduled probes are unique unless repeat group
- dependencies are respected
- completed + pending + cancelled covers known scheduled probes
- max depth is never exceeded
- budget is never negative
```

---

## Failure-Proofing Checklist

- [ ] No `unwrap` in runtime paths.
- [ ] No unbounded task spawning.
- [ ] No unbounded channels.
- [ ] No unbounded output buffers.
- [ ] No shell interpolation.
- [ ] Every child process has timeout.
- [ ] Process tree cleanup is best-effort but recorded.
- [ ] Evidence append happens before inference.
- [ ] Target failures are not confused with CLIARE failures.
- [ ] Every public error has remediation.
- [ ] Every artifact has schema version.
- [ ] Every score has model version.
- [ ] Every finding has evidence references.
- [ ] Resume validates fingerprint.
- [ ] Score replay is deterministic.

---

## MVP Runtime Cut

Build the runtime in this order:

1. `cliare-core`: IDs, schemas, artifacts, errors.
2. `cliare-sandbox`: portable executor with timeout/output limits/temp env.
3. `cliare-evidence`: append-only writer and reader.
4. `cliare-scheduler`: wave planner, frontier, dedup, budgets.
5. `cliare-infer`: help parser and basic claim model.
6. `cliare-score`: deterministic score model.
7. `cliare-cli`: commands, diagnostics, reports.

This structure keeps the difficult async runtime isolated from scoring math and from report generation.
