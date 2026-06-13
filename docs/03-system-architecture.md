# 03 - System Architecture

> **Scope:** Component model, data flow, storage layout, CLI surface, extension points, and execution lifecycle.
> **Status:** Reference Design

---

## Architectural Goal

CLIARE must be able to inspect an arbitrary command-line binary without source access and produce a reproducible scorecard. The design separates expensive runtime probing from cheap inference and scoring.

That separation is central:

```
Probe once. Infer many times. Score many times.
```

If CLIARE improves its scoring model, users should be able to rescore old evidence without rerunning the binary. If a user wants to debug one finding, the raw observation must be available. If the leaderboard changes the public weighting model, historical scorecards can be regenerated from evidence when evidence is available.

---

## Major Components

```
+--------------------+
| CLI Frontend       |
+---------+----------+
          |
          v
+--------------------+
| Run Planner        |
+---------+----------+
          |
          v
+--------------------+       +--------------------+
| Probe Scheduler    +------>+ Sandbox Runtime    |
+---------+----------+       +---------+----------+
          |                            |
          | observations               | process exec
          v                            v
+--------------------+       +--------------------+
| Evidence Recorder  |<------+ Target CLI Binary  |
+---------+----------+       +--------------------+
          |
          v
+--------------------+
| Inference Engine   |
+---------+----------+
          |
          v
+--------------------+
| Shape Catalog      |
+---------+----------+
          |
          v
+--------------------+
| Scoring Engine     |
+---------+----------+
          |
          v
+--------------------+
| Reporters          |
+--------------------+
```

---

## CLI Frontend

The public binary is `cliare`.

Primary commands:

```sh
cliare measure <binary>
cliare infer <binary>
cliare score <shape-or-evidence>
cliare guard <binary> --baseline <scorecard>
cliare certify <binary>
cliare report <scorecard-or-workdir>
cliare rescore <evidence>
cliare publish <scorecard>
```

Secondary commands:

```sh
cliare cache inspect
cliare cache clean
cliare policy validate
cliare schema print command-shape
cliare schema print scorecard
cliare benchmark run
cliare benchmark validate
```

The CLI frontend should be thin. It validates arguments, loads config, initializes the run planner, and writes final artifacts.

---

## Run Planner

The run planner decides what needs to happen for a target binary.

Inputs:

- binary path
- requested mode
- probe profile
- policy file
- cache directory
- previous artifacts
- baseline scorecard
- user-provided workload hints
- allowed environment variables
- sandbox backend

Outputs:

- run manifest
- fingerprint
- probe plan
- cache reuse decision
- artifact destinations

The run planner is also responsible for determining whether previous evidence can be reused.

---

## Fingerprint Model

CLIARE must not recompute blindly, but it must also avoid stale reuse.

Fingerprint fields:

```json
{
  "binary_path": "/repo/dist/mycli",
  "binary_sha256": "...",
  "reported_version": "1.2.3",
  "os": "linux",
  "arch": "x86_64",
  "cliare_version": "0.1.0",
  "probe_profile": "certify",
  "sandbox_policy_hash": "...",
  "environment_allowlist_hash": "...",
  "config_inputs_hash": "...",
  "plugin_fingerprints": [],
  "completion_fingerprints": []
}
```

Cache rule:

```text
same fingerprint + same requested artifact version => reuse
```

But the fingerprint must carry a stability classification:

| Stability | Meaning |
|-----------|---------|
| `binary_only` | Shape is expected to depend primarily on binary content |
| `binary_plus_config` | Config files affect available commands or behavior |
| `plugin_dynamic` | Plugin directories or extension registries affect shape |
| `remote_dynamic` | Remote service state affects commands or outputs |
| `auth_dynamic` | Auth scope changes available commands or outputs |

The scorecard should disclose this classification.

---

## Probe Scheduler

The probe scheduler chooses concrete invocations to run.

It operates in waves:

1. bootstrap probes
2. completion probes
3. help traversal probes
4. invalid input probes
5. grammar confirmation probes
6. output classification probes
7. safety and side-effect probes
8. repeated determinism probes

The scheduler should be adaptive. Each wave can add new candidates.

Example:

```text
Run: mycli --help
Observe: "Commands: project, auth, deploy"
Add probes:
  mycli project --help
  mycli auth --help
  mycli deploy --help
  mycli help project
  mycli project __cliare_unknown__
```

The scheduler must also respect risk:

- safe probes can run automatically
- read probes run only in read profile
- mutating probes require dry-run evidence or fixture profile
- unknown high-risk probes are skipped and recorded

---

## Sandbox Runtime

The sandbox runtime executes every command invocation. It is described in detail in [04-probe-sandbox-runtime.md](04-probe-sandbox-runtime.md).

Minimal responsibilities:

- no shell interpolation
- explicit argv execution
- controlled environment
- fresh temp HOME
- fresh temp cwd
- timeout
- output byte limit
- file diffing
- process metadata
- network policy if backend supports it
- cleanup after run

The sandbox returns an observation:

```json
{
  "probe_id": "p_000142",
  "argv": ["mycli", "project", "list", "--format", "json"],
  "exit_code": 0,
  "stdout": "...",
  "stderr": "",
  "duration_ms": 92,
  "timed_out": false,
  "fs_diff": {},
  "network_events": [],
  "env": { "CI": "1", "NO_COLOR": "1" }
}
```

---

## Evidence Recorder

The evidence recorder writes `evidence.jsonl`.

It must be append-only during a probe run. A crash should never corrupt prior evidence.

Each line is one observation or derived annotation:

- probe scheduled
- process started
- process completed
- stdout/stderr captured
- filesystem diff captured
- network event captured
- classifier annotation
- redaction event
- run checkpoint

Raw output should be stored with size limits and redaction. Large output can be stored separately by content hash.

---

## Inference Engine

The inference engine transforms evidence into claims.

Claims are probabilistic:

```json
{
  "claim_id": "claim.flag.abc",
  "subject": "command:mycli.project.list",
  "predicate": "flag_exists",
  "object": "--format",
  "probability": 0.94,
  "evidence_refs": ["e_001", "e_009", "e_031"],
  "model": "cliare-infer-v1"
}
```

The inference engine produces:

- command candidates
- command tree
- flags
- aliases
- positionals
- value domains
- output classes
- side-effect classes
- exit-code behavior
- contradictions
- unknowns

The engine must be framework-agnostic. Clap, Cobra, Click, argparse, and custom CLIs all enter through the same evidence-to-claim pipeline. Framework detectors may adjust priors, but they must not select a hard-coded truth path. Help text generates candidate claims; runtime confirmation probes update confidence.

Early versions can use rule-based likelihoods and weighted log-odds updates. Later versions can add learned calibration models over the same generic claim representation.

---

## Shape Catalog

The shape catalog is the normalized view of the inferred CLI.

It is not a replacement for evidence. It is a compiled artifact derived from evidence.

Top-level sections:

```json
{
  "schema_version": "cliare.command-shape.v1",
  "target": {},
  "fingerprint": {},
  "commands": [],
  "global_flags": [],
  "environment": [],
  "output_contracts": [],
  "side_effects": [],
  "contradictions": [],
  "coverage": {},
  "model": {}
}
```

Every meaningful field should include:

- confidence
- evidence references
- source type
- model version

---

## Scoring Engine

The scoring engine consumes:

- evidence log
- shape catalog
- optional baseline
- optional policy
- optional workload

It emits:

- total score
- subscores
- confidence intervals
- findings
- deltas
- pass/fail status

Scoring must be deterministic for a given input artifact and model version.

The scorecard must include model versions:

```json
{
  "score_model": "cliare-score-v1",
  "inference_model": "cliare-infer-v1",
  "shape_schema": "cliare.command-shape.v1",
  "evidence_schema": "cliare.evidence.v1"
}
```

---

## Reporters

Reporters generate user-facing outputs.

Required:

- JSON scorecard
- Markdown report
- SARIF report
- JUnit report
- badge JSON

Optional:

- HTML report
- terminal summary
- GitHub Step Summary
- CSV trend export

The Markdown report should be good enough to paste into a PR.

---

## Storage Layout

Default local layout:

```
.cliare/
  run.json
  evidence.jsonl
  shape.json
  scorecard.json
  report.md
  sarif.json
  artifacts/
    stdout/
    stderr/
    traces/
  checkpoints/
    scheduler.json
    inference.json
```

Cache layout:

```
~/.cache/cliare/
  binaries/
    <binary_sha256>/
      <fingerprint_hash>/
        evidence.jsonl
        shape.json
        scorecard.json
```

The project-local `.cliare` directory is the durable CI artifact. The user cache is an optimization.

---

## Configuration

`cliare.yaml`:

```yaml
target:
  binary: ./dist/mycli
  name: mycli

profile: certify

environment:
  allow:
    - MYCLI_TEST_TOKEN
  set:
    CI: "1"
    NO_COLOR: "1"
    TERM: dumb

sandbox:
  network: deny
  timeout_ms: 5000
  output_limit_bytes: 1048576

policy:
  minimum_score: 80
  minimum_subscores:
    safety: 85
    output: 70

workload:
  command_importance:
    "mycli deploy": 2.0
    "mycli project list": 1.5
```

Config is optional. Zero-config should still work.

---

## Extension Points

CLIARE needs extension points without letting plugins compromise the core score.

Extensions:

- framework detectors
- completion adapters
- help parsers
- output classifiers
- sandbox backends
- redactors
- reporters
- policy packs

Each extension should declare:

- name
- version
- input schemas
- output claims
- whether it affects certified score

For the public leaderboard, only approved certified extensions should affect official scores. Local reports can use experimental extensions.

---

## Failure Model

CLIARE must distinguish tool failure from target failure.

| Failure | Meaning | Artifact Behavior |
|---------|---------|-------------------|
| Probe timeout | Target invocation exceeded limit | Evidence recorded, claim uncertainty updated |
| Sandbox error | Runtime failed to execute probe | Run warning, no target claim |
| Redaction failure | Sensitive output could not be safely stored | Raw omitted, metadata retained |
| Inference failure | Model crashed on evidence | Probe artifacts preserved |
| Score failure | Score model cannot evaluate | Shape still emitted |
| Publish failure | Hosted service unavailable | Local scorecard remains valid |

Never throw away evidence because later stages failed.

---

## Security Boundary

The target CLI is untrusted.

Assumptions:

- It may try to read env vars.
- It may write files.
- It may spawn subprocesses.
- It may attempt network calls.
- It may behave differently under CI.
- It may print secrets from inherited config.
- It may hang.

Therefore:

- run with minimal env
- isolate HOME and cwd
- avoid credentials by default
- apply output limits
- enforce timeouts
- deny network by default in certified mode
- redact outputs
- never run through a shell

---

## Initial Reference Architecture

The first implementation can be smaller:

```
cliare measure
  -> bootstrap/help probes
  -> evidence.jsonl
  -> simple command/flag inference
  -> scorecard
  -> markdown report
```

The initial reference implementation does not require:

- remote publishing
- full network tracing
- all shell completion formats
- learned model calibration
- HTML reports
- plugin system

But the artifact model should be correct from day one.
