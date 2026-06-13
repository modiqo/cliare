# 01 - Vision and Positioning

> **Scope:** Why CLIARE should exist, how it should be positioned, and how stewardship can support ecosystem adoption without compromising independence.
> **Status:** Reference Design

---

## Summary

CLIARE is a standard and reference implementation for measuring CLI readiness for agents, harnesses, CI systems, and automation.

The project should stand on technical merit. It must remain useful to teams that never become modiqo customers; that independence is what makes the standard durable.

Public positioning:

> CLIARE measures how ready a CLI is for agents, CI, and automation, then produces the evidence needed to improve it.

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

Today there is no widely accepted score for that.

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
- It works as the name of a tool, a scorecard format, a CI action, and a hosted leaderboard.
- It can stand for both the tool and the standard.

Suggested surfaces:

```
cliare
cliare score
cliare certify
cliare report
cliare publish
CLIARE Score
CLIARE Badge
CLIARE Command Shape
CLIARE Evidence Log
```

Potential tagline:

```
Measure how ready your CLI is for agents.
```

More technical tagline:

```
Black-box command-shape inference and readiness scoring for agent-operated CLIs.
```

---

## Product Shape

CLIARE should be three things at once:

1. **A standard**
   - JSON schemas
   - evidence model
   - scoring model
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
   - badges
   - public leaderboard
   - verified scorecards
   - benchmark corpus
   - CLI improvement guide

The reference implementation should be useful without any hosted service.

The hosted layer should be optional:

- scorecard publishing
- leaderboard
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

A leaderboard needs to prevent obvious gaming while staying accessible.

Use verification tiers:

| Level | Name | Meaning |
|-------|------|---------|
| 0 | Self Reported | User uploaded a scorecard without provenance |
| 1 | CI Attested | Scorecard came from a known CI provider with OIDC or signed artifact |
| 2 | Evidence Attested | Redacted evidence hashes or full evidence logs are available |
| 3 | Reproducible | A third party can rerun the same release and get equivalent results |
| 4 | Certified | modiqo or a trusted auditor reran the benchmark under a controlled profile |

The public leaderboard should display the score and the verification level together.

This avoids the false binary of "trusted" vs "untrusted."

---

## Ecosystem Adoption

CLIARE should be adopted because it gives maintainers immediate engineering value. A run should answer:

- "How agent-ready is our CLI?"
- "Why did the score drop?"
- "What do we fix to improve?"
- "Can we put a badge in our README?"
- "How do we compare to other CLIs?"

The open-source project should let any maintainer run:

```sh
cliare certify ./dist/mycli
```

and get a useful report.

The ecosystem benefits when projects can publish a concise, evidence-backed signal:

```text
CLIARE Score: 84
Agent-ready CLI
```

The hosted leaderboard is the discovery surface. The improvement report is the engineering artifact that makes the score actionable.

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

### 3. Scores Must Be Explainable

The score is not useful unless it decomposes into action.

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

CLIARE should run where the binary already exists:

- GitHub Actions
- Buildkite
- GitLab CI
- CircleCI
- local developer machine
- release pipeline

modiqo cloud should receive scorecards, not binaries, by default.

### 6. Improvement Must Be Measurable

The primary objective is improvement, not ranking alone.

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
CLIARE measures how ready your CLI is for agents and automation.

It runs your command-line tool in a sandbox, infers its command shape from
runtime evidence, scores discovery, grammar, execution, output, safety, and
recovery, then emits a CI-friendly scorecard and improvement report.
```

Suggested badge:

```text
[CLIARE 84 | agent-ready CLI]
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

1. It gives maintainers a score they want to improve.
2. It gives agents a catalog grounded in runtime evidence.
3. It gives the ecosystem a shared language for CLI readiness.

A useful score creates its own adoption pressure: maintainers can improve it, agent builders can consume the resulting catalogs, platform teams can gate on it, and security teams can review the evidence behind it.

CLIARE can serve each of those audiences only if the measurement remains rigorous, reproducible, and transparent.
