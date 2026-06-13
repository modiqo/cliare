# CLIARE Progress Scorecard

> Last updated: after checkpoint `feat: emit ci artifacts`

This scorecard tracks implementation progress for the reference CLIARE runner. It is not the public CLI readiness score model; it is the project delivery scorecard for the MVP.

---

## Current Status

| Area | Status | Notes |
|---|---:|---|
| Repository and package foundation | 100% | Rust project, license, README, design docs, private GitHub repo |
| CLI surface | 60% | `measure`, `guard`, traversal profiles, cache bypass, CI artifacts; baseline/publish/certify still planned |
| Runtime probing | 68% | Safe bootstrap, bounded output, timeouts, recursive probes, isolated HOME/PWD/XDG config-cache-data/TMP, sanitized env, per-probe filesystem diffs |
| Generic inference | 58% | Layout claims, runtime confirmation, Bayesian confidence, usage positionals, aliases, flag grammar, and output-mode claims exist; value domains still planned |
| Command shape artifact | 68% | Commands, aliases, positionals, flags, flag arity, output contracts, gaps, confidence, and evidence references exist; richer value domains still planned |
| Scoring | 58% | v0 dimensions for discovery, grammar, execution, recovery, output, and initial safety; determinism still planned |
| CI guard | 70% | Baseline comparison, SARIF, JUnit, Markdown CI summary, and GitHub Action wrapper exist; policy files still planned |
| Cache and fingerprinting | 62% | Binary/profile/version/sandbox-profile/artifact-set cache reuse exists; replay/resume checkpoints still planned |
| Traversal control | 65% | quick/standard/deep profiles, expected-value scheduling, convergence thresholds, stop reasons, and pressure reporting exist; async parallel traversal still planned |
| QA and calibration | 40% | Synthetic fixture tests cover command inference, cache, guard, sandbox isolation, parseable JSON, malformed JSON, clean probes, cache writes, credential-like writes, SARIF, JUnit, and CI summaries; real CLI corpus still planned |
| Public publishing | 5% | Designed but not implemented |

## MVP Completion

Estimated MVP completion: **80%**

Estimated MVP work remaining: **20%**

The current implementation is useful for local measurement and early CI regression checks. The remaining MVP work is mostly hardening: CI packaging, calibration, async traversal scale, and richer policy controls.

---

## Completed Checkpoints

1. Repository and documentation foundation.
2. Generic `measure` pipeline with evidence, shape, scorecard, and report artifacts.
3. Framework-agnostic inference path rather than Clap-specific parsing.
4. Score v0 over measured dimensions.
5. `guard` regression check against a baseline scorecard.
6. Coverage pressure reporting for depth and probe budgets.
7. Fingerprint/profile-based cache reuse with `--refresh`.
8. Named traversal profiles: `quick`, `standard`, `deep`.
9. Adaptive traversal convergence with expected-value scheduling and typed stop reasons.
10. Richer grammar extraction for aliases, usage positionals, flag arity, required flags, optional values, and repeated values.
11. Sandbox isolation profile with sanitized env, isolated HOME/PWD/XDG config-cache-data/TMP, evidence metadata, scorecard metadata, cache profile matching, and fixture tests proving probe writes land inside the sandbox.
12. Output classification and machine-readable mode detection with generic output-mode claims, safe `--help` output probes, shape output contracts, measured output subscore, and fixtures for parseable and malformed JSON.
13. Side-effect observation and initial safety scoring with per-probe sandbox snapshots, created/modified/deleted file evidence, measured safety subscore, and fixtures for clean CLIs, cache-writing CLIs, and credential-like writes.
14. CI output formats and GitHub Action wrapper with `summary.md`, `findings.sarif`, `junit.xml`, guard-aware CI context, cache artifact-set validation, score outputs, artifact upload, and fixture coverage for generated CI files.

---

## Next Checkpoint

### Checkpoint 15: Async Traversal Execution With Bounded Parallelism

Goal: make deep recursive CLI exploration faster and more scalable without losing determinism, budget accounting, sandbox isolation, or reproducible score outputs.

Acceptance criteria:

- Introduce a bounded async probe scheduler with a configurable concurrency limit.
- Preserve deterministic probe IDs, evidence ordering, planner convergence, and score reproducibility.
- Keep every concurrent probe isolated with independent sandbox snapshots and side-effect diffs.
- Add clear budget accounting for scheduled, completed, cancelled, skipped, and converged probes.
- Add tests that prove repeated runs over the same fixture produce equivalent shape and score artifacts under concurrent execution.
- Dogfood the implementation against CLIARE itself with the `deep` profile.

Why this is next:

- Deeper CLIs need more traversal work than a purely serial executor should perform in CI.
- Parallel execution must be introduced before real corpus calibration so benchmark timings are representative.
- Deterministic concurrency is a core credibility requirement for a scoring standard.

---

## Near-Term Order

1. Async traversal execution with bounded parallelism and deterministic convergence.
2. Policy controls for side-effect allowances and score thresholds.
3. Real CLI benchmark corpus and calibration metrics.
4. Baseline accept/rescore/certify/publish command surfaces.
