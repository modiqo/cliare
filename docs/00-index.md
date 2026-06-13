# CLIARE: CLI for Agent Readiness

> **Status:** Product and Technical Design Draft
> **Owner:** modiqo
> **Scope:** Independent OSS standard, reference implementation, CI runner, scorecard, and leaderboard for agent-ready CLIs
> **Working Name:** CLIARE, pronounced "clear"
> **Expansion:** CLI for Agent Readiness

---

## Problem Statement

Agents increasingly rely on command-line tools. They use CLIs to build software, manage infrastructure, operate SaaS APIs through vendor-provided tools, run tests, move data, and automate local workflows. The current state is informal:

- An agent reads help output.
- It searches docs.
- It guesses flags.
- It runs commands in a shell.
- It learns by failure.
- It may cache successful patterns in a skill, prompt, workflow, or harness.

That approach works until the CLI drifts. Help text changes. Flags move. Output formats change. Completion scripts expose commands not documented in help. Hidden plugin commands appear. A new version introduces a destructive default. A command that used to print JSON starts printing progress text before JSON. The agent's learned use becomes stale.

The industry needs a standard way to measure whether a CLI is ready for agents and automation.

CLIARE is that standard.

It is not a documentation linter. It is not a wrapper generator. It is not just a parser for `--help`. CLIARE is a runtime measurement system:

1. It treats the CLI as a black box.
2. It probes the binary inside an isolated sandbox.
3. It captures evidence from help, completions, errors, exit codes, stdout, stderr, file effects, and repeated executions.
4. It infers a probabilistic command-shape model.
5. It computes an agent-readiness score with confidence intervals.
6. It produces a portable scorecard and improvement report.
7. It can run in the project's own CI without uploading binaries to modiqo.

---

## Core Thesis

The main thesis is:

> A CLI's agent readiness is the posterior expected utility of a competent agent using that CLI across realistic tasks, given observed runtime evidence.

That is intentionally mathematical. CLIARE should be credible as a standard. A score should improve when the CLI improves, degrade when behavior becomes less safe or less discoverable, and remain explainable at every level.

The score must never be a black-box opinion. Every point should trace to evidence:

- which probes ran
- what they observed
- which claims were inferred
- what confidence each claim has
- which score dimensions changed
- which fixes would improve the score

---

## Document Map

| # | Document | Purpose | Key Decisions |
|---|----------|---------|---------------|
| **01** | [Vision and Positioning](01-vision-and-positioning.md) | Product strategy, OSS role, modiqo GTM wedge, naming, audience | CLIARE as local-first standard and optional hosted leaderboard |
| **02** | [Use Cases and Personas](02-use-cases-and-personas.md) | Who uses CLIARE and what jobs it solves | Maintainers, agent builders, platform teams, security teams, CI owners |
| **03** | [System Architecture](03-system-architecture.md) | Components, data flow, storage, CLI commands, plugin boundary | Probe -> Evidence -> Inference -> Shape -> Score -> Report |
| **04** | [Probe Sandbox Runtime](04-probe-sandbox-runtime.md) | How CLIARE safely exercises arbitrary binaries | Temp HOME, network policy, filesystem diffing, timeouts, profiles |
| **05** | [Evidence and Command Shape Spec](05-evidence-and-command-shape-spec.md) | Evidence log schema and normalized command-shape IR | Every inferred fact carries provenance and confidence |
| **06** | [Mathematical Model](06-mathematical-model.md) | Formal probabilistic model, Bayesian updates, scoring theory | Posterior expected utility, proper scoring rules, calibration |
| **07** | [Scoring and Improvement Tracking](07-scoring-and-improvement-tracking.md) | Subscores, regressions, monotonic improvements, baselines | Separate known-surface, capability-adjusted, and whole-surface scores |
| **08** | [CI, Leaderboard, and GTM](08-ci-leaderboard-and-gtm.md) | Local CI execution, scorecard publishing, badge strategy | modiqo hosts scorecards, not untrusted binary execution |
| **09** | [QA, Benchmarking, and Calibration](09-qa-benchmarking-and-calibration.md) | Test strategy, benchmark corpus, fixtures, ground truth | Synthetic CLIs plus real CLIs with human-verified truth sets |
| **10** | [Checkpointing and Resume](10-checkpointing-and-resume.md) | Long-running probes, resumability, cache keys, artifact lifecycle | Evidence is append-only; inference and scoring are replayable |
| **11** | [Implementation Plan](11-implementation-plan.md) | Phased roadmap, acceptance criteria, repo layout | MVP through public standard launch |
| **12** | [Reference CLI Behavior Guide](12-reference-cli-behavior-guide.md) | Practical guidance for CLI maintainers | How to improve score and agent usability |
| **13** | [Rust Runtime Engineering](13-rust-runtime-engineering.md) | Async recursive probing, bounded parallelism, Rust traits, memory discipline, error handling | Domain scheduler over Tokio, typed errors, deterministic convergence |
| **14** | [Operational Contracts](14-operational-contracts.md) | Post-core hardening for cache reuse, adversarial targets, dependency policy, score governance, and reproducibility | Non-critical follow-up before public certification |
| **15** | [Generic Inference Processor](15-generic-inference-processor.md) | Corrected framework-agnostic inference design: layout observations, candidate claims, Bayesian updates, confirmation probes | Clap is dogfood only, not an inference assumption |
| **16** | [Progress Scorecard](16-progress-scorecard.md) | Project delivery scorecard, MVP progress, next checkpoint | Current MVP estimate: 55% complete, 45% remaining |

---

## Architecture at a Glance

```
User CI / local machine
    |
    | shell: cliare certify ./dist/mycli
    v
+-------------------------+
| CLIARE Runner           |
| - binary fingerprint    |
| - profile selection     |
| - probe scheduler       |
+-----------+-------------+
            |
            v
+-------------------------+
| Sandbox Runtime         |
| - temp HOME/cwd/XDG     |
| - network policy        |
| - env policy            |
| - fs/process tracing    |
| - time/output limits    |
+-----------+-------------+
            |
            v
+-------------------------+
| Evidence Log            |
| evidence.jsonl          |
| raw observations        |
| redacted payloads       |
+-----------+-------------+
            |
            v
+-------------------------+
| Inference Engine        |
| - command discovery     |
| - grammar inference     |
| - output classification |
| - safety classification |
+-----------+-------------+
            |
            v
+-------------------------+
| Command Shape Catalog   |
| shape.json              |
| claims + confidence     |
+-----------+-------------+
            |
            v
+-------------------------+
| Scoring Engine          |
| - expected utility      |
| - confidence intervals  |
| - deltas vs baseline    |
+-----------+-------------+
            |
            v
+-------------------------+
| Reports and Publishing  |
| scorecard.json          |
| report.md               |
| sarif.json              |
| badge data              |
| optional leaderboard    |
+-------------------------+
```

---

## First-Class Outputs

CLIARE should produce four durable artifacts:

```
.cliare/
  evidence.jsonl
  shape.json
  scorecard.json
  report.md
```

The evidence log is the raw observation record. It is append-only and replayable.

The shape catalog is the inferred command surface.

The scorecard is the compact CI and leaderboard artifact.

The report is the human improvement guide.

---

## Non-Goals

CLIARE is intentionally not:

- a remote cloud executor for arbitrary binaries
- a replacement for a CLI's own test suite
- a generic shell command recorder
- a prompt-only evaluation benchmark
- a wrapper that hides unsafe CLI design
- a static source-code analyzer
- a documentation crawler with no runtime validation

CLIARE can use docs as weak evidence, but the reference implementation should work with only the CLI binary.

---

## Definition of Done for the Standard

CLIARE becomes standard-worthy when:

1. Two independent implementations can consume and produce compatible `shape.json` and `scorecard.json`.
2. A CLI maintainer can run CLIARE in CI without modiqo cloud access.
3. A score is reproducible from evidence.
4. Score changes decompose into understandable improvements or regressions.
5. The benchmark corpus includes synthetic and real-world CLIs with public ground truth.
6. Public leaderboard entries are distinguishable by verification level.
7. The improvement guide tells maintainers exactly how to raise their score.
