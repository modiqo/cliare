# CLIARE Progress Scorecard

> Last updated: after checkpoint `feat: add adaptive traversal convergence`

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
| Traversal control | 65% | quick/standard/deep profiles, expected-value scheduling, convergence thresholds, stop reasons, and pressure reporting exist; async parallel traversal still planned |
| QA and calibration | 25% | Synthetic fixture tests exist; real CLI corpus and calibration metrics still planned |
| Public publishing | 5% | Designed but not implemented |

## MVP Completion

Estimated MVP completion: **51%**

Estimated MVP work remaining: **49%**

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
9. Adaptive traversal convergence with expected-value scheduling and typed stop reasons.

---

## Next Checkpoint

### Checkpoint 10: Richer Grammar Extraction

Goal: infer more of the CLI contract from runtime evidence, not only command existence.

Acceptance criteria:

- Command shape includes positional placeholders from usage lines.
- Flags distinguish boolean, optional-value, required-value, and repeated forms when help output exposes it.
- Alias rows are represented without treating aliases as unrelated commands.
- Scorecard grammar rationale references extracted grammar evidence.
- Fixture tests cover usage syntax, required flags, optional flags, repeated flags, aliases, and malformed help.

Why this is next:

- Current discovery is stronger than current grammar inference.
- Agent readiness depends on knowing how to call a command, not only that a command exists.
- Better grammar extraction improves score quality before adding sandbox-only safety probes.

---

## Near-Term Order

1. Richer grammar extraction for positionals, required flags, aliases, and value hints.
3. Sandbox profile for temp HOME/cwd/env isolation.
4. CI output formats and GitHub Action wrapper.
5. Real CLI benchmark corpus and calibration metrics.
