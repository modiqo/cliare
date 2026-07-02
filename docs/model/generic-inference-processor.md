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

## Semi-Structured CLI Text Interpretation

Real CLI surfaces vary widely. Some tools expose Clap, Cobra, Click, argparse, oclif, or custom parser output. Some tools expose generated manpages. Some tools expose plugin commands that change with auth, installation state, config, or the current working directory. Some tools do not support `--help` on every path, but still print a useful command menu when a group command is invoked directly. CLIARE treats those surfaces as runtime evidence with different confidence, not as one required format.

The current extractor can operate on semi-structured text when the output has at least some of these conventions:

| Surface pattern | Current interpretation |
|---|---|
| `USAGE` or `Usage:` lines | Strong command-scope and positional-argument signal when the usage path matches the current probe path. |
| Aligned command tables | Candidate command paths and summaries from compact first-column invocation cells and second-column descriptions. |
| Option rows | Candidate flags, aliases, value names, and summaries from dash-prefixed tokens and nearby row text. |
| Examples | Weak output-mode and invocation evidence, especially when examples advertise JSON, YAML, table, or plain-text modes. |
| Missing-argument diagnostics | Runtime evidence that the command path was recognized, with possible operand hints. Current scoring treats invalid probes mainly as diagnostics; richer operand extraction from these messages is future work. |
| Direct group output without `--help` | Useful evidence when collected as a help-like observation. Current traversal does not yet schedule bare group invocations as a first-class confirmation probe for every discovered group. |
| Manpage or pager-like output | Parsed conservatively. CLIARE avoids deep child extraction from non-root formatted manpage output to reduce prose false positives. |
| Free prose with no rows, usage, options, or diagnostics | Usually not enough for command-shape extraction. It may remain evidence text, but it should not become a high-confidence command shape by itself. |

This means CLIARE does not need machine-readable output to build a useful command index. It can build a provisional map from structured plain text. Machine-readable output still matters for output-contract confidence because agents need parseable task results, not only command discovery.

### Nested Row Example

Consider a help row from a command group:

```text
auth scheme add <id> <scheme>  Enable an adapter auth scheme (bearer/api-key/basic/oauth2)
```

If this row appears while probing:

```text
rote adapter --help
```

the current layout extractor reads the first aligned column as an invocation cell:

```text
auth scheme add <id> <scheme>
```

It then walks tokens from left to right:

```text
auth -> command token
scheme -> command token
add -> command token
<id> -> argument-like token, stop command-path extraction
<scheme> -> not part of the command path after stop
```

The extracted relative command path is:

```text
auth scheme add
```

Because the current probe scope is `adapter`, CLIARE emits the candidate command path:

```text
adapter auth scheme add
```

The second aligned column becomes the command summary:

```text
Enable an adapter auth scheme (bearer/api-key/basic/oauth2)
```

At this point the claim is a candidate, not a confirmed contract. A harness should treat it as discoverable but not automatically route through it until deeper evidence confirms it.

### Argument Extraction Boundary

Current positional-argument extraction is stronger when operands appear in a matching usage syntax:

```text
USAGE
  rote adapter auth scheme add <id> <scheme>
```

That matching usage line can attach required positionals such as `id` and `scheme` to the command. A command-table row that includes operands currently helps identify where the command path stops, but does not attach those operands as full positional claims with the same confidence as a matching usage line.

For the row:

```text
auth scheme add <id> <scheme>
```

current output should be understood as:

```text
path: adapter auth scheme add
summary: Enable an adapter auth scheme (bearer/api-key/basic/oauth2)
positionals: unknown unless confirmed by matching usage or diagnostics
runtime_state: candidate until a probe confirms the path
```

The planned improvement is to preserve row operands as provisional positional claims and then upgrade or reject them through matching usage, direct help-like output, or missing-argument diagnostics.

### Deep Traversal Boundary

When a deep candidate path is discovered from a row, the planner can schedule confirmation probes for the discovered path:

```text
rote adapter auth scheme add --help
rote help adapter auth scheme add
```

The current planner does not yet aggressively synthesize intermediate group confirmations from every deep row. From:

```text
adapter auth scheme add
```

a better traversal frontier would prioritize:

```text
rote adapter auth
rote adapter auth scheme
rote adapter auth scheme add
```

That matters for CLIs where group commands print structured text directly, even when `--help` is absent or incomplete. In those cases, direct group output should count as medium-to-high discoverability evidence when it is help-like and path-matched. Canonical `--help` should still provide an additional confidence boost because it is a portable contract that most agent harnesses know to try.

### Harness Interpretation

From the agent perspective, CLIARE's current artifact states should be read as:

| Evidence state | Harness interpretation |
|---|---|
| Layout-only candidate | The command was seen in structured text, but should be verified before automatic use. |
| Direct help-like group output | The group is discoverable and navigable. It can guide exploration even without `--help`. |
| Runtime-confirmed help | The command path is recognized and its surface is reliable enough for routing decisions, subject to gaps. |
| Missing-argument diagnostic | The leaf command likely exists and rejected an incomplete invocation. Use the diagnostic to infer required operands or fixture needs. |
| Machine-readable output probed and parsed | The command has a result contract an agent can consume directly. |
| Output advertised but unprobed | Treat as a promise needing validation before automatic parsing. |

The core product boundary is therefore:

```text
structured text can produce a map
runtime confirmation decides how much to trust the map
parseable output contracts decide whether agents can consume results safely
```

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
- first-class bare group invocation probes for every discovered command group
- provisional positional claims from command-table row operands
- deep-prefix synthesis for every nested command row
- a database-backed claim graph
- formally calibrated probability distributions for every emitted field

These may be useful later, but they should be introduced only if they preserve deterministic evidence, bounded probing, and explainable artifacts.

---

## Future Direction

Possible future extensions:

- add optional shell-completion probes as evidence when they can be run safely
- add an evidence replay/rescore command for regression analysis
- add fixture packs so maintainers can validate commands that require safe operands
- add direct help-like group probes for discovered command prefixes
- preserve command-table operands as provisional positionals before runtime confirmation
- prioritize intermediate prefixes for nested command rows before spending budget on lower-value leaves
- improve calibration with curated CLI corpora and proper scoring rules
- add optional framework-specific weak priors that emit generic claims and never bypass runtime validation

Any future processor extension must maintain the same boundary:

```text
framework-specific signal -> generic claim -> runtime evidence -> emitted shape
```

No framework-specific helper should become an authoritative truth path.
