# CLIARE Progress Scorecard

> Last updated: after checkpoint `feat: enforce guard policies`

This scorecard tracks implementation progress for the reference CLIARE runner. It is not the public CLI readiness score model; it is the project delivery scorecard for the MVP.

---

## Current Status

| Area | Status | Notes |
|---|---:|---|
| Repository and package foundation | 100% | Rust project, license, README, design docs, private GitHub repo |
| CLI surface | 68% | `measure`, `guard`, traversal profiles, cache bypass, CI artifacts, and policy files; baseline/publish/certify still planned |
| Runtime probing | 76% | Safe bootstrap, bounded output, timeouts, recursive probes, per-probe isolated HOME/PWD/XDG config-cache-data/TMP, sanitized env, per-probe filesystem diffs |
| Generic inference | 58% | Layout claims, runtime confirmation, Bayesian confidence, usage positionals, aliases, flag grammar, and output-mode claims exist; value domains still planned |
| Command shape artifact | 68% | Commands, aliases, positionals, flags, flag arity, output contracts, gaps, confidence, and evidence references exist; richer value domains still planned |
| Scoring | 58% | v0 dimensions for discovery, grammar, execution, recovery, output, and initial safety; determinism still planned |
| CI guard | 86% | Baseline comparison, policy evaluation, SARIF, JUnit, Markdown CI summary, and GitHub Action wrapper exist; richer policy ergonomics still planned |
| Cache and fingerprinting | 65% | Binary/profile/version/sandbox-profile/concurrency/artifact-set cache reuse exists; replay/resume checkpoints still planned |
| Traversal control | 82% | quick/standard/deep profiles, expected-value scheduling, convergence thresholds, stop reasons, pressure reporting, and bounded async probe rounds exist; richer cancellation policy still planned |
| QA and calibration | 50% | Synthetic fixture tests cover command inference, cache, guard, policies, sandbox isolation, parseable JSON, malformed JSON, clean probes, cache writes, credential-like writes, SARIF, JUnit, CI summaries, and serial-vs-concurrent traversal equivalence; real CLI corpus still planned |
| Public publishing | 5% | Designed but not implemented |

## MVP Completion

Estimated MVP completion: **91%**

Estimated MVP work remaining: **9%**

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
16. Policy controls with `cliare.policy.v1` JSON files, total score thresholds, per-dimension subscore thresholds, side-effect allow paths, unapproved side-effect limits, credential-like deny rules, guard integration, CI summary/JUnit reporting, GitHub Action `policy` input, and fixture coverage for pass/fail cases.

---

## Next Checkpoint

### Checkpoint 17: Real CLI Benchmark Corpus and Calibration Metrics

Goal: validate scoring behavior against a representative local corpus of real CLIs so the score model is defensible, stable, and useful outside synthetic fixtures.

Acceptance criteria:

- Define a local benchmark manifest for common CLIs available in CI/dev environments.
- Add a benchmark runner that records score, runtime, probe count, traversal completion, and findings per target.
- Store expected score bands rather than brittle exact scores.
- Add calibration checks that flag score model regressions and runtime blowups.
- Include at least one deep-subcommand CLI, one sparse-help CLI, one JSON-friendly CLI, and one side-effect-prone CLI.
- Produce a benchmark Markdown/JSON report suitable for release notes and model tuning.

Why this is next:

- Synthetic fixtures prove mechanics, but the public standard needs real-world calibration.
- Benchmark bands make score changes explainable before publishing a leaderboard.
- Runtime metrics are needed to keep deep traversal practical in CI.

---

## Near-Term Order

1. Real CLI benchmark corpus and calibration metrics.
2. Baseline accept/rescore/certify/publish command surfaces.
3. Replay/resume checkpointing for long deep-profile runs.
4. Policy ergonomics: named policy presets and policy schema docs.
