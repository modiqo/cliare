# 15 - Generic Inference Processor

> **Scope:** Framework-agnostic CLI shape inference from runtime evidence.
> **Status:** Inference Reference
> **Priority:** Core design constraint. CLIARE must not treat any single CLI framework's help format as the protocol.

---

## Framework Boundary

CLIARE's inference boundary is framework-agnostic. Target CLIs may use Clap, Cobra, Click, argparse, oclif, shell scripts, custom parsers, plugin dispatchers, or legacy conventions.

CLIARE itself uses Clap for its own command surface. When CLIARE measures CLIARE, the measurement path still treats the binary as an unknown black-box CLI. Framework-specific clues may contribute weak priors after evidence suggests them, but they do not select a hard-coded parser or bypass the generic processor.

The generic processor is:

```text
runtime evidence
  -> document/layout observations
  -> candidate claims
  -> Bayesian belief updates
  -> confirmation probes
  -> command shape with confidence and evidence
```

Help text is one weak evidence source. Runtime confirmation is stronger evidence. No single help parser is authoritative.

---

## Design Constraints

1. **No framework-specific truth path**
   - Clap-style `Commands:` and `Options:` sections are just one layout pattern.
   - Cobra, Click, custom tabular help, man-page style help, and poor help must all flow through the same claim engine.

2. **No regex-driven command catalog**
   - Regex may be used only for small lexical primitives if needed, not as the architecture.
   - The core processor should use tokenization, indentation, aligned columns, repeated row shapes, and runtime validation.

3. **No semantic section-title allowlist**
   - Do not decide that a row is a command because the nearest heading says `Commands`, `Subcommands`, `Available Commands`, or any localized variant.
   - Section titles are weak layout features at most; they are not gates.
   - The command extractor must prefer structural evidence: contiguous row blocks, repeated alignment, compact invocation cells, command-token morphology, non-prose summaries, and later runtime confirmation.
   - Manpage-style output must be handled as its own format signal. In particular, formatted manpages often contain wrapped prose that looks columnar, so child-command discovery from non-root manpage help should be suppressed unless another strong runtime signal exists.

4. **Help output generates hypotheses**
   - A row in help output can produce `candidate_command` or `candidate_flag`.
   - It does not prove the command or flag exists.

5. **Runtime confirmation updates beliefs**
   - Candidate commands should be probed with safe forms such as `<candidate> --help`, `help <candidate>`, and invalid child probes.
   - Acceptance, rejection, help-like output, and diagnostic suggestions update confidence.
   - Runtime precondition diagnostics are not command nonexistence evidence. They should create a `precondition_blocked` runtime state, preserve positive evidence that the dispatcher recognized the command path, and mark the required precondition such as `auth_required`, `local_context_required`, or `fixture_required`.
   - Precondition classification must use diagnostic features rather than vendor-specific phrase buckets. Labeled recovery blocks, executable examples, action families, neighboring probes, and weak token classes are acceptable evidence. Exact diagnostic sentences belong in calibration corpora and tests, not in production match lists.

6. **Every emitted shape field carries confidence**
   - The shape catalog must say what is known, how strongly it is known, and which evidence supports it.

---

## Processing Model

### Phase 1: Evidence Collection

Run safe probes:

```text
target --help
target -h
target help
target --version
target version
target <invalid-command>
target --<invalid-flag>
```

Later waves add candidate-specific probes:

```text
target <candidate> --help
target help <candidate>
target <candidate> <invalid-child>
target <candidate> --<invalid-flag>
```

### Phase 2: Help Document Layout

Transform raw text into layout blocks:

```text
HelpDocument
  sections[]
    lines[]
      indent
      tokens
      column breaks
      continuation likelihood
      header likelihood
      row likelihood
```

This is not a framework parser. It is a document-layout pass.

Features:

- indentation depth
- blank-line grouping
- uppercase or title-case section header likelihood
- repeated row alignment
- first-cell token shape
- dash-prefixed token presence
- metavariable token presence
- bracket/angle argument hints
- prose-like trailing cell
- continuation-line likelihood
- backspace/control-character patterns that indicate formatted manpage output

The production command-row extractor should not maintain a list of accepted command-section labels. Fixture tests may include words such as `Commands:` and `Options:` because real CLIs print them, but those words must not be the production gate for command discovery.

### Phase 3: Candidate Claims

The layout pass emits claims:

```text
line_is_command_row(line_id)
line_is_flag_row(line_id)
section_contains_commands(section_id)
section_contains_flags(section_id)
candidate_command(["workspace", "ls"])
candidate_flag("--verbose")
candidate_flag_takes_value("--config")
```

Each claim starts with low to moderate confidence and references the evidence.

### Phase 4: Belief Updates

Use lightweight Bayesian log-odds updates:

```text
log_odds_new = log_odds_old + evidence_weight
probability = sigmoid(log_odds_new)
```

Examples:

| Evidence | Claim | Weight |
|----------|-------|--------|
| row-like help layout | command candidate | weak positive |
| command appears in aligned command section | command candidate | medium positive |
| completion lists command | command exists | strong positive |
| `<cmd> --help` exits 0 with help-like output | command exists | very strong positive |
| runtime rejects candidate as unknown | command exists | strong negative |
| invalid flag error names valid flags | flag/domain claims | medium positive |

Weights should be calibrated later with fixtures and proper scoring rules.

### Phase 5: Confirmation Planning

The scheduler ranks claims by expected information gain:

```text
priority =
  uncertainty
+ candidate importance
+ contradiction resolution value
- risk
- cost
```

High-value uncertain command candidates get safe confirmation probes first.

### Phase 6: Shape Emission

Only after evidence and belief updates does CLIARE emit:

```text
commands[]
flags[]
positionals[]
outputs[]
side_effects[]
preconditions[]
contradictions[]
```

Each field includes:

- probability/confidence
- evidence references
- inference model version
- whether runtime-confirmed
- whether runtime-blocked by auth, local context, fixture/input data, profile, dependency, or other preconditions

---

## Lightweight ML Scope

CLIARE may use lightweight statistical models:

- weighted log-odds for binary claims
- Beta-Bernoulli for calibrated binary facts
- Dirichlet-Categorical for line class, output kind, arity, and side-effect class
- Naive Bayes or logistic regression for line/block classification once fixtures exist

CLIARE should not use heavyweight or opaque ML in the core path for v1:

- no LLM parser dependency
- no embedding model dependency
- no large ML runtime
- no nondeterministic model output in certified scoring

The model must remain explainable and replayable from evidence.

---

## CLIARE On CLIARE

CLIARE should measure its own executable like any other unknown CLI:

```sh
cliare measure ./target/debug/cliare
```

That run should not use a special Clap parser. It should use the same generic layout processor and runtime confirmation loop that would process any other CLI.

The fact that CLIARE uses Clap internally is useful because it gives a clean, high-quality help surface. It is not a contract between the target and the inference engine.

---

## Implementation Boundary

The initial implementation should introduce these domain modules:

```text
layout       raw help text -> document blocks and rows
claims       candidate facts with evidence references
belief       Bayesian updates and confidence values
planner      confirmation probes from uncertain claims
shape        final catalog emission
```

Avoid module names like `clap_parser` or `cobra_parser` in the core. Framework-specific helpers may exist later as optional priors or extractors, but they must emit generic claims and never bypass runtime validation.
