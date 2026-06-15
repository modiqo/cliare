# CLIARE Progress Scorecard

> Last updated: after checkpoint `detached measurement jobs and status inspection`

This scorecard tracks implementation progress for the reference CLIARE runner. It is not the public CLI readiness score model; it records implementation progress toward the first stable release.

---

## Current Status

| Area | Status | Notes |
|---|---:|---|
| Repository and package foundation | 100% | Rust project, license, README, design docs, GitHub repository |
| CLI surface | 82% | `measure`, `measure --detach`, `jobs status`, `guard`, `benchmark`, `report`, `describe`, `skills`, `metadata`, traversal profiles, cache bypass, CI artifacts, and policy files; baseline/publish/certify still planned |
| Runtime probing | 82% | Safe bootstrap, bounded output, timeouts, recursive probes, per-probe isolated HOME/PWD/XDG config-cache-data/TMP, sanitized env, per-probe filesystem diffs, transient-file-safe snapshots, and inherited pipe drain protection |
| Generic inference | 69% | Layout claims, runtime confirmation, precondition-blocked runtime state, Bayesian confidence, usage positionals, aliases, flag grammar, output-mode claims, format-context output filtering, structural command-row extraction, and manpage false-positive suppression exist; value domains still planned |
| Artifact contracts | 76% | Command shape, command index, issue ledger, persona packets, CI artifacts, and artifact maps exist; richer value domains and formal JSON schemas still planned |
| Scoring | 62% | v0 dimensions for discovery, grammar, execution, recovery, output, and initial safety; Bayesian claim confidence and implemented formulas are now documented; full calibration and confidence intervals still planned |
| CI guard | 90% | Baseline comparison, policy evaluation, SARIF, JUnit, Markdown CI summary, GitHub Action wrapper, and CLIARE-on-CLIARE workflow exist; richer policy ergonomics still planned |
| Cache, fingerprinting, and jobs | 70% | Binary/profile/version/sandbox-profile/concurrency/artifact-set cache reuse exists; foreground and detached progress jobs exist; replay/resume checkpoints still planned |
| Traversal control | 86% | quick/standard/deep profiles, expected-value scheduling, convergence thresholds, stop reasons, pressure reporting, bounded async probe rounds, and corpus-level target parallelism exist; richer cancellation policy still planned |
| QA and calibration | 77% | Synthetic fixture tests cover command inference, precondition-blocked probes, cache, guard, policies, sandbox isolation, parseable JSON, malformed JSON, clean probes, cache writes, credential-like writes, SARIF, JUnit, CI summaries, and serial-vs-concurrent traversal equivalence; real CLI benchmark corpus and calibration bands now run locally; public authority plan now defines truth sets, calibration metrics, certified profiles, and false-safe-rate requirements |
| Public publishing | 5% | Designed but not implemented |

## Implementation Completion

Estimated initial-release completion: **98%**

Estimated initial-release work remaining: **2%**

The current implementation is useful for local measurement, CI regression checks, and real CLI corpus calibration. The remaining initial-release work is mostly hardening public command surfaces and replay/resume support for long deep-profile runs.

---

## Completed Checkpoints

1. Repository and documentation foundation.
2. Generic `measure` pipeline with evidence, shape, scorecard, and report artifacts.
3. Framework-agnostic inference path rather than Clap-specific parsing.
4. Score v0 through a typed bundled score-model artifact with declared-dimension normalization.
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
17. Real CLI benchmark corpus with `cliare.benchmark-corpus.v1` manifests, target-level parallelism, expected score bands, runtime caps, streaming JSON/Markdown reports, single-writer atomic aggregate writes, output-directory locking, per-target artifacts, and a deep local corpus covering `cliare`, `rote`, `git`, `supabase`, `gh`, `cargo`, `npm`, `docker`, and `deno`.
18. Structural command-row extraction hardening that avoids semantic section-title allowlists, rejects wrapped prose and numeric menu rows, suppresses non-root manpage child-command false positives, and keeps command discovery rooted in layout morphology plus runtime confirmation.
19. Precondition-blocked runtime state with feature-based diagnostic classification for auth and local context requirements, command shape `runtime_state: precondition_blocked`, precondition gaps, scorecard/report coverage counts, actionable recovery scoring, and benchmark precondition totals.
20. Scoring model documentation that explains the current `cliare-score-v0` formulas, Bayesian log-odds claim confidence, the calibration boundary between CI-ready scoring and public leaderboard certification, and the path to `cliare-score-v1`.
21. Calibration authority documentation that defines truth corpus layers, human-reviewed labels, proper scoring metrics, false-safe-rate reporting, repeated-run stability, confidence intervals, certified profiles, provenance, verification levels, anti-gaming fixtures, `calibrate` artifacts, and `cliare-score-v1` freeze criteria.
22. GitHub Actions CLIARE-on-CLIARE workflow that builds the project, measures the freshly built binary with the local composite action, publishes a job-summary score, uploads CLIARE artifacts, runs `quick` on pull requests, `standard` on `main`, and `deep` on the weekly schedule.
23. Public documentation and CLIARE-on-CLIARE score hardening with mature README/docs language, renamed CI workflow/artifacts, an explicit parseable `metadata --format json` contract, stricter output-contract inference that rejects file defaults and unrelated help text, and a standard-profile CLIARE-on-CLIARE score of 97/100 with zero findings, one machine-readable output contract, one successful output parse, zero side effects, and complete traversal.
24. Detached measurement jobs with `measure --detach`, worker re-exec, Unix process-group isolation, child PID reporting, stdout/stderr job streams, preserved `jobs/current` metadata, `jobs status --out <dir>`, and live verification against CLIARE measuring CLIARE.
25. Artifact directory description with `cliare describe <folder>`, `cliare.artifact-map.v1`, typed file roles, health/missing-required checks, current job status, score/issue/command summaries, Markdown and JSON output, `--write` support, agent skill updates, and documentation/paper coverage.
26. Score-model v1 foundation with `score-models/cliare-score-v0.json`, typed Rust validation in `src/score_model.rs`, model SHA-256 provenance in scorecards, whole-point v0 score precision, extraction-limited measurement findings, partial-score invariants, cache summary round-trip tests, and a vendor calibration corpus manifest for train/validation/holdout planning.

---

## Next Checkpoint

### Checkpoint 27: Calibration Workflow Foundation

Goal: add the first honest calibration workflow so benchmark runs can become labeled truth sets and model-quality reports.

Acceptance criteria:

- Add `cliare calibrate init` to scaffold `cliare.truth.v1` from an existing measurement artifact.
- Add `cliare calibrate check` to validate corpus structure, split discipline, runtime-context labeling, and review readiness.
- Add `cliare calibrate evaluate` to compare predicted claims against truth labels and emit calibration metrics.
- Keep fitting out of scope until enough labeled train, validation, and holdout data exists.
- Keep `cliare-score-v0` as the active score model until calibration evidence supports a candidate v1.

Why this is next:

- The benchmark tracker can only mature into a standard if measurements become truth-labeled evaluation data.
- Calibration is the bridge between useful CI scoring and credible public leaderboard authority.
- The workflow must be implemented before model fitting so the project does not train on its own unverified predictions.

## Latest Benchmark Snapshot

CLIARE on CLIARE standard-profile run:

```sh
cliare measure ./target/debug/cliare --out /tmp/cliare-on-cliare-polish2 --profile standard --concurrency 4 --refresh
```

Result:

| Score | Probes | Output contracts | Parse successes | Findings | Side effects | Traversal complete |
|---:|---:|---:|---:|---:|---:|---|
| 97 | 23 | 1 | 1 | 0 | 0 | true |

Local deep corpus run:

Run:

```sh
cliare benchmark --manifest benchmarks/local-corpus.json --out /tmp/cliare-benchmark-deep --target-concurrency 3 --refresh
```

Result:

| Target | Score | Duration ms | Probes | Traversal complete |
|---|---:|---:|---:|---|
| cliare | 94 | 843 | 20 | true |
| rote | 63 | 106110 | 768 | n/a |
| git | 92 | 45266 | 73 | true |
| supabase | 94 | 120855 | 409 | true |
| gh | 90 | 223860 | 512 | n/a |
| cargo | 92 | 6447 | 152 | true |
| npm | 36 | 416 | 7 | true |
| docker | 86 | 41571 | 253 | true |
| deno | 72 | 399806 | 512 | n/a |

Corpus totals: 9 measured, 0 skipped, 0 failed, 2706 probes, 100% expected-band pass rate, 66.7% traversal completion rate, and 33.3% budget exhaustion rate.

---

## Near-Term Order

1. Calibration workflow foundation: init, check, evaluate.
2. Baseline accept/rescore/certify command surfaces.
3. Replay/resume checkpointing for long deep-profile and benchmark runs.
4. Policy ergonomics: named policy presets and policy schema docs.
5. Public scorecard publishing and leaderboard ingestion.
