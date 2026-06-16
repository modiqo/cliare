# 11 - Milestones

> **Scope:** What CLIARE has already shipped, what is partially implemented, what remains aspirational, and how future trackers should be created.
> **Status:** Living roadmap.

---

## Summary

CLIARE is no longer just an implementation plan. The core local-first measurement loop exists:

```text
target CLI
  -> runtime probes
  -> evidence.jsonl
  -> inferred shape.json
  -> command-index.json / command-index.md
  -> scorecard.json
  -> issues and persona reports
  -> CI summaries and guardrails
```

This document replaces the old phase plan with a milestone ledger. Each milestone is labeled as:

| Label | Meaning |
|-------|---------|
| **Done** | Implemented in the repository and exposed through current commands or generated artifacts |
| **Partial** | Implemented enough to use, but not yet at the standard/certification bar |
| **Aspirational** | Product direction or standard work that should not be described as current behavior |

---

## Current Command Surface

The current top-level commands are:

| Command | Status | Purpose |
|---------|--------|---------|
| `cliare measure` | Done | Probe a target CLI and generate measurement artifacts |
| `cliare jobs status` | Done | Inspect latest foreground or detached measurement progress |
| `cliare guard` | Done | Measure a target and fail when score regresses against a baseline |
| `cliare benchmark` | Partial | Run a manifest-defined corpus and produce benchmark reports |
| `cliare context` | Done | Compare measurements across runtime contexts |
| `cliare report` | Done | Generate persona-specific outcome packets |
| `cliare describe` | Done | Generate an artifact map for a CLIARE artifact directory |
| `cliare skills` | Done | List or install CLIARE artifact-review skills for local agents |
| `cliare issues` | Done | List issues and record maintainer dispositions |
| `cliare playbook` | Done | Print role-specific execution playbooks |
| `cliare metadata` | Done | Print implementation metadata |

Commands such as `cliare replay`, `cliare rescore`, `cliare publish`, `cliare certify`, and `cliare baseline accept` are not implemented today.

---

## Done Milestones

### M1: OSS Foundation

Status: **Done**

Implemented:

- Rust crate named `cliare`
- Apache-2.0 license
- package metadata for crates.io
- release documentation
- GitHub release-binary workflow
- crates.io release workflow
- curl installer path through GitHub release assets
- root README and launch-oriented documentation

Notes:

- The repository is currently a single Rust crate, not the multi-crate workspace described by earlier design sketches.
- The implementation favors a small, auditable runtime over premature crate splitting.

---

### M2: Local Runtime Measurement

Status: **Done**

Implemented:

- target resolution from explicit path or `PATH`
- binary fingerprinting
- safe bootstrap probes
- bounded stdout and stderr capture
- per-probe timeout
- null stdin
- isolated runtime root for default measurement
- controlled HOME, PWD, XDG, temp, and environment behavior
- host execution mode for authenticated or local-context measurements
- runtime context metadata through `runtime-context.json`
- context-suite artifact layout

Current command:

```sh
cliare measure <target-cli> --out .cliare/<target-cli> --profile quick|standard|deep
```

---

### M3: Evidence Log

Status: **Done**

Implemented:

- `evidence.jsonl`
- schema version `cliare.evidence.v1`
- run-start events
- probe-scheduled events
- process-completed events
- run-finished events
- bounded output payloads
- side-effect summaries on process evidence
- runtime sandbox metadata

Current limitation:

- Fresh measurement runs truncate and rewrite `evidence.jsonl`.
- Partial evidence is inspectable but not resumable or replayable through a public command.

---

### M4: Command Shape And Command Index

Status: **Done**

Implemented:

- framework-agnostic help and runtime observation processing
- command candidate inference
- flag candidate inference
- runtime confirmation probes
- alternate help-form handling
- output-contract detection and safe parse probes
- precondition and fixture-needed reporting
- `shape.json`
- `command-index.json`
- `command-index.md`
- agent-facing `AGENT_SKILL.md`

Value:

- Maintainers get an evidence-backed view of the actual CLI surface.
- Agent harnesses get a command index instead of repeatedly rediscovering syntax through trial and error.

---

### M5: Scorecard And Issues

Status: **Done**

Implemented:

- `scorecard.json`
- deterministic score model loading from bundled score-model JSON
- scoring dimensions for discovery, grammar, execution, output, safety, and recovery
- issue generation
- `issues.json`
- `issues.md`
- `cliare issues list`
- `cliare issues mark`
- maintainer dispositions in `issue-dispositions.json`
- support for muting or explaining maintainer-reviewed issues without deleting evidence

Current limitation:

- The score is useful for local improvement and CI regression tracking.
- It is not yet a publicly calibrated certification score.

---

### M6: Persona Reports And Playbooks

Status: **Done**

Implemented:

- maintainer report
- harness report
- platform report
- security report
- OSS report
- DevRel report
- research report
- filtered report output by area or issue
- optional evidence attachment for focused reports
- maintainer, harness, and security playbooks
- report output as Markdown, JSON, or bundle
- human-oriented output for playbooks and issue lists where implemented

Current commands:

```sh
cliare report maintainer --out .cliare/<target-cli> --format markdown
cliare report harness --out .cliare/<target-cli> --format markdown
cliare report security --out .cliare/<target-cli> --format markdown
cliare playbook maintainer --target <target-cli>
```

---

### M7: Jobs, Cache, And Long-Run Ergonomics

Status: **Done**

Implemented:

- progress logs under `<artifact-dir>/jobs`
- `jobs/current`
- progress formula in progress-log headers
- detached measurement through `cliare measure --detach`
- detached stdout and stderr logs
- `cliare jobs status`
- preflight target validation before detached spawn
- active detached-job guard per artifact directory
- artifact-level cache through `measure-cache.json`
- `--refresh` to bypass reusable artifacts

Current limitation:

- This is not probe-level checkpoint/resume.
- Interrupted partial runs are not resumed.

---

### M8: CI Integration

Status: **Done**

Implemented:

- composite GitHub Action in `action.yml`
- measure mode
- guard mode
- baseline score comparison
- allowed score drop
- policy input for guard mode
- CI step summary publishing
- artifact upload
- SARIF output
- JUnit output
- `CLIARE on CLIARE` dogfood workflow
- issue table with disposition hints in the dogfood workflow

Current command:

```sh
cliare guard <target-cli> --baseline <scorecard.json> --out .cliare/<target-cli>
```

Current limitation:

- There is no `cliare baseline accept` command yet.
- Baseline management is currently file-based.

---

### M9: Command Index Registry Workflow

Status: **Done**

Implemented:

- manual GitHub workflow to measure a target CLI
- copies command index, scorecard, summary, issues, and artifact map into `registry/<artifact-id>`
- opens or updates a pull request for review

Workflow:

```text
.github/workflows/extract-command-index-pr.yml
```

Current limitation:

- The workflow exists, but the public registry corpus is not yet curated at launch scale.
- Registry entries should be reviewed before being treated as canonical agent guidance.

---

## Partial Milestones

### M10: Benchmarking And Calibration

Status: **Partial**

Implemented:

- benchmark command
- manifest-driven target corpus
- target concurrency
- benchmark lock file
- aggregate benchmark JSON and Markdown reports
- local corpus manifests and reporting

Not done:

- public truth sets
- human-verified command-surface labels
- published calibration metrics
- certified score-model releases
- calibrated leaderboard authority

Interpretation:

- Benchmarks are useful for regression testing CLIARE itself.
- Benchmarks are not yet a public claim that one CLI is objectively better than another.

---

### M11: Security And Side-Effect Review

Status: **Partial**

Implemented:

- sandbox filesystem snapshots
- persistent file-change summaries
- credential-like path detection
- safety scoring from safe-probe side effects
- security persona report
- security playbook

Not done:

- full OS-level syscall tracing
- network egress enforcement across all platforms
- signed security profiles
- public risk certification

Interpretation:

- CLIARE can surface undocumented filesystem behavior and side effects.
- It should not be marketed as a complete malware sandbox or endpoint security product.

---

### M12: Agent Skill Distribution

Status: **Partial**

Implemented:

- generated `AGENT_SKILL.md` per measurement
- local skill listing and installation
- support for Claude, Codex, Cursor, and all-agent installs in the current installer surface
- harness reports that explain how an agent should consume the command index

Not done:

- hosted skill registry
- signed skill bundles
- versioned skill update protocol
- broad harness-specific integrations

Interpretation:

- The command index is the durable agent-facing artifact.
- Skills are useful instructions around the artifact, but they do not replace the command index.

---

### M13: Publishing And Distribution

Status: **Partial**

Implemented:

- crates.io package metadata
- GitHub tag-driven binary release workflow
- release assets with checksums
- installer script included in release assets
- release and changelog docs

Not done:

- Homebrew tap
- package-manager matrix beyond cargo and release assets
- signed binary attestations
- hosted scorecard publishing API

Interpretation:

- Users can install and run CLIARE today.
- Broader distribution channels should be added after release reliability is stable.

---

## Aspirational Milestones

### A1: Probe-Level Checkpoint And Resume

Status: **Aspirational**

Desired:

- run manifest
- scheduler checkpoint
- evidence validation before reuse
- `cliare measure --resume`
- partial-run recovery
- probe-level deduplication
- artifact hashes

Why it matters:

- Large CLIs and authenticated contexts can take long enough that crash recovery becomes important.

Why it is not first:

- Incorrect resume is worse than no resume. It can mix evidence from incompatible binaries, contexts, or probe policies.

---

### A2: Replay And Rescore

Status: **Aspirational**

Desired:

- replay existing evidence into `shape.json`, `command-index.json`, reports, and issues
- rescore existing evidence or shape with a new model version
- deterministic provenance for every replayed artifact

Why it matters:

- Scoring and inference will improve faster than users can remeasure every CLI.

Why it is not current:

- The current implementation writes derived artifacts during measurement. It does not expose a separate replay pipeline.

---

### A3: Certified Score Model

Status: **Aspirational**

Desired:

- public truth corpus
- calibration reports
- model cards
- score-model version governance
- leaderboard-grade verification levels

Why it matters:

- Public scores need calibration and resistance to gaming.

Why it is not current:

- The current score is valuable for improvement loops and CI regressions, but public certification needs independently reviewed truth sets.

---

### A4: Public Registry, Leaderboard, And Badges

Status: **Aspirational**

Desired:

- reviewed command-index registry
- public scorecard publishing
- badges
- category-aware leaderboards
- evidence-backed claims rather than popularity contests

Why it matters:

- Public visibility can motivate maintainers to improve CLI shape for agents.

Why it must be gated:

- A leaderboard without calibration, categories, and provenance can create misleading incentives.

---

### A5: Maintainer Baseline Workflow

Status: **Aspirational**

Desired:

- `cliare baseline accept`
- baseline metadata
- reviewed baseline history
- per-dimension guard policies
- clear upgrade path from local baseline to CI baseline

Why it matters:

- Maintainers need a low-cognitive-load way to bless known-good measurements.

Why it is not current:

- `cliare guard` exists, but baseline acceptance is still a file-management workflow.

---

### A6: Stronger Runtime Isolation Profiles

Status: **Aspirational**

Desired:

- stricter network policy enforcement
- platform-specific sandbox backends where available
- reproducible fixture environments
- richer process and filesystem tracing
- signed execution profiles

Why it matters:

- Enterprise adoption depends on predictable, auditable risk boundaries.

Why it must be incremental:

- Portable sandbox behavior differs sharply by OS. CLIARE should keep the default local measurement model honest and explicit.

---

## Launch Readiness View

CLIARE is launch-ready for these claims:

- It measures a CLI as a black box from runtime evidence.
- It generates an evidence-backed command index.
- It helps maintainers find CLI shape, output, diagnostics, and safety issues.
- It helps agent harnesses avoid rediscovering command syntax at token cost.
- It can run locally and in CI without uploading binaries.
- It can compare scores against a baseline and fail regressions.
- It can surface safe-probe filesystem side effects.

CLIARE should not yet claim:

- public certified rankings
- calibrated universal score authority
- full malware sandboxing
- probe-level resume
- replay or rescore from existing evidence through public commands
- hosted publishing as a required path

---

## Tracker Policy

Future work should be tracked from real demand, not from every aspirational paragraph in the docs.

Recommended tracker creation rule:

1. Keep aspirational milestones in this document until users ask for them or they block launch quality.
2. Convert an aspirational milestone into a GitHub tracker only when there is a concrete use case, reproducer, or community vote.
3. Each tracker should define the current gap, target user, acceptance criteria, non-goals, and required docs update.
4. Do not create implementation trackers that imply unsupported commands already exist.
5. Close the loop by updating this milestone ledger when a tracker ships.

P.S. We should create trackers based on community vote and launch feedback. That keeps CLIARE focused on the features that make CLI maintainers and agent harness builders successful, instead of turning the roadmap into a speculative backlog.
