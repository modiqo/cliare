# CLIARE: CLI Agent Readiness Evaluation

> **Status:** Technical Reference
> **Owner:** modiqo
> **Scope:** Independent OSS standard, reference implementation, CI runner, runtime catalog, scorecard, and calibrated publishing path for agent-ready CLIs
> **Name:** CLIARE, pronounced "clear"
> **Expansion:** CLI Agent Readiness Evaluation

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

The industry needs a standard way to observe, catalog, and improve CLIs for agents and automation. CLIARE provides a runtime evidence standard for that work.

It is not a documentation linter. It is not a wrapper generator. It is not just a parser for `--help`. CLIARE is a runtime measurement system:

1. It treats the CLI as a black box.
2. It probes the binary inside an isolated sandbox.
3. It captures evidence from help, completions, errors, exit codes, stdout, stderr, file effects, and repeated executions.
4. It infers a probabilistic command-shape model.
5. It emits command indexes, issue ledgers, scorecards, and improvement reports.
6. It derives an experimental agent-readiness score with confidence metadata.
7. It can run in the project's own CI without uploading binaries to modiqo.

---

## Core Thesis

The main thesis is:

> A CLI's agent readiness is the posterior expected utility of a competent agent using that CLI across realistic tasks, given observed runtime evidence.

That definition is intentionally formal. CLIARE is evaluated as a measurement standard: its runtime catalog should become more accurate as evidence improves, and its score should improve when the CLI becomes more discoverable, safer, more parseable, and easier to recover from.

The score is not the root of trust. Every point traces back to evidence and inferred claims:

- which probes ran
- what they observed
- which claims were inferred
- what confidence each claim has
- which score dimensions changed
- which fixes would improve the score

---

## Document Map

### Overview

| Document | Purpose | Key Decisions |
|---|---|---|
| [Vision and Positioning](overview/vision-and-positioning.md) | Product strategy, OSS role, naming, audience, and adoption path | CLIARE as local-first standard and optional hosted leaderboard |
| [Use Cases and Personas](overview/use-cases-and-personas.md) | Who uses CLIARE and what jobs it solves | Maintainers, agent builders, platform teams, security teams, CI owners |
| [Milestones](overview/milestones.md) | Shipped milestones, partial work, aspirational direction, and tracker policy | Current implementation is separated from future roadmap claims |

### Architecture

| Document | Purpose | Key Decisions |
|---|---|---|
| [System Architecture](architecture/system-architecture.md) | Components, data flow, storage, CLI commands, plugin boundary | Probe -> Evidence -> Inference -> Shape -> Score -> Report |
| [Probe Sandbox Runtime](architecture/probe-sandbox-runtime.md) | How CLIARE safely exercises arbitrary binaries | Temp HOME, network policy, filesystem diffing, timeouts, profiles |
| [Checkpointing and Resume](architecture/checkpointing-and-resume.md) | Cache reuse, detached jobs, progress logs, artifact lifecycle, and checkpoint behavior | Artifact cache, detached jobs, and internal probe-level resume are current; public replay/rescore is future work |
| [Rust Runtime Engineering](architecture/rust-runtime-engineering.md) | Current Rust crate layout, bounded Tokio probing, sandbox/process execution, cache/jobs, and runtime invariants | Single-crate implementation with deterministic planner, typed errors, and bounded subprocess execution |
| [Operational Contracts](architecture/operational-contracts.md) | Current cache, runtime context, guard, policy, sandbox, dependency, score-model, and reproducibility contracts | Artifact cache, guard, policy, and contexts are current; certification and replay are future work |

### Model

| Document | Purpose | Key Decisions |
|---|---|---|
| [Evidence and Command Shape Spec](model/evidence-and-command-shape-spec.md) | Evidence log schema and normalized command-shape IR | Every inferred fact carries provenance and confidence |
| [Computational Scoring Model](model/computational-scoring-model.md) | Probabilistic scoring model, Bayesian updates, and calibration theory | Posterior expected utility, proper scoring rules, calibration |
| [Scoring and Improvement Tracking](model/scoring-and-improvement-tracking.md) | Subscores, regressions, monotonic improvements, baselines | Separate known-surface, capability-adjusted, and whole-surface scores |
| [Generic Inference Processor](model/generic-inference-processor.md) | Current framework-agnostic inference path: layout extraction, confidence-scored claims, deterministic confirmation probes, and shape/index emission | CLIARE on CLIARE uses the generic processor; Clap is not an inference assumption |

### Operations

| Document | Purpose | Key Decisions |
|---|---|---|
| [CI, Publishing, and Calibrated Leaderboards](operations/ci-leaderboard-and-publishing.md) | Local CI execution, evidence-backed scorecard publishing, badge strategy | modiqo hosts artifacts and scorecards, not untrusted binary execution; ranking waits for calibration |
| [QA, Benchmarking, and Calibration](operations/qa-benchmarking-and-calibration.md) | Test strategy, benchmark corpus, fixtures, ground truth | Synthetic CLIs plus real CLIs with human-verified truth sets |
| [Calibration and Leaderboard Authority](operations/calibration-and-leaderboard-authority.md) | Truth corpus, calibration metrics, certified profiles, provenance, anti-gaming, and leaderboard authority requirements | Public ranking requires calibrated models, stability reports, verification levels, and reproducible certified profiles |
| [CLI Benchmark Corpus Tracker](operations/cli-benchmark-corpus-tracker.md) | Current benchmark manifests, low-pretraining launch corpus, vendor candidates, and backlog coverage | Manifests are source of truth; expected bands are QA signals, not calibrated truth labels |
| [Calibration Workflow TODO](operations/calibration-workflow-todo.md) | Future implementation tracker for proposed `calibrate init`, `calibrate check`, and `calibrate evaluate` commands | Truth sets and metrics come before fitting or `cliare-score-v1` certification |

### Guides

| Document | Purpose | Key Decisions |
|---|---|---|
| [Reference CLI Behavior Guide](guides/reference-cli-behavior-guide.md) | Practical guidance for CLI maintainers | How to improve score and agent usability |
| [Persona Outcome Packets](guides/persona-outcome-packets.md) | Current persona-specific reports generated from one measurement run | One evidence-backed measurement produces maintainer, harness, platform, security, OSS, DevRel, and research packets |
| [Reviewable Issue Ledger and Persona Views](guides/reviewable-issue-ledger.md) | Current issue ledger, maintainer dispositions, issue list views, evidence context, and persona projections | Persona reports and `cliare issues list` are filtered views over deterministic, reviewable issues |
| [Agent-Ready CLI Standard Template](guides/agent-ready-cli-standard-template.md) | Runtime behavior guidance for CLIs that want strong agent and CLIARE compatibility | Help, output, diagnostics, preconditions, fixtures, and CI artifacts form the measurable baseline |
| [Agent Skill Installation](guides/agent-skills-installation.md) | Current installable CLIARE review skills and persona commands for agent tools | Claude, Codex, and Cursor can inspect CLIARE artifacts through the same table-first review discipline |
| [Maintainer Issue Dispositions](guides/maintainer-dispositions.md) | Maintainer review workflow for accepting, rejecting, deferring, or fixture-gating CLIARE issues | Dispositions annotate evidence instead of suppressing findings |
| [Maintainer Playbook](guides/maintainer-playbook.md) | End-to-end maintainer command sequence from first measurement through CI and agent-surface publishing | Measure, view, act, disposition, remeasure, gate, publish |

### Papers

| Document | Purpose | Key Decisions |
|---|---|---|
| [Runtime Evidence for Agent-Ready Command-Line Interfaces](papers/runtime-evidence-for-agent-ready-clis.md) ([PDF](papers/runtime-evidence-for-agent-ready-clis.pdf)) | Technical paper covering the motivation, architecture, inference model, score semantics, evaluation strategy, calibration boundary, and research agenda | CLI-agent-readiness should be measured from runtime evidence produced by the released executable |

---

## Architecture at a Glance

```
User CI / local machine
    |
    | shell: cliare measure ./dist/mycli --out .cliare/mycli
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
| - context metadata      |
| - env policy            |
| - filesystem diffing    |
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
| Command Index           |
| command-index.json      |
| command-index.md        |
| agent-facing lookup     |
+-----------+-------------+
            |
            v
+-------------------------+
| Scoring Engine          |
| - expected utility      |
| - deterministic model   |
| - guard deltas          |
+-----------+-------------+
            |
            v
+-------------------------+
| Reports and CI Outputs  |
| scorecard.json          |
| report.md               |
| summary.md              |
| findings.sarif          |
| junit.xml               |
+-------------------------+
```

---

## First-Class Outputs

CLIARE should produce durable artifacts that separate evidence, inference, command navigation, scoring, and human review:

```
.cliare/
  artifact-map.json
  artifact-map.md
  evidence.jsonl
  shape.json
  command-index.json
  command-index.md
  scorecard.json
  report.md
```

The artifact map is the directory-level navigation contract. It describes the folder kind, file roles, schemas, required and missing artifacts, current job state, and recommended inspection order.

The evidence log is the raw observation record for the completed measurement. Active runs write in-progress evidence and compatible interrupted measurements can resume internally; public replay and rescore commands are future work.

The shape catalog is the raw inferred command surface.

The command index is the command-centric lookup table for agents and maintainers. It summarizes each command's parameters, runtime state, preconditions, output contracts, suitability, gaps, and evidence pointers.

The scorecard is the compact CI and local improvement artifact.

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

CLIARE is ready for certified public scoring when:

1. Two independent implementations can consume and produce compatible `artifact-map.json`, `shape.json`, `command-index.json`, and `scorecard.json`.
2. A CLI maintainer can run CLIARE in CI without modiqo cloud access.
3. A score is reproducible from evidence.
4. Score changes decompose into understandable improvements or regressions.
5. The benchmark corpus includes synthetic and real-world CLIs with public ground truth.
6. Public leaderboard entries are distinguishable by verification level.
7. The improvement guide tells maintainers exactly how to raise their score.
