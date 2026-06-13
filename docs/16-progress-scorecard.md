# CLIARE Progress Scorecard

> Last updated: after checkpoint `ci: CLIARE on CLIARE score and documentation hardening`

This scorecard tracks implementation progress for the reference CLIARE runner. It is not the public CLI readiness score model; it is the project delivery scorecard for the MVP.

---

## Current Status

| Area | Status | Notes |
|---|---:|---|
| Repository and package foundation | 100% | Rust project, license, README, design docs, private GitHub repo |
| CLI surface | 76% | `measure`, `guard`, `benchmark`, `metadata`, traversal profiles, cache bypass, CI artifacts, and policy files; baseline/publish/certify still planned |
| Runtime probing | 82% | Safe bootstrap, bounded output, timeouts, recursive probes, per-probe isolated HOME/PWD/XDG config-cache-data/TMP, sanitized env, per-probe filesystem diffs, transient-file-safe snapshots, and inherited pipe drain protection |
| Generic inference | 69% | Layout claims, runtime confirmation, precondition-blocked runtime state, Bayesian confidence, usage positionals, aliases, flag grammar, output-mode claims, format-context output filtering, structural command-row extraction, and manpage false-positive suppression exist; value domains still planned |
| Command shape artifact | 72% | Commands, runtime states, auth preconditions, aliases, positionals, flags, flag arity, output contracts, gaps, confidence, and evidence references exist; richer value domains still planned |
| Scoring | 62% | v0 dimensions for discovery, grammar, execution, recovery, output, and initial safety; Bayesian claim confidence and implemented formulas are now documented; full calibration and confidence intervals still planned |
| CI guard | 90% | Baseline comparison, policy evaluation, SARIF, JUnit, Markdown CI summary, GitHub Action wrapper, and CLIARE-on-CLIARE workflow exist; richer policy ergonomics still planned |
| Cache and fingerprinting | 65% | Binary/profile/version/sandbox-profile/concurrency/artifact-set cache reuse exists; replay/resume checkpoints still planned |
| Traversal control | 86% | quick/standard/deep profiles, expected-value scheduling, convergence thresholds, stop reasons, pressure reporting, bounded async probe rounds, and corpus-level target parallelism exist; richer cancellation policy still planned |
| QA and calibration | 77% | Synthetic fixture tests cover command inference, precondition-blocked probes, cache, guard, policies, sandbox isolation, parseable JSON, malformed JSON, clean probes, cache writes, credential-like writes, SARIF, JUnit, CI summaries, and serial-vs-concurrent traversal equivalence; real CLI benchmark corpus and calibration bands now run locally; public authority plan now defines truth sets, calibration metrics, certified profiles, and false-safe-rate requirements |
| Public publishing | 5% | Designed but not implemented |

## MVP Completion

Estimated MVP completion: **97%**

Estimated MVP work remaining: **3%**

The current implementation is useful for local measurement, CI regression checks, and real CLI corpus calibration. The remaining MVP work is mostly hardening public command surfaces and replay/resume support for long deep-profile runs.

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
17. Real CLI benchmark corpus with `cliare.benchmark-corpus.v1` manifests, target-level parallelism, expected score bands, runtime caps, streaming JSON/Markdown reports, single-writer atomic aggregate writes, output-directory locking, per-target artifacts, and a deep local corpus covering `cliare`, `rote`, `git`, `supabase`, `gh`, `cargo`, `npm`, `docker`, and `deno`.
18. Structural command-row extraction hardening that avoids semantic section-title allowlists, rejects wrapped prose and numeric menu rows, suppresses non-root manpage child-command false positives, and keeps command discovery rooted in layout morphology plus runtime confirmation.
19. Precondition-blocked runtime state with auth-required diagnostic classification, command shape `runtime_state: precondition_blocked`, precondition gaps, scorecard/report coverage counts, recovery accounting that excludes auth-blocked invalid probes, and benchmark precondition totals.
20. Scoring model documentation that explains the current `cliare-score-v0` formulas, Bayesian log-odds claim confidence, the calibration boundary between CI-ready scoring and public leaderboard certification, and the path to `cliare-score-v1`.
21. Calibration authority documentation that defines truth corpus layers, human-reviewed labels, proper scoring metrics, false-safe-rate reporting, repeated-run stability, confidence intervals, certified profiles, provenance, verification levels, anti-gaming fixtures, `calibrate` artifacts, and `cliare-score-v1` freeze criteria.
22. GitHub Actions CLIARE-on-CLIARE workflow that builds the project, measures the freshly built binary with the local composite action, publishes a job-summary score, uploads CLIARE artifacts, runs `quick` on pull requests, `standard` on `main`, and `deep` on the weekly schedule.
23. Public documentation and CLIARE-on-CLIARE score hardening with mature README/docs language, renamed CI workflow/artifacts, an explicit parseable `metadata --format json` contract, stricter output-contract inference that rejects file defaults and unrelated help text, and a standard-profile CLIARE-on-CLIARE score of 97.4/100 with zero findings, one machine-readable output contract, one successful output parse, zero side effects, and complete traversal.

---

## Next Checkpoint

### Checkpoint 24: Baseline Accept, Rescore, Certify, and Replay

Goal: turn the current measurement and benchmark core into a complete user-facing MVP flow for maintainers and CI.

Acceptance criteria:

- Add baseline acceptance so projects can intentionally bless the current scorecard.
- Add evidence replay/rescore so score-model changes can be evaluated without re-running every probe.
- Add a `certify` surface that composes measure, guard policy, benchmark metadata, and profile labeling.
- Add resumable checkpoints for long deep-profile runs and benchmark corpuses.
- Keep every new command artifact-compatible with existing `scorecard.json`, `shape.json`, `evidence.jsonl`, and benchmark reports.

Why this is next:

- Maintainers need a stable workflow after the first measurement.
- Replay/rescore is the missing bridge between experimental scoring and reproducible model evolution.
- Certification should be a composition of measured evidence, policy, and provenance rather than a separate scoring path.

## Latest Benchmark Snapshot

CLIARE on CLIARE standard-profile run:

```sh
cliare measure ./target/debug/cliare --out /tmp/cliare-on-cliare-polish2 --profile standard --concurrency 4 --refresh
```

Result:

| Score | Probes | Output contracts | Parse successes | Findings | Side effects | Traversal complete |
|---:|---:|---:|---:|---:|---:|---|
| 97.4 | 23 | 1 | 1 | 0 | 0 | true |

Local deep corpus run:

Run:

```sh
cliare benchmark --manifest benchmarks/local-corpus.json --out /tmp/cliare-benchmark-deep --target-concurrency 3 --refresh
```

Result:

| Target | Score | Duration ms | Probes | Traversal complete |
|---|---:|---:|---:|---|
| cliare | 94.4 | 843 | 20 | true |
| rote | 62.9 | 106110 | 768 | n/a |
| git | 92.2 | 45266 | 73 | true |
| supabase | 93.6 | 120855 | 409 | true |
| gh | 89.6 | 223860 | 512 | n/a |
| cargo | 92.0 | 6447 | 152 | true |
| npm | 36.0 | 416 | 7 | true |
| docker | 86.3 | 41571 | 253 | true |
| deno | 72.0 | 399806 | 512 | n/a |

Corpus totals: 9 measured, 0 skipped, 0 failed, 2706 probes, 100% expected-band pass rate, 66.7% traversal completion rate, and 33.3% budget exhaustion rate.

---

## Near-Term Order

1. Baseline accept/rescore/certify command surfaces.
2. Replay/resume checkpointing for long deep-profile and benchmark runs.
3. Policy ergonomics: named policy presets and policy schema docs.
4. Public scorecard publishing and leaderboard ingestion.
