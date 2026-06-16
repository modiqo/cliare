# 15 - Generic Inference Processor

> **Scope:** Framework-agnostic CLI shape inference from runtime evidence.
> **Status:** Current implementation reference, with future direction called out separately.
> **Priority:** Core design constraint. CLIARE must not treat any single CLI framework's help format as the protocol.

---

## Summary

CLIARE infers a CLI's command shape from runtime observations. It does not assume the target was built with Clap, Cobra, Click, argparse, oclif, or any other parser framework.

CLIARE itself uses Clap for its own command surface. When CLIARE measures CLIARE, that fact is not used as a shortcut. The binary is still treated as an unknown executable, and shape is inferred through the same help, diagnostic, output, and side-effect observations used for any other target.

The current processor is:

```text
runtime probe results
  -> shape observations
  -> help/layout extraction
  -> confidence-scored claims
  -> deterministic confirmation probes
  -> shape.json and command-index artifacts
```

Help text is evidence, not truth. Runtime confirmation is stronger evidence. Target CLI failures are treated as evidence unless CLIARE itself fails to run the probe.

---

## Current Implementation

The generic processor is implemented across these modules:

| Module | Responsibility |
| --- | --- |
| `layout` | Parse help-like text into rows, usage syntax, command candidates, flag candidates, positional arguments, output-mode candidates, and extraction profiles. |
| `layout_tokens` | Token-level helpers for flags, placeholders, command-like cells, and column splitting. |
| `layout_usage` | Usage-line parsing for command scope and positional arguments. |
| `claims` | Convert observations into command, flag, positional, output-contract, and precondition claims. |
| `belief` | Apply deterministic log-odds confidence updates for claim existence. |
| `planner` | Schedule follow-up probes from current claims with depth, deduplication, and convergence bounds. |
| `output` | Classify output-mode probe results as JSON, YAML-like, table-like, plain text, help text, empty, failed, or unparseable. |
| `diagnostic` / `precondition` | Classify target diagnostics and runtime preconditions. |
| `shape` | Emit `shape.json`, `command-index.json`, and `command-index.md`. |

There is no standalone `cliare infer` command. Inference runs inside `cliare measure`.

---

## Evidence Collection

Measurement starts with safe bootstrap probes:

```text
<target> --help
<target> -h
<target> help
<target> --version
<target> version
<target> <invalid-command>
<target> --<invalid-flag>
```

As claims are discovered, the planner schedules follow-up probes such as:

```text
<target> <command-path> --help
<target> help <command-path>
<target> <command-path> <invalid-child>
<target> <command-path> --<invalid-flag>
<target> <command-path> <output-fragment>
<target> <command-path> <output-fragment> --help
```

Root `--help` and `-h` are seeded during bootstrap. For discovered command paths, direct `<command-path> --help` is the canonical confirmation probe. `help <command-path>` is treated as an optional compatibility form. If direct command help succeeds and `help <command-path>` fails, the command is not marked as help unavailable; CLIARE records the lower-severity `alternate_help_form_unavailable` gap.

Output-mode probes are skipped when the command has required positionals and CLIARE does not have safe operand values. The resulting command index can mark those contracts as needing fixtures rather than executing an unsafe or meaningless invocation.

---

## Help Layout Extraction

The help/layout pass is a document-layout pass, not a framework parser.

It extracts signals from:

- indentation
- blank-line grouping
- row-like aligned columns
- section headers
- command-like first cells
- dash-prefixed flag tokens
- placeholder and metavariable tokens
- usage lines
- examples that advertise output modes
- backspace/control-character patterns that indicate formatted manpage output

Current production code does classify some section titles as command, flag/global, example, or non-command signals. Those titles are supporting evidence and filters; they are not a hard-coded framework truth path. Candidate commands still flow through runtime confirmation.

Manpage-like output is handled conservatively. For non-root command help, CLIARE suppresses child-command discovery from formatted manpage output to avoid treating wrapped prose as subcommands.

---

## Claims And Confidence

`ClaimSet::from_observations` rebuilds the current claim set from observed probe results. Claims are not stored in a separate graph database and are not currently replayed through a standalone evidence-replay command.

Current claim families include:

- command existence, aliases, summaries, usage observation, runtime confirmation, invalid-child diagnostics, invalid-flag diagnostics, alternate help compatibility, and preconditions
- flag existence, short aliases, summaries, value kind, value name, requiredness, repeatability, and scope
- positional arguments from matching usage syntax
- output contracts advertised through flags or examples
- runtime preconditions such as authentication, local context, fixture data, network availability, and runtime dependency availability

Command and flag confidence use deterministic log-odds updates:

```text
log_odds_new = log_odds_old + evidence_weight
confidence = sigmoid(log_odds_new)
```

The priors and inference weights come from the active score model. This keeps claim confidence deterministic for the same evidence and model version. The numbers should be read as model confidence scores, not as a formally validated statistical posterior.

---

## Confirmation Planning

The planner is deterministic. It keeps a bounded queue of `ProbePlan` values and deduplicates scheduled argument vectors.

The planner currently accounts for:

- maximum command depth
- already scheduled probe arguments
- expected-value convergence threshold
- direct help confirmation before compatibility help
- invalid-child probes only when child candidates exist
- invalid-flag diagnostic probes after runtime confirmation
- output-mode probes and output-mode help precedence probes
- skipping output-mode execution when required positionals need fixtures

Planner progress logs include scheduled and completed probe counts. Percent progress is based on completed probes divided by the configured max-probe budget, not divided by the current frontier size.

---

## Shape And Command Index Emission

`shape.json` is the lower-level command-shape artifact. It includes:

- target fingerprint
- commands
- flags
- output contracts
- gaps
- inference model metadata

Command entries include command IDs, paths, argv forms, summaries, aliases, positionals, usage observation, confidence, runtime state, preconditions, and evidence references.

Flag entries include command scope, long and short names, summary, value kind, value name, requiredness, repeatability, confidence, and evidence references.

Output contracts include mode, flag name, argv fragment, scope, advertised/probed state, parse success, observed kind, help precedence behavior, preconditions, diagnostics, and evidence references.

`command-index.json` and `command-index.md` are the agent-facing artifacts derived from the shape. They summarize agent suitability as `ready`, `conditional`, `needs_fixture`, `blocked`, or `candidate`.

---

## Safety Boundary

The generic processor must stay conservative around side effects and missing operands.

CLIARE can safely run help probes, invalid-command diagnostics, invalid-flag diagnostics, and output-mode probes only when the invocation is bounded and does not require unknown operands. When a command requires values such as `<project_id>`, `<file>`, `<endpoint_url>`, or similar, CLIARE should prefer a fixture requirement over inventing values.

This is why a command can be correctly discovered while still having output contracts marked as `needs_fixture` or `unprobed`.

---

## What Is Not Implemented

The current generic inference processor does not implement:

- shell-completion discovery as an inference input
- a standalone evidence replay command
- a standalone `cliare infer` command
- plugin recognizers for individual frameworks
- external documentation crawling
- an LLM parser in the core path
- embedding-based command matching
- a database-backed claim graph
- formally calibrated probability distributions for every emitted field

These may be useful later, but they should be introduced only if they preserve deterministic evidence, bounded probing, and explainable artifacts.

---

## Future Direction

Possible future extensions:

- add optional shell-completion probes as evidence when they can be run safely
- add an evidence replay/rescore command for regression analysis
- add fixture packs so maintainers can validate commands that require safe operands
- improve calibration with curated CLI corpora and proper scoring rules
- add optional framework-specific weak priors that emit generic claims and never bypass runtime validation

Any future processor extension must maintain the same boundary:

```text
framework-specific signal -> generic claim -> runtime evidence -> emitted shape
```

No framework-specific helper should become an authoritative truth path.
