# CLIARE Progress Scorecard

> Last updated: after checkpoint `feat: isolate probe sandbox runtime`

This scorecard tracks implementation progress for the reference CLIARE runner. It is not the public CLI readiness score model; it is the project delivery scorecard for the MVP.

---

## Current Status

| Area | Status | Notes |
|---|---:|---|
| Repository and package foundation | 100% | Rust project, license, README, design docs, private GitHub repo |
| CLI surface | 55% | `measure`, `guard`, traversal profiles, cache bypass; baseline/publish/certify still planned |
| Runtime probing | 60% | Safe bootstrap, bounded output, timeouts, recursive probes, isolated HOME/PWD/XDG config-cache-data/TMP, sanitized env |
| Generic inference | 50% | Layout claims, runtime confirmation, Bayesian confidence, usage positionals, aliases, and flag grammar exist; value domains still planned |
| Command shape artifact | 60% | Commands, aliases, positionals, flags, flag arity, gaps, confidence, and evidence references exist; richer value domains still planned |
| Scoring | 42% | v0 dimensions for discovery, grammar, execution, recovery; scorecards now disclose runtime isolation; output, safety, determinism still planned |
| CI guard | 40% | Baseline comparison exists; policy files, SARIF/JUnit, GitHub Action still planned |
| Cache and fingerprinting | 60% | Binary/profile/version/sandbox-profile cache reuse exists; replay/resume checkpoints still planned |
| Traversal control | 65% | quick/standard/deep profiles, expected-value scheduling, convergence thresholds, stop reasons, and pressure reporting exist; async parallel traversal still planned |
| QA and calibration | 25% | Synthetic fixture tests exist; real CLI corpus and calibration metrics still planned |
| Public publishing | 5% | Designed but not implemented |

## MVP Completion

Estimated MVP completion: **62%**

Estimated MVP work remaining: **38%**

The current implementation is useful for local measurement and early CI regression checks. The remaining MVP work is mostly hardening: output classification, CI packaging, side-effect scoring, calibration, and async traversal scale.

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

---

## Next Checkpoint

### Checkpoint 12: Output Classification and Machine-Readable Mode Detection

Goal: classify output contracts from runtime evidence so CLIARE can score whether agents can request and parse stable machine-readable results.

Acceptance criteria:

- Detect advertised JSON/YAML/table/plain modes from help, usage, and diagnostics.
- Probe safe output-mode flags such as `--json`, `--format json`, and documented equivalents when they appear in evidence.
- Add posterior output-kind claims with evidence references.
- Add scorecard/report fields for machine-readable availability, parse success, and ambiguity.
- Keep probing generic; no framework-specific assumptions.
- Add fixtures for CLIs with JSON support, table-only output, misleading format flags, and malformed JSON.

Why this is next:

- Discovery and grammar are now useful, but agents also need stable parseable outputs.
- Output classification unlocks the currently unmeasured `output` dimension.
- The sandbox milestone makes safe output-mode probing more credible.

---

## Near-Term Order

1. Output classification and machine-readable mode detection.
2. Side-effect observation inside the sandbox and initial safety scoring.
3. CI output formats and GitHub Action wrapper.
4. Real CLI benchmark corpus and calibration metrics.
