# 10 - Checkpointing and Resume

> **Scope:** Long-running analysis, crash recovery, resumable probing, cache invalidation, replayability, and artifact lifecycle.
> **Status:** Reference Design

---

## Summary

Some CLIs are small and can be measured in seconds. Others may have hundreds of commands, plugins, slow startup, network behavior, or fixture setup. CLIARE needs checkpointing from the beginning.

The principle:

> Evidence is append-only. All later stages are replayable.

If a run crashes halfway through, existing evidence remains useful. If inference improves, old evidence can be replayed. If scoring changes, old shape catalogs can be rescored.

---

## Run Lifecycle

```
initialize
  |
  v
fingerprint target
  |
  v
load cache / prior checkpoint
  |
  v
plan probes
  |
  v
execute probe wave
  |
  v
write checkpoint
  |
  v
repeat until budget exhausted or complete
  |
  v
infer shape
  |
  v
score
  |
  v
report
```

---

## Run Manifest

File:

```
.cliare/run.json
```

Example:

```json
{
  "schema_version": "cliare.run.v1",
  "run_id": "run_20260613_abc",
  "target": {
    "binary": "./dist/mycli",
    "name": "mycli"
  },
  "fingerprint": {},
  "profile": "certify",
  "started_at": "2026-06-13T00:00:00Z",
  "status": "running",
  "budgets": {
    "max_probes": 1000,
    "max_duration_seconds": 600
  },
  "artifacts": {
    "evidence": ".cliare/evidence.jsonl",
    "shape": ".cliare/shape.json",
    "scorecard": ".cliare/scorecard.json"
  }
}
```

---

## Checkpoint Files

```
.cliare/checkpoints/
  scheduler.json
  sandbox.json
  inference.json
  scoring.json
```

Scheduler checkpoint:

```json
{
  "completed_probes": ["p_000001", "p_000002"],
  "pending_probes": ["p_000003", "p_000004"],
  "discovered_candidates": [],
  "probe_budget_remaining": 842,
  "wave": "help_traversal"
}
```

Inference checkpoint:

```json
{
  "last_evidence_event": "e_000912",
  "claim_store_hash": "...",
  "model": "cliare-infer-v1"
}
```

---

## Append-Only Evidence

Evidence must be written so partial runs are valid.

Rules:

- line-delimited JSON
- fsync or durable flush at checkpoint boundaries
- each event has unique ID
- events are immutable
- corrections are new events, not edits

If a classifier changes its mind:

```json
{
  "kind": "claim_superseded",
  "payload": {
    "old_claim": "claim_001",
    "new_claim": "claim_104",
    "reason": "new runtime evidence contradicted help-only inference"
  }
}
```

---

## Resume Semantics

Command:

```sh
cliare measure ./mycli --resume
```

Behavior:

1. Load run manifest.
2. Verify fingerprint still matches.
3. Validate evidence log integrity.
4. Load scheduler checkpoint.
5. Skip completed probes.
6. Continue from pending probes.

If fingerprint changed:

```text
Target fingerprint changed. Cannot resume safely.
Use --new-run or --reuse-evidence=unsafe to override.
```

---

## Budgets

CLIARE should support explicit budgets:

```sh
cliare measure ./mycli --max-probes 500 --max-time 5m
```

Budget exhaustion is not failure. It produces a partial score with wider confidence intervals.

Scorecard should disclose:

```json
{
  "coverage": {
    "budget_exhausted": true,
    "probes_completed": 500,
    "probes_pending": 230,
    "confidence_penalty": 4.2
  }
}
```

---

## Probe Deduplication

Many candidate probes can be equivalent.

Dedup key:

```text
argv + stdin hash + env policy hash + cwd fixture hash + sandbox policy hash
```

If the same probe has already run under the same conditions, reuse observation.

Repeated probes for determinism should declare repeat intent:

```json
{
  "repeat_group": "determinism.output.project_list",
  "repeat_index": 2
}
```

---

## Artifact Hashing

Every artifact should have a hash.

```json
{
  "artifact_hashes": {
    "evidence_sha256": "...",
    "shape_sha256": "...",
    "scorecard_sha256": "...",
    "report_sha256": "..."
  }
}
```

Hashing supports:

- cache reuse
- leaderboard submission
- provenance
- integrity checks
- reproducibility

---

## Cache Strategy

Cache expensive outputs:

- evidence logs by fingerprint
- completion scripts by binary/version
- parsed help pages
- inferred shape

Do not cache blindly:

- auth profile outputs
- remote dynamic outputs
- fixture-dependent outputs unless fixture hash matches

Cache entry:

```json
{
  "fingerprint_hash": "...",
  "created_at": "...",
  "profile": "safe",
  "stability": "binary_only",
  "evidence": "evidence.jsonl",
  "shape": "shape.json",
  "scorecard": "scorecard.json"
}
```

---

## Recompute Policy

When to recompute probes:

- binary hash changed
- reported version changed
- CLIARE probe profile changed
- sandbox policy changed
- environment allowlist changed
- plugin hash changed
- config input hash changed
- user requested `--no-cache`

When not to recompute:

- only score model changed
- only report template changed
- only policy thresholds changed

In those cases:

```sh
cliare rescore .cliare/evidence.jsonl
cliare report .cliare/scorecard.json
```

---

## Checkpointing In CI

CI usually runs from scratch, but checkpointing still helps:

- job retries
- matrix jobs
- large CLIs
- nightly deep scans
- artifact reuse

Recommended:

- PR: limited budget, guard mode
- main: larger budget
- release: certify profile
- nightly: deep profile with extended budget

---

## Partial Results

CLIARE should represent partial results explicitly.

Report:

```text
Analysis completed with partial coverage.

Completed probes: 500 / estimated 730
Discovery confidence: medium
Score interval: 71.2 to 84.8
Reason: probe budget exhausted during grammar confirmation wave
```

Partial scores can be useful but should not receive the same verification level as complete certified runs.

---

## Crash Recovery

On startup with an existing `.cliare/run.json` marked `running`:

```text
Previous CLIARE run did not finish.
Options:
  --resume       continue it
  --new-run      archive it and start over
  --inspect      show status
```

In CI, default should be `--new-run` unless explicit cache restore is configured.

---

## Replay

Replay command:

```sh
cliare replay .cliare/evidence.jsonl
```

Stages:

- validate evidence
- regenerate annotations
- regenerate shape
- regenerate scorecard
- regenerate report

Replay is essential for:

- debugging
- model development
- calibration
- reproducibility
- leaderboard rescoring

---

## Artifact Lifecycle

Recommended lifecycle:

| Artifact | Local | CI | Hosted |
|----------|-------|----|--------|
| evidence.jsonl | keep | upload optional | optional redacted |
| shape.json | keep | upload | optional |
| scorecard.json | keep | upload | required for leaderboard |
| report.md | keep | upload | rendered |
| raw stdout/stderr | debug only | optional | private only |

---

## Initial Checkpointing Scope

The initial checkpointing implementation should provide:

- run manifest
- append-only evidence
- scheduler checkpoint
- resume by fingerprint
- rescore from evidence
- artifact hashes

The initial checkpointing implementation can defer:

- distributed probe execution
- advanced cache eviction
- partial artifact uploads
- long-term hosted evidence storage
