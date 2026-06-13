# CLIARE Progress Scorecard

> Last updated: after checkpoint `feat: add traversal profiles`

This scorecard tracks implementation progress for the reference CLIARE runner. It is not the public CLI readiness score model; it is the project delivery scorecard for the MVP.

---

## Current Status

| Area | Status | Notes |
|---|---:|---|
| Repository and package foundation | 100% | Rust project, license, README, design docs, private GitHub repo |
| CLI surface | 55% | `measure`, `guard`, traversal profiles, cache bypass; baseline/publish/certify still planned |
| Runtime probing | 45% | Safe bootstrap, bounded output, timeouts, recursive probes; sandbox isolation still planned |
| Generic inference | 40% | Layout claims, runtime confirmation, Bayesian confidence; richer grammar still planned |
| Command shape artifact | 45% | Commands, flags, gaps, confidence, evidence references; positional/value domains still planned |
| Scoring | 35% | v0 dimensions for discovery, grammar, execution, recovery; output, safety, determinism still planned |
| CI guard | 40% | Baseline comparison exists; policy files, SARIF/JUnit, GitHub Action still planned |
| Cache and fingerprinting | 55% | Binary/profile/version cache reuse exists; replay/resume checkpoints still planned |
| Traversal control | 50% | quick/standard/deep profiles and pressure reporting exist; adaptive convergence still planned |
| QA and calibration | 25% | Synthetic fixture tests exist; real CLI corpus and calibration metrics still planned |
| Public publishing | 5% | Designed but not implemented |

## MVP Completion

Estimated MVP completion: **48%**

Estimated MVP work remaining: **52%**

The current implementation is already useful for local measurement and early CI regression checks. The remaining MVP work is mostly hardening: adaptive traversal, stronger grammar extraction, sandbox isolation, CI packaging, and calibration.

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

---

## Next Checkpoint

### Checkpoint 9: Adaptive Traversal Convergence

Goal: make traversal feel less like a fixed probe loop and more like a bounded compiler pass.

Acceptance criteria:

- The scheduler reports why it stopped: exhausted frontier, probe budget, depth budget, or convergence.
- Scorecards include a traversal status summary suitable for CI.
- The planner can stop early when no high-value uncertain claims remain.
- Terminal output distinguishes complete runs from budget-limited runs.
- Tests cover complete traversal, probe-budget stop, depth-budget stop, and convergence stop.

Why this is next:

- It turns traversal profiles into intelligent execution rather than static presets.
- It helps maintainers understand whether a score is trustworthy enough for CI gating.
- It prepares the runtime for async parallel probing and checkpoint/resume without changing the public CLI again.

---

## Near-Term Order

1. Adaptive traversal convergence.
2. Richer grammar extraction for positionals, required flags, aliases, and value hints.
3. Sandbox profile for temp HOME/cwd/env isolation.
4. CI output formats and GitHub Action wrapper.
5. Real CLI benchmark corpus and calibration metrics.
