# Agent CLI Evaluation Plan

> **Scope:** Proposed evaluations for CLIARE's developer feedback and agent-shape product use cases.
> **Status:** Design proposal; not current implementation.

---

## Evaluation Goals

CLIARE should be evaluated on two product outcomes:

1. **Developer feedback effectiveness**
   - Maintainers receive actionable findings.
   - Fixing those findings improves actual agent use.
   - Score movement corresponds to measurable harness improvement.

2. **Agent shape usefulness**
   - A harness using `shape.json` and `command-index.json` performs better than
     the same harness using raw terminal probing and docs alone.
   - The harness performs fewer exploratory calls, makes fewer malformed calls,
     avoids commands with unmet preconditions, and parses outputs more reliably.

These outcomes require evaluation beyond the current local fixture tests.

---

## Evaluation Design

### A/B Harness Protocol

For each target CLI and task:

| Condition | Harness Inputs |
|---|---|
| Baseline | Raw terminal access, target binary, standard docs if available. |
| CLIARE Shape | Raw terminal access plus `shape.json`, `command-index.json`, scorecard, and artifact map. |
| CLIARE Shape + Guidance | Shape artifacts plus maintainer-provided fixture/context declarations and recommended invocation templates. |

The agent model, harness, task prompt, sandbox, time budget, and execution
budget must be held constant across conditions.

### What To Measure

| Metric | Meaning | Product Use |
|---|---|---|
| Task success | Did the agent complete the task according to executable tests or state checks? | Validates overall shape usefulness. |
| Step count | Number of shell/tool calls before success or failure. | Measures exploration cost. |
| Help-probe count | Number of commands spent rediscovering the interface. | Shape should reduce this. |
| Malformed invocation rate | Calls rejected due to wrong flags, missing args, bad value forms, or wrong subcommands. | Tests grammar usefulness. |
| Precondition miss rate | Calls that fail because auth, cwd, fixture, network, or runtime dependency requirements were not known or satisfied. | Tests precondition shape. |
| Output parse failure rate | Agent attempts to parse human output or malformed machine-readable output. | Tests output-contract shape. |
| Unsafe action count | Side effects outside allowed paths, credential-like writes, destructive operations, or policy violations. | Tests safety shape. |
| Recovery success | Whether the agent repairs after a rejected or blocked command. | Tests diagnostics and recovery guidance. |
| Token and wall-clock cost | Prompt tokens, observation tokens, runtime duration. | Tests operational value for harnesses. |

---

## Target Corpus

The corpus should include three tiers.

### Tier 1: Controlled Fixture CLIs

Purpose: isolate causal relationships.

Fixture families:

- well-structured CLI with strong help, JSON output, and actionable diagnostics
- identical CLI with weak help and human-only output
- CLI with deep nested commands
- CLI with global and command-local flags
- CLI with required fixture data
- CLI with auth/local-context/network/runtime-dependency blockers
- CLI with dangerous or credential-like side effects
- CLI with stale or misleading help
- CLI with shell-completion data that differs from help output

Each fixture should have known ground truth for:

- command tree,
- flags and value grammar,
- output modes,
- preconditions,
- expected safe and unsafe side effects,
- task solutions.

### Tier 2: Real Developer CLIs

Purpose: test transfer to actual command surfaces.

Initial candidates should cover:

- version control and repository tools,
- package managers,
- build/test tools,
- cloud/vendor CLIs,
- database and SaaS CLIs,
- coding-agent or harness CLIs,
- newer CLIs with lower pretraining likelihood.

Each target should be tagged by:

- framework if known,
- command depth,
- plugin surface,
- auth requirement,
- network requirement,
- machine-readable output support,
- fixture availability,
- destructive-command risk.

### Tier 3: Agent-Harness Tasks

Purpose: measure whether shape helps real harness workflows.

Task classes:

- inspect project status and report next action,
- run tests and parse failures,
- configure or query a local service,
- inspect cloud resource state with safe read-only calls,
- export data in JSON/YAML and transform it,
- recover from missing auth or missing cwd,
- choose between multiple CLIs for the same task,
- avoid a dangerous command and select a dry-run path.

Tasks must be executable and scored by state checks, file checks, structured
output checks, or task-specific validators.

---

## Shape Quality Metrics

CLIARE should evaluate shape quality independently from agent success.

### Ground Truth Accuracy

For fixture and manually audited real CLIs:

| Shape Field | Metric |
|---|---|
| Commands | precision, recall, F1 by command path |
| Aliases | precision, recall by alias mapping |
| Flags | precision, recall by command scope |
| Value grammar | exact match for value kind, arity, requiredness, repeatability |
| Positionals | exact match for order, arity, requiredness |
| Output contracts | precision, recall, parse-success agreement |
| Preconditions | classification accuracy by auth/cwd/fixture/network/runtime-dependency |
| Safety | side-effect recall, credential-like path precision, destructive-affordance recall |

### Confidence Calibration

For each claim family, compare confidence against truth labels:

- reliability diagrams,
- expected calibration error,
- Brier score,
- log loss where applicable,
- precision at confidence thresholds,
- abstention quality when confidence is low.

Initial claim families:

- command existence,
- flag existence,
- output contract existence,
- output parseability,
- precondition classification,
- safety side-effect classification.

### Provenance Completeness

Measure whether shape entries include enough evidence for a harness or reviewer:

- target fingerprint present,
- model id and hash present,
- command/flag evidence references present,
- runtime confirmation evidence present where available,
- output parse evidence present where claimed,
- precondition diagnostic evidence present where claimed,
- safety evidence present where scored.

---

## Maintainer Feedback Evaluation

Developer feedback should be evaluated as an intervention.

### Intervention Protocol

1. Measure target CLI with CLIARE.
2. Record baseline score, shape quality, and harness task success.
3. Apply one category of recommended improvements.
4. Remeasure target CLI.
5. Rerun harness tasks.
6. Compare deltas.

Improvement categories:

- add or improve command help,
- add explicit usage lines and examples,
- add machine-readable output mode,
- add actionable diagnostics,
- add dry-run/preview/read-only mode,
- expose fixture/context requirements,
- stabilize command aliases,
- remove misleading or stale help,
- bound side effects.

### Expected Direction

| Maintainer Change | Expected CLIARE Movement | Expected Harness Movement |
|---|---|---|
| Add complete command help | Discovery and shape recall increase. | Fewer help probes and wrong command paths. |
| Add explicit argument grammar | Grammar score and shape precision increase. | Fewer malformed invocations. |
| Add JSON/YAML output | Output score and parse confidence increase. | Fewer parse failures and lower token cost. |
| Add actionable errors | Recovery score increases. | Better repair after blocked calls. |
| Add precondition diagnostics | Harness shape confidence increases for conditional commands. | Fewer wasted calls under missing auth/cwd/fixtures. |
| Add dry-run/preview | Safety score and readiness increase. | Fewer unsafe exploratory actions. |

---

## Agent Harness Evaluation

The harness should receive the CLIARE shape as structured input, not prose.

### Suggested Harness Contract

Before acting, the harness receives:

```text
target fingerprint
shape.json
command-index.json
scorecard.json
runtime-context.json
artifact-map.json, when available
```

The harness instruction should require:

- prefer high-confidence ready commands,
- verify candidate commands before using them for task state changes,
- satisfy or ask for unmet preconditions,
- prefer machine-readable output contracts,
- avoid safety-sensitive commands unless task intent requires them,
- cite shape evidence when selecting commands.

### Harness Output Trace

To evaluate shape usage, collect:

- commands considered,
- shape entries consulted,
- evidence ids consulted,
- reason for selected command,
- reason for rejected commands,
- whether the command was run as ready, conditional, candidate, or blocked,
- whether the command succeeded,
- whether fallback probing occurred.

This allows CLIARE to measure whether the shape artifact is actually used, not
only whether it exists.

---

## Calibration Splits

The corpus should be split by CLI target, not by individual task alone, to avoid
leakage from similar command surfaces.

| Split | Purpose |
|---|---|
| Train | Fit scoring weights, thresholds, and confidence calibration. |
| Validation | Tune model selection and thresholds. |
| Holdout | Report final calibration and improvement claims. |

Do not mix versions of the same CLI across splits unless the evaluation is
explicitly a version-drift test.

---

## Evaluation Artifacts

Every evaluation run should emit:

```text
cliare-eval.json
cliare-eval.md
shape-quality.json
shape-quality.md
harness-trace.jsonl
agent-task-results.json
calibration-report.json
calibration-report.md
```

These should reference the original CLIARE measurement artifacts rather than
copying them.

---

## Acceptance Gates For Future Certified Scoring

Before promoting a model from experimental to certified:

1. Shape ground-truth metrics must be reported on holdout CLIs.
2. Confidence calibration must be reported for each claim family.
3. Harness A/B results must show statistically meaningful improvement from
   shape access on at least one held-out task family.
4. Maintainer interventions must show that recommended fixes predictably move
   both CLIARE score and harness outcomes.
5. Safety metrics must report false negatives separately from false positives.
6. Model artifact, corpus version, harness version, target versions, and
   evaluator versions must all be hashable and reproducible.

---

## Near-Term Implementation Path

1. Add a shape-quality evaluator for fixture CLIs with known truth.
2. Add harness-trace schema for shape consultation and command selection.
3. Add a small A/B harness runner using one stable local agent harness.
4. Add `shape_confidence` and `maintainer_readiness` views to scorecard output.
5. Expand the benchmark corpus with fixture/context/safety tags.
6. Produce validation and holdout reports before changing public scoring claims.
