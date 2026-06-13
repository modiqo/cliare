# CLIARE Progress Scorecard

> Last updated: after checkpoint `feat: score sandbox side effects`

This scorecard tracks implementation progress for the reference CLIARE runner. It is not the public CLI readiness score model; it is the project delivery scorecard for the MVP.

---

## Current Status

| Area | Status | Notes |
|---|---:|---|
| Repository and package foundation | 100% | Rust project, license, README, design docs, private GitHub repo |
| CLI surface | 55% | `measure`, `guard`, traversal profiles, cache bypass; baseline/publish/certify still planned |
| Runtime probing | 68% | Safe bootstrap, bounded output, timeouts, recursive probes, isolated HOME/PWD/XDG config-cache-data/TMP, sanitized env, per-probe filesystem diffs |
| Generic inference | 58% | Layout claims, runtime confirmation, Bayesian confidence, usage positionals, aliases, flag grammar, and output-mode claims exist; value domains still planned |
| Command shape artifact | 68% | Commands, aliases, positionals, flags, flag arity, output contracts, gaps, confidence, and evidence references exist; richer value domains still planned |
| Scoring | 58% | v0 dimensions for discovery, grammar, execution, recovery, output, and initial safety; determinism still planned |
| CI guard | 40% | Baseline comparison exists; policy files, SARIF/JUnit, GitHub Action still planned |
| Cache and fingerprinting | 60% | Binary/profile/version/sandbox-profile cache reuse exists; replay/resume checkpoints still planned |
| Traversal control | 65% | quick/standard/deep profiles, expected-value scheduling, convergence thresholds, stop reasons, and pressure reporting exist; async parallel traversal still planned |
| QA and calibration | 36% | Synthetic fixture tests cover command inference, cache, guard, sandbox isolation, parseable JSON, malformed JSON, clean probes, cache writes, and credential-like writes; real CLI corpus still planned |
| Public publishing | 5% | Designed but not implemented |

## MVP Completion

Estimated MVP completion: **74%**

Estimated MVP work remaining: **26%**

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

---

## Next Checkpoint

### Checkpoint 14: CI Output Formats and GitHub Action Wrapper

Goal: make CLIARE easy to adopt in CI by emitting machine-readable test artifacts and providing a minimal GitHub Action wrapper.

Acceptance criteria:

- Emit SARIF for findings that should appear in code-scanning or PR review surfaces.
- Emit JUnit XML for CI systems that expect test-style pass/fail artifacts.
- Emit a compact Markdown summary suitable for GitHub step summaries.
- Add a GitHub Action wrapper that runs `cliare measure` or `cliare guard`, uploads artifacts, and exposes score outputs.
- Preserve local-first behavior; CI should run the binary in the caller's environment and never upload the target CLI.
- Add fixtures/tests for generated SARIF, JUnit, and summary files.

Why this is next:

- The core scorecard is now useful; adoption depends on frictionless CI integration.
- CI artifacts make CLIARE reviewable by maintainers and easy to wire into score regression policies.
- This is the shortest path to OSS visibility and practical GTM distribution.

---

## Near-Term Order

1. CI output formats and GitHub Action wrapper.
2. Async traversal execution with bounded parallelism and deterministic convergence.
3. Policy controls for side-effect allowances and score thresholds.
4. Real CLI benchmark corpus and calibration metrics.
