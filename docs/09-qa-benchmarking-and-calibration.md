# 09 - QA, Benchmarking, and Calibration

> **Scope:** How CLIARE itself is tested, how the inference model is calibrated, and how benchmark results become credible.
> **Status:** Draft

---

## Summary

CLIARE is a measurement tool. Its own correctness matters. If it produces scores that are noisy, uncalibrated, or easy to game, it will not become a standard.

QA has three layers:

1. **Implementation QA**
   - unit tests
   - integration tests
   - sandbox tests
   - schema compatibility tests

2. **Inference QA**
   - compare inferred claims against known truth
   - use proper scoring rules
   - calibrate probabilities

3. **Scoring QA**
   - verify monotonic improvements
   - verify regression detection
   - verify score stability under repeated runs

---

## Benchmark Corpus

CLIARE needs both synthetic and real-world CLIs.

### Synthetic CLIs

Small purpose-built binaries with known truth.

Categories:

- perfect CLI
- poor help CLI
- completion-only CLI
- help-only CLI
- hidden commands
- inconsistent help/runtime
- ambiguous arity
- enum flags
- variadic positionals
- JSON output
- mixed output
- interactive prompt
- dry-run support
- destructive command without mitigation
- plugin commands
- nondeterministic output
- auth-required commands

Synthetic CLIs should be cheap to run and portable.

Current fixture coverage includes executable black-box CLIs for:

- custom aligned help with multi-token commands
- simple comma-separated aliases
- noisy stderr during otherwise valid help
- help rows that look command-like but are runtime false positives

These fixtures exercise the full measurement path: fingerprinting, process execution, evidence logging, claim updates, planner expansion, and shape emission.

### Real-World CLIs

Public tools with human-verified truth subsets.

Candidates:

- git
- npm
- docker
- kubectl
- gh
- terraform
- cargo
- pip
- aws
- gcloud

Real-world truth sets do not need full coverage at first. They can focus on representative command families.

### Fixture CLIs

Generated CLIs across frameworks:

- clap
- Cobra
- Click
- argparse
- Typer
- oclif
- yargs

These are useful for framework fingerprinting and completion behavior.

---

## Ground Truth Format

Ground truth should use the same command-shape vocabulary, but with certainty.

```json
{
  "schema_version": "cliare.ground-truth.v1",
  "target": "fixture-clap-basic",
  "commands": [
    {
      "argv": ["fixture", "project", "list"],
      "exists": true,
      "flags": [
        {
          "name": "--format",
          "arity": "one",
          "values": ["json", "table"],
          "required": false
        }
      ],
      "output": {
        "kind": "json",
        "when": ["--format", "json"]
      },
      "side_effects": "read"
    }
  ]
}
```

Ground truth should include negative facts:

```json
{
  "command": ["fixture", "delete"],
  "dry_run_supported": false
}
```

Negative facts are essential for calibration.

---

## Inference Metrics

For binary claims:

- accuracy
- precision
- recall
- Brier score
- log loss
- calibration error

For categorical claims:

- top-1 accuracy
- top-3 accuracy
- categorical log loss
- expected calibration error

For command discovery:

- observed recall against ground truth
- estimated coverage error
- false discovered commands

For grammar:

- flag existence F1
- arity accuracy
- positional requiredness accuracy
- enum extraction accuracy

For safety:

- write/destructive recall
- false safe rate
- dry-run detection accuracy

Safety should bias toward avoiding false safe classifications.

---

## Score Stability Tests

Run same target multiple times.

Metrics:

- total score standard deviation
- subscore standard deviation
- claim probability drift
- nondeterministic findings

A stable CLI under stable sandbox should produce stable scores.

Acceptable thresholds:

```text
Total score stddev < 1.0 for deterministic fixtures
Subscore stddev < 2.0 for deterministic fixtures
No pass/fail flapping in guard mode
```

---

## Monotonicity Tests

Create paired fixtures:

1. bad CLI
2. improved CLI

Improvement examples:

- add root help
- add completion
- add JSON output
- add dry-run
- add suggestions
- fix help/runtime mismatch
- stabilize exit codes

Expected:

```text
Improved score >= original score
Relevant subscore strictly increases
Unrelated subscores do not change materially
```

Exception tests:

- improvement also reveals hidden destructive command
- surface expansion adds poor commands

These should be classified as revealed risk or surface growth, not failed monotonicity.

---

## Regression Tests

Create paired fixtures where behavior gets worse:

- remove `--json`
- remove dry-run
- make valid command fail
- add color to JSON output
- break completion
- remove error suggestions
- change flag arity

Expected:

- score decreases
- correct finding appears
- guard mode fails when policy threshold is crossed

---

## Sandbox QA

Sandbox tests:

- temp HOME is used
- host env is denied
- env allowlist works
- file writes are captured
- timeouts kill process tree
- output limit truncates safely
- nonzero exit recorded
- signals recorded
- cleanup removes sandbox
- keep-sandbox preserves on failure

Network tests where backend supports it:

- network denied
- network attempt recorded
- local stub allowed

---

## Schema QA

Schema tests:

- every artifact validates against JSON Schema
- old artifacts remain readable
- unknown fields are preserved or ignored appropriately
- scorecard includes model versions
- evidence references resolve
- shape references evidence IDs that exist
- redacted artifacts remain valid

Compatibility tests:

- v1 reader accepts v1.1 additive fields
- v1 reader rejects v2 breaking schema clearly

---

## Report QA

Report tests:

- top findings are sorted by impact/severity
- every finding has evidence references
- recommendations are actionable
- Markdown renders
- SARIF validates
- JUnit validates
- CI summary fits size limits

Golden tests can compare report snapshots, but should avoid overfitting to prose.

---

## Leaderboard QA

Leaderboard ingestion tests:

- valid scorecard accepted
- invalid schema rejected
- unsupported score model rejected or quarantined
- duplicate release handled idempotently
- attestation verified
- verification level assigned correctly
- evidence hash mismatch rejected

Anti-gaming checks:

- scorecard cannot claim higher verification than provenance supports
- model version cannot be omitted
- old score model displayed separately

---

## Calibration Workflow

1. Run CLIARE against benchmark corpus.
2. Extract predicted claims.
3. Compare against ground truth.
4. Compute proper scoring metrics.
5. Update likelihood weights.
6. Re-run.
7. Publish calibration report.

Calibration report:

```text
Model: cliare-infer-v1
Corpus: cliare-bench-2026-06

Command existence:
  Brier: 0.031
  ECE: 0.042

Flag arity:
  Accuracy: 0.91
  LogLoss: 0.28

Side-effect class:
  Top-1: 0.84
  False-safe rate: 0.02
```

False-safe rate should be a headline safety metric.

---

## QA Matrix

| Area | Test Type | Required For MVP |
|------|-----------|------------------|
| Evidence schema | unit/schema | yes |
| Shape schema | unit/schema | yes |
| Sandbox env isolation | integration | yes |
| Help parsing | fixture | yes |
| Flag inference | fixture | yes |
| Output classification | fixture | yes |
| Safety classification | fixture | yes |
| Bayesian calibration | benchmark | partial |
| CI guard | integration | yes |
| Publish/leaderboard | service | later |

---

## Release Criteria

### Alpha

- runs locally
- emits evidence, shape, scorecard, report
- handles simple synthetic CLIs
- no public leaderboard score claims

### Beta

- GitHub Action
- baseline guard
- synthetic benchmark corpus
- initial calibration report
- public experimental score

### v1 Standard

- stable schemas
- calibrated score model
- public leaderboard
- verification levels
- real-world benchmark set
- documented governance for model changes
