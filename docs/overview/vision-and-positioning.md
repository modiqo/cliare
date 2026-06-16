# 01 - Vision and Positioning

> **Scope:** Why CLIARE exists, how it is positioned, and how stewardship supports ecosystem adoption without compromising independence.
> **Status:** Reference Design

---

## Summary

CLIARE is a standard and reference implementation for building evidence-backed runtime catalogs of CLIs used by agents, harnesses, CI systems, and automation.

The project stands on technical merit. It remains useful to teams that never become modiqo customers; that independence is what makes the standard durable.

Public positioning:

> CLIARE turns a released CLI into an evidence-backed runtime catalog, drift signal, and improvement report for agents, CI, and automation.

Stewardship value:

> modiqo becomes associated with rigorous measurement infrastructure for agent-ready command surfaces.

---

## Why This Needs To Exist Now

The agent ecosystem is shifting from API-only tool use toward mixed execution:

- local CLIs
- vendor CLIs
- package-manager CLIs
- infrastructure CLIs
- test runners
- project-specific scripts
- language toolchains
- deployment tools
- data tools
- internal admin tools

Agents often prefer CLIs because they are already installed, authenticated, documented, versioned, and embedded in developer workflows. But CLIs are not designed uniformly for agents.

Human operators can recover from ambiguity:

- they read prose
- they search docs
- they notice context
- they understand dangerous operations
- they tolerate colorful output, pagers, spinners, and prompts
- they can retry interactively

Agents need more structure:

- predictable discovery
- parseable errors
- stable exit codes
- machine-readable output
- safe dry-run modes
- explicit destructive actions
- noninteractive behavior
- consistent syntax
- durable documentation

Today there is no widely accepted runtime evidence standard for that.

---

## Naming

The working name is **CLIARE**.

Expansion:

```text
CLI Agent Readiness Evaluation
```

Pronunciation:

```
clear
```

Why it works:

- It contains CLI without reducing the project to a shell or terminal pun.
- It points at clarity, which is the central quality being measured.
- It works as the name of a tool, a catalog format, a scorecard format, a CI action, and a calibrated publishing surface.
- It can stand for both the tool and the standard.

Suggested surfaces:

```
cliare
cliare report
cliare guard
cliare describe
cliare calibrate
cliare certify
cliare publish
CLIARE Command Shape
CLIARE Command Index
CLIARE Evidence Log
CLIARE Score
CLIARE Badge
```

Potential tagline:

```
Map how your CLI behaves for agents.
```

More technical tagline:

```
Black-box runtime catalogs, drift detection, and calibrated readiness signals for agent-operated CLIs.
```

---

## Product Shape

CLIARE should be three things at once:

1. **A standard**
   - JSON schemas
   - evidence model
   - command shape and command index model
   - score model governance
   - calibration model
   - CI semantics
   - benchmark rules

2. **A reference implementation**
   - local runner
   - sandbox runtime
   - inference engine
   - scoring engine
   - reports
   - GitHub Action

3. **An ecosystem**
   - evidence bundles
   - verified scorecards
   - calibrated badges
   - public leaderboard after calibration
   - benchmark corpus
   - CLI improvement guide

The reference implementation should be useful without any hosted service.

The hosted layer should be optional:

- scorecard publishing
- evidence and drift trend charts
- calibrated leaderboard after score-model certification
- historical trend charts
- team dashboards
- enterprise policy gates
- cross-version drift analysis

---

## Trust Boundary

The default trust model should be local-first.

CLIARE should not require users to upload binaries to modiqo. Arbitrary CLI execution is a risky and expensive service boundary. It would slow adoption and create security objections.

Instead:

1. The project builds its CLI in its own CI.
2. CLIARE runs in that CI environment.
3. CLIARE produces a scorecard.
4. The scorecard can be published to modiqo or anywhere else.

This mirrors how coverage, security scan, and benchmark artifacts often work.

The hosted layer validates provenance and displays results. It does not need to execute untrusted code.

---

## Verification Levels

Public score publishing needs to prevent obvious gaming while staying accessible. Leaderboards should remain downstream of calibration, not the first adoption surface.

Use verification tiers:

| Level | Name | Meaning |
|-------|------|---------|
| 0 | Self Reported | User uploaded a scorecard without provenance |
| 1 | CI Attested | Scorecard came from a known CI provider with OIDC or signed artifact |
| 2 | Evidence Attested | Redacted evidence hashes or full evidence logs are available |
| 3 | Reproducible | A third party can rerun the same release and get equivalent results |
| 4 | Certified | modiqo or a trusted auditor reran the benchmark under a controlled profile |

Any public leaderboard should display the score, score model, profile, calibration state, and verification level together.

This avoids the false binary of "trusted" vs "untrusted."

---

## Ecosystem Adoption

CLIARE should be adopted because it gives maintainers immediate engineering value. A run should answer:

- "What command surface did CLIARE observe?"
- "What changed since the last release?"
- "Which commands, flags, outputs, and preconditions can agents rely on?"
- "Why did the score drop?"
- "What do we fix to improve the catalog, diagnostics, outputs, and score?"
- "Are we ready to publish an evidence bundle or calibrated badge?"
- "How do we compare to other CLIs?"

The open-source project should let any maintainer run:

```sh
cliare measure ./dist/mycli --out .cliare/mycli --profile standard --refresh
```

and get a useful report.

The ecosystem benefits first when projects can publish a concise, evidence-backed catalog:

```text
CLIARE runtime catalog
Evidence-backed command index
Drift and remediation report
```

The hosted surface should begin as a place to publish and compare evidence-backed scorecards, command catalogs, and drift history. Public ranking should wait for calibrated models, certified profiles, and reproducible verification.

---

## Principles

### 1. Runtime Evidence Over Documentation Claims

Documentation matters, but observed runtime behavior is stronger evidence.

If help says `--format json` exists but runtime rejects it, CLIARE should treat that as a contradiction.

If completion exposes a hidden command and help omits it, CLIARE should preserve both facts with different confidence and visibility classifications.

### 2. Confidence Is Part Of The Contract

Every inferred fact must carry confidence.

Bad:

```json
{ "flag": "--output", "arity": "one" }
```

Good:

```json
{
  "flag": "--output",
  "arity": "one",
  "confidence": 0.91,
  "evidence": ["help_usage", "runtime_accept", "missing_value_error"]
}
```

### 3. Scores Are Derived From Evidence

The score is not the product's root of trust. It is a derived summary over evidence and inferred claims. The primary artifacts are the evidence log, command index, issue ledger, persona reports, and scorecard.

Every report should answer:

- What did CLIARE observe?
- What did it infer?
- How confident is it?
- Which findings affected the score?
- Which changes would improve the score?

### 4. Safety Is Not A Footnote

A CLI that is easy to use but easy to misuse should not score highly.

Agent readiness includes:

- noninteractive safety
- dry-run support
- clear destructive verbs
- confirmation behavior
- network and filesystem visibility
- scoped auth
- deterministic failure modes

### 5. Local CI Is The Default

CLIARE runs where the binary already exists:

- GitHub Actions
- Buildkite
- GitLab CI
- CircleCI
- local developer machine
- release pipeline

modiqo cloud should receive scorecards, not binaries, by default.

### 6. Improvement Must Be Measurable

The primary objective is improvement, not ranking.

A maintainer should be able to make a change such as:

- add `--json`
- add `--dry-run`
- fix unknown flag errors
- stabilize exit codes
- expose completion
- document hidden commands

and see the relevant subscore improve.

---

## Public Positioning

Suggested README opening:

```text
CLIARE builds an evidence-backed runtime catalog for your CLI.

It runs your command-line tool under bounded probes, infers its command shape
from runtime evidence, detects drift, and emits a command index, issue ledger,
scorecard, and CI-friendly improvement report.
```

Suggested badge:

```text
[CLIARE catalog | evidence-backed]
```

Suggested comparison:

```text
CLIARE is analogous to test coverage plus compiler diagnostics for CLI usability:
it does not prove a CLI is perfect, but it shows what agents can reliably
discover, execute, parse, and recover from.
```

---

## Success Criteria

The project succeeds if it does three things well:

1. It gives maintainers a runtime catalog and drift report they trust.
2. It gives agents a catalog grounded in runtime evidence.
3. It gives the ecosystem a shared language for CLI readiness and calibration.

A useful catalog creates its own adoption pressure: maintainers can improve it, agent builders can consume it, platform teams can gate on drift and evidence quality, and security teams can review the observations behind every claim. Scores remain useful, but their authority depends on calibration.

CLIARE can serve each of those audiences only if the measurement remains rigorous, reproducible, and transparent.
