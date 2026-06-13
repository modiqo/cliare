# CLIARE Progress Scorecard

> Last updated: after checkpoint `feat: classify machine-readable output modes`

This scorecard tracks implementation progress for the reference CLIARE runner. It is not the public CLI readiness score model; it is the project delivery scorecard for the MVP.

---

## Current Status

| Area | Status | Notes |
|---|---:|---|
| Repository and package foundation | 100% | Rust project, license, README, design docs, private GitHub repo |
| CLI surface | 55% | `measure`, `guard`, traversal profiles, cache bypass; baseline/publish/certify still planned |
| Runtime probing | 60% | Safe bootstrap, bounded output, timeouts, recursive probes, isolated HOME/PWD/XDG config-cache-data/TMP, sanitized env |
| Generic inference | 58% | Layout claims, runtime confirmation, Bayesian confidence, usage positionals, aliases, flag grammar, and output-mode claims exist; value domains still planned |
| Command shape artifact | 68% | Commands, aliases, positionals, flags, flag arity, output contracts, gaps, confidence, and evidence references exist; richer value domains still planned |
| Scoring | 50% | v0 dimensions for discovery, grammar, execution, recovery, and output; safety and determinism still planned |
| CI guard | 40% | Baseline comparison exists; policy files, SARIF/JUnit, GitHub Action still planned |
| Cache and fingerprinting | 60% | Binary/profile/version/sandbox-profile cache reuse exists; replay/resume checkpoints still planned |
| Traversal control | 65% | quick/standard/deep profiles, expected-value scheduling, convergence thresholds, stop reasons, and pressure reporting exist; async parallel traversal still planned |
| QA and calibration | 30% | Synthetic fixture tests cover command inference, cache, guard, sandbox isolation, parseable JSON, and malformed JSON; real CLI corpus still planned |
| Public publishing | 5% | Designed but not implemented |

## MVP Completion

Estimated MVP completion: **68%**

Estimated MVP work remaining: **32%**

The current implementation is useful for local measurement and early CI regression checks. The remaining MVP work is mostly hardening: side-effect scoring, CI packaging, calibration, and async traversal scale.

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

---

## Next Checkpoint

### Checkpoint 13: Side-Effect Observation and Initial Safety Scoring

Goal: observe filesystem side effects inside the sandbox and turn the `safety` dimension into a measured score without executing known-destructive workflows.

Acceptance criteria:

- Snapshot sandbox HOME/cwd/XDG/TMP before and after each safe probe.
- Record created, modified, and deleted files by sandbox region.
- Treat writes during help/version/diagnostic probes as safety evidence.
- Add safety coverage fields and an initial measured safety subscore.
- Add findings for unexpected writes, credential-looking files, and writes outside expected runtime dirs.
- Add fixtures for clean CLIs, cache-writing CLIs, config-writing CLIs, and destructive-looking commands that are not executed.

Why this is next:

- The sandbox exists; the next value is measuring what probes actually changed.
- Side-effect evidence unlocks the currently unmeasured `safety` dimension.
- Local-first OSS adoption depends on proving CLIARE can report surprising writes without needing cloud execution.

---

## Near-Term Order

1. Side-effect observation inside the sandbox and initial safety scoring.
2. CI output formats and GitHub Action wrapper.
3. Async traversal execution with bounded parallelism and deterministic convergence.
4. Real CLI benchmark corpus and calibration metrics.
