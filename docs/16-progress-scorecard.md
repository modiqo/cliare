# CLIARE Progress Scorecard

> Last updated: after checkpoint `feat: run bounded async probes`

This scorecard tracks implementation progress for the reference CLIARE runner. It is not the public CLI readiness score model; it is the project delivery scorecard for the MVP.

---

## Current Status

| Area | Status | Notes |
|---|---:|---|
| Repository and package foundation | 100% | Rust project, license, README, design docs, private GitHub repo |
| CLI surface | 60% | `measure`, `guard`, traversal profiles, cache bypass, CI artifacts; baseline/publish/certify still planned |
| Runtime probing | 76% | Safe bootstrap, bounded output, timeouts, recursive probes, per-probe isolated HOME/PWD/XDG config-cache-data/TMP, sanitized env, per-probe filesystem diffs |
| Generic inference | 58% | Layout claims, runtime confirmation, Bayesian confidence, usage positionals, aliases, flag grammar, and output-mode claims exist; value domains still planned |
| Command shape artifact | 68% | Commands, aliases, positionals, flags, flag arity, output contracts, gaps, confidence, and evidence references exist; richer value domains still planned |
| Scoring | 58% | v0 dimensions for discovery, grammar, execution, recovery, output, and initial safety; determinism still planned |
| CI guard | 70% | Baseline comparison, SARIF, JUnit, Markdown CI summary, and GitHub Action wrapper exist; policy files still planned |
| Cache and fingerprinting | 65% | Binary/profile/version/sandbox-profile/concurrency/artifact-set cache reuse exists; replay/resume checkpoints still planned |
| Traversal control | 82% | quick/standard/deep profiles, expected-value scheduling, convergence thresholds, stop reasons, pressure reporting, and bounded async probe rounds exist; richer cancellation policy still planned |
| QA and calibration | 45% | Synthetic fixture tests cover command inference, cache, guard, sandbox isolation, parseable JSON, malformed JSON, clean probes, cache writes, credential-like writes, SARIF, JUnit, CI summaries, and serial-vs-concurrent traversal equivalence; real CLI corpus still planned |
| Public publishing | 5% | Designed but not implemented |

## MVP Completion

Estimated MVP completion: **86%**

Estimated MVP work remaining: **14%**

The current implementation is useful for local measurement and early CI regression checks. The remaining MVP work is mostly hardening: policy controls, calibration, public command surfaces, and replay/resume support for long deep-profile runs.

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
15. Bounded async traversal with profile-derived and user-configurable concurrency, per-probe sandbox roots, deterministic round scheduling, stable probe IDs, ordered evidence commits, scheduler accounting in scorecards/reports/summaries, cache profile matching, and serial-vs-concurrent fixture equivalence.

---

## Next Checkpoint

### Checkpoint 16: Policy Controls for Side Effects and Score Thresholds

Goal: let maintainers define project-specific policy without changing the core score model, especially for expected side effects, minimum acceptable subscores, and stricter CI failure conditions.

Acceptance criteria:

- Add a versioned policy file schema.
- Support minimum total score and per-dimension subscore thresholds.
- Support allowed side-effect path patterns and credential-like path deny rules.
- Let `guard` evaluate both score regression and policy failures.
- Render policy pass/fail details in terminal output, `summary.md`, and `junit.xml`.
- Add fixture tests for allowed cache writes, denied credential writes, output threshold failures, and strict score threshold failures.

Why this is next:

- Safety findings are currently informational unless the total score regresses.
- CI adopters need explicit pass/fail policy controls before using CLIARE as a release gate.
- Policy controls are the bridge between generic scoring and project-specific operational tolerance.

---

## Near-Term Order

1. Policy controls for side-effect allowances and score thresholds.
2. Real CLI benchmark corpus and calibration metrics.
3. Baseline accept/rescore/certify/publish command surfaces.
4. Replay/resume checkpointing for long deep-profile runs.
