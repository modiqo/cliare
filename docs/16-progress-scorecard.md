# CLIARE Progress Scorecard

> Last updated: after checkpoint `feat: extract command grammar`

This scorecard tracks implementation progress for the reference CLIARE runner. It is not the public CLI readiness score model; it is the project delivery scorecard for the MVP.

---

## Current Status

| Area | Status | Notes |
|---|---:|---|
| Repository and package foundation | 100% | Rust project, license, README, design docs, private GitHub repo |
| CLI surface | 55% | `measure`, `guard`, traversal profiles, cache bypass; baseline/publish/certify still planned |
| Runtime probing | 45% | Safe bootstrap, bounded output, timeouts, recursive probes; sandbox isolation still planned |
| Generic inference | 50% | Layout claims, runtime confirmation, Bayesian confidence, usage positionals, aliases, and flag grammar exist; value domains still planned |
| Command shape artifact | 60% | Commands, aliases, positionals, flags, flag arity, gaps, confidence, and evidence references exist; richer value domains still planned |
| Scoring | 40% | v0 dimensions for discovery, grammar, execution, recovery; grammar now credits extracted usage and flag arity; output, safety, determinism still planned |
| CI guard | 40% | Baseline comparison exists; policy files, SARIF/JUnit, GitHub Action still planned |
| Cache and fingerprinting | 55% | Binary/profile/version cache reuse exists; replay/resume checkpoints still planned |
| Traversal control | 65% | quick/standard/deep profiles, expected-value scheduling, convergence thresholds, stop reasons, and pressure reporting exist; async parallel traversal still planned |
| QA and calibration | 25% | Synthetic fixture tests exist; real CLI corpus and calibration metrics still planned |
| Public publishing | 5% | Designed but not implemented |

## MVP Completion

Estimated MVP completion: **55%**

Estimated MVP work remaining: **45%**

The current implementation is already useful for local measurement and early CI regression checks. The remaining MVP work is mostly hardening: sandbox isolation, output classification, CI packaging, side-effect scoring, and calibration.

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

---

## Next Checkpoint

### Checkpoint 11: Sandbox Isolation Profile

Goal: make runtime probing safer and more reproducible by isolating process environment, home directories, and working directories.

Acceptance criteria:

- Each measurement run uses an explicit sandbox root under the artifact directory or temp directory.
- Probes receive deterministic `HOME`, `PWD`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, and `CLIARE=1` environment values.
- Scorecards record sandbox profile metadata.
- Evidence records each probe's sandbox-relevant cwd/env policy.
- Tests prove probes do not write into the real home directory for fixture CLIs.
- The default path is safe; a bypass flag should wait until the threat model is explicit.

Why this is next:

- CLIARE is intentionally a runtime exerciser of untrusted or semi-trusted binaries.
- Public OSS adoption depends on a credible local safety story.
- Sandboxing creates the foundation for later side-effect detection and safety scoring.

---

## Near-Term Order

1. Sandbox profile for temp HOME/cwd/env isolation.
2. Output classification and machine-readable mode detection.
3. CI output formats and GitHub Action wrapper.
4. Real CLI benchmark corpus and calibration metrics.
