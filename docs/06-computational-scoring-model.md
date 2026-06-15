# 06 - Computational Scoring Model

> **Scope:** Computational model for black-box CLI inference, Bayesian evidence updates, agent-readiness scoring, calibration, and monotonic improvement.
> **Status:** Reference Design

---

## Summary

CLIARE's score is grounded in a formal utility interpretation rather than an ad hoc heuristic. The score is interpretable as:

> The posterior expected utility of an agent using this CLI across a task distribution, given observed runtime evidence.

More compactly:

```text
CLIARE Score = 100 * E[ U(G, T) | E ]
```

Where:

- `G` is the latent true command surface of the CLI.
- `T` is the task distribution or workload.
- `E` is the observed evidence from probing.
- `U` is an agent-readiness utility function.

This reference describes the model and the implementation boundary for the current score engine.

---

## Implementation Map

This reference describes the computational scoring target and the current implementation. The current implementation is an evidence-only v0 score model (`cliare-score-v0`) with log-odds belief updates, deterministic subscores, explicit gaps, and evidence-linked reports. The full calibrated Bayesian model described here is the direction of travel for model v1 and later.

| Model Concept | Rust Implementation |
|---|---|
| Target fingerprint and executable identity | [`src/fingerprint.rs`](../src/fingerprint.rs) |
| Probe scheduling, traversal budgets, and progress | [`src/measure.rs`](../src/measure.rs), [`src/planner.rs`](../src/planner.rs), [`src/jobs.rs`](../src/jobs.rs) |
| Process execution and sandbox side-effect capture | [`src/process.rs`](../src/process.rs), [`src/sandbox.rs`](../src/sandbox.rs) |
| Runtime evidence and normalized shape observations | [`src/evidence.rs`](../src/evidence.rs), [`src/observation.rs`](../src/observation.rs) |
| Framework-neutral layout extraction | [`src/layout.rs`](../src/layout.rs) |
| Log-odds belief state and claim updates | [`src/belief.rs`](../src/belief.rs), [`src/claims.rs`](../src/claims.rs) |
| Output-mode and precondition classification | [`src/output.rs`](../src/output.rs), [`src/precondition.rs`](../src/precondition.rs) |
| Shape and harness command index artifacts | [`src/shape.rs`](../src/shape.rs) |
| Scorecard, dimension weights, findings, and report rendering | [`src/score.rs`](../src/score.rs) |
| Guard policies and CI failure semantics | [`src/guard.rs`](../src/guard.rs), [`src/policy.rs`](../src/policy.rs) |
| Persona packets and reviewable issue ledger | [`src/report.rs`](../src/report.rs) |
| Benchmark corpus execution and calibration inputs | [`src/benchmark.rs`](../src/benchmark.rs) |
| Bundled typed score-model spec and validation | [`score-models/cliare-score-v0.json`](../score-models/cliare-score-v0.json), [`src/score_model.rs`](../src/score_model.rs) |

Normative language in this reference describes the target standard model. Mentions of "current v0" describe the linked Rust implementation.

---

## Computation Model V1 Direction

The score model must be an artifact, not a set of constants hidden in implementation code. CLIARE now treats the current v0 model as a typed bundled model spec:

```text
score-models/cliare-score-v0.json
```

The spec declares:

- model id, schema version, status, source, normalization rule, and display precision
- dimension weights for discovery, grammar, execution, recovery, output, and safety
- scoring coefficients for discovery, grammar, output, recovery, and safety
- finding thresholds for low confirmation, grammar gaps, recovery, and extraction-limited runs
- claim priors and evidence weights that define the current log-odds inference posture
- calibration requirements: train, validation, holdout splits and proper scoring metrics

The Rust implementation loads this spec through [`src/score_model.rs`](../src/score_model.rs), validates invariants, and embeds the model hash in `scorecard.json`. A model version is therefore auditable: changing a weight, threshold, prior, or precision changes the model file and its SHA-256.

Current v0 remains `experimental_partial`; the architectural change is that v0 is now a concrete model artifact that can be calibrated, revised, frozen, and compared instead of an informal set of code literals.

Baseline governance rule:

```text
The model artifact owns both claim inference parameters and score aggregation parameters.
```

In v0 this means command priors, flag priors, output-contract priors, log-odds evidence weights, dimension weights, score coefficients, thresholds, and display precision all live in `score-models/cliare-score-v0.json`. The claims layer receives typed inference parameters derived from that artifact. If a future release tunes one of these values, the change must be represented as a model revision with a new hash rather than an invisible implementation tweak.

### Model Artifact Contract

A score model must be immutable once published. Revisions create a new model id:

```text
cliare-score-v0
cliare-score-v0.1
cliare-score-v0.2
cliare-score-v1
```

Each scorecard should preserve:

```json
{
  "score": {
    "model": "cliare-score-v0",
    "status": "experimental_partial"
  },
  "model": {
    "name": "cliare-score-v0",
    "sha256": "...",
    "normalization": "declared_weight"
  }
}
```

That model hash is as important as the binary fingerprint. Without it, a score cannot be reproduced or compared fairly.

### Train, Validation, Holdout Discipline

The calibration corpus must be split before tuning starts:

| Split | Purpose | Rule |
|---|---|---|
| Train | Fit priors, evidence weights, thresholds, and scoring coefficients. | May be inspected and iterated on frequently. |
| Validation | Select model revisions and check overfitting. | May guide model choice, but should not be used for final claims. |
| Holdout | Report final model quality. | Must remain unseen until the candidate model is frozen. |

The split is by CLI family, not by individual probe rows. For example, if `gh` is used to tune output-field probing, it should not also be used as holdout evidence for the same model version.

### Calibration Objective

The calibration process should optimize for useful, trustworthy measurement rather than a superficially high score:

```text
loss =
  alpha * command_existence_log_loss
+ beta  * flag_arity_log_loss
+ gamma * output_contract_log_loss
+ delta * precondition_classification_loss
+ eta   * false_safe_penalty
+ zeta  * extraction_ambiguity_penalty
```

The model must also satisfy monotonicity constraints:

- runtime-confirmed command evidence cannot reduce command existence confidence
- parseable machine-readable output cannot reduce output utility
- additional unapproved side effects cannot improve safety
- clearer actionable recovery cannot reduce recovery utility
- extraction-limited runs must be labeled as measurement-limited rather than silently blaming the target CLI

### Freeze Criteria

A candidate `cliare-score-v1` should not be frozen until it has:

- human-reviewed truth labels for synthetic and real CLIs
- reported Brier score, log loss, expected calibration error, false-safe rate, false-confirmed-command rate, and depth-weighted recall
- repeated-run stability across clean environments
- certified traversal profiles with fixed depth, probe budget, timeout, isolation, and fixture rules
- published model hash and calibration report

Until then, v0 scores are appropriate for CI improvement loops and benchmark development, not authoritative public ranking.

---

## Running Example Used Throughout

The examples use a fictional CLI named `acmectl`. The command of interest is:

```sh
acmectl project list --format json
```

Assume one CLIARE run observes:

- `e_0001`: `acmectl --help` exits 0 and lists `project`.
- `e_0002`: `acmectl project --help` exits 0 and lists `list`.
- `e_0003`: `acmectl project list --help` exits 0 and shows `Usage: acmectl project list [--format <json|table>] [--org <ORG>]`.
- `e_0004`: `acmectl project list --format json` exits with an auth-required diagnostic in an empty sandbox.
- `e_0005`: the same command under a read-only fixture token exits 0 and emits a JSON array.
- `e_0006`: `acmectl project list --__cliare_unknown_flag__` exits nonzero with an unknown-flag diagnostic.
- `e_0007`: one help probe creates `home/.acme/cache/help.json` inside the sandbox.

The point of the example is not that `acmectl` is good or bad. The point is to show how the same evidence flows through discovery, grammar, execution, output, safety, recovery, scoring, and calibration.

---

## CLI As A Black-Box Runtime System

Treat a CLI as a stochastic transducer:

```text
M: (argv, stdin, env, cwd, fs_state, network_state) -> (exit, stdout, stderr, fs_delta, net_events, duration)
```

The CLI may be deterministic or nondeterministic. It may depend on:

- config files
- plugins
- auth state
- remote services
- time
- current directory
- terminal type
- environment variables

CLIARE does not assume source access. It observes executions.

Running example:

```text
argv      = ["acmectl", "project", "list", "--format", "json"]
stdin     = empty
env       = CI=1 plus sanitized sandbox environment
cwd       = sandbox workdir
fs_state  = fresh isolated HOME/PWD/XDG/TMP tree
network   = whatever the local CI profile permits

observed  = (exit=2, stdout="", stderr="auth required", fs_delta={}, duration=42ms)
```

That observation does not prove that `project list` is broken. It says this runtime path is gated by an auth precondition in the current sandbox. The implementation records that distinction through [`src/precondition.rs`](../src/precondition.rs), [`src/claims.rs`](../src/claims.rs), and [`src/shape.rs`](../src/shape.rs).

---

## Latent Command Surface

Define the latent command surface:

```text
G = (C, R, F, A, V, O, X, S)
```

Where:

- `C` is the set of commands and subcommands.
- `R` is the command tree relation.
- `F` is the set of flags/options.
- `A` is the set of positional arguments.
- `V` is the set of value domains and constraints.
- `O` is the output behavior.
- `X` is the exit-code behavior.
- `S` is the side-effect and safety behavior.

We do not observe `G` directly.

We observe evidence:

```text
E = {e_1, e_2, ..., e_n}
```

Each evidence item is generated by running probes:

```text
p_i = (argv_i, stdin_i, env_i, sandbox_i)
```

Running example:

The latent surface may contain `project list`, `project create`, and `project delete`, but CLIARE only knows what the evidence supports. From `e_0001` through `e_0003`, it can infer a command path `["project", "list"]`, a flag `--format`, a value-bearing grammar, and a positional-free usage form. From `e_0004`, it learns about the auth precondition. From `e_0005`, it can validate JSON output when a fixture is available. From `e_0007`, it learns about a persistent sandbox file change.

The current shape artifact is emitted by [`src/shape.rs`](../src/shape.rs) as `shape.json`; the harness-oriented projection is emitted as `command-index.json`.

---

## Claims

Inference should be represented as claims.

Examples:

```text
c1: command_exists(["mycli", "project", "list"])
c2: flag_exists(command="project list", flag="--format")
c3: flag_arity("--format") = one
c4: value_domain("--format") = {"json", "table", "yaml"}
c5: output_kind("project list --format json") = json
c6: side_effect_class("project list") = read
c7: dry_run_supported("deploy") = true
```

For every claim `z`, CLIARE estimates:

```text
P(z | E)
```

For categorical claims:

```text
P(Z = k | E)
```

For example:

```text
P(side_effect_class = read | E) = 0.82
P(side_effect_class = local_write | E) = 0.11
P(side_effect_class = remote_write | E) = 0.04
P(side_effect_class = destructive | E) = 0.01
P(side_effect_class = unknown | E) = 0.02
```

Running example:

The `acmectl` run generates claims such as:

```text
command_exists(["project", "list"])
flag_exists(command=["project", "list"], flag="--format")
flag_arity("--format") = required_value
value_domain("--format") includes {"json", "table"}
runtime_precondition(["project", "list"]) includes auth_required
output_contract(["project", "list"], "--format json") = advertised
output_contract(["project", "list"], "--format json") = parse_success under fixture
invalid_flag_rejected(["project", "list"]) = true
safe_probe_side_effect(home/.acme/cache/help.json) = cache_write
```

In the current implementation, these are represented by `CommandClaim`, `FlagClaim`, and `OutputContractClaim` in [`src/claims.rs`](../src/claims.rs), then materialized into `shape.json`, `command-index.json`, and issue reports.

---

## Evidence Likelihoods

Each evidence type has a likelihood model.

For a binary claim `z`, Bayesian update:

```text
posterior odds = prior odds * likelihood ratio
```

```text
O(z | e) = O(z) * P(e | z) / P(e | not z)
```

Multiple conditionally independent evidence items:

```text
O(z | E) = O(z) * product_i LR_i
```

In practice, evidence is not fully independent. The first version can use capped likelihood accumulation to avoid overconfidence.

Help text is not modeled as a parser-specific truth source. It is transformed into layout features and candidate claims. A line that looks like a command row increases the posterior probability that a command exists, but runtime confirmation is stronger evidence.

Framework identity is a prior, not a switch. If evidence suggests a target is Clap-like, the prior probabilities for common help and completion conventions may change, but the emitted shape still comes from generic claims and evidence updates.

Example:

```text
help mentions --format
completion suggests --format
runtime accepts --format json
runtime rejects --format with missing value
```

These are correlated but still collectively strong.

Running example:

For `command_exists(["project", "list"])`, `e_0002` contributes weak positive evidence because a parent help layout lists `list`; `e_0003` contributes strong positive evidence because command-specific help matches the probed path; `e_0004` contributes positive existence evidence plus an `auth_required` precondition because the process reached command-specific validation instead of returning "unknown command".

Current v0 implements this as additive log-odds weights in [`src/belief.rs`](../src/belief.rs) and [`src/claims.rs`](../src/claims.rs). For example, a layout row, usage syntax, and matching runtime help raise the command posterior, while non-help output from a supposed help probe lowers it.

---

## Binary Claim Model

Use Beta-Bernoulli for binary claims.

For claim `z`:

```text
theta_z ~ Beta(alpha_0, beta_0)
observations y_i ~ Bernoulli(theta_z)
posterior theta_z | y ~ Beta(alpha_0 + successes, beta_0 + failures)
```

Evidence items are not literally successes/failures with equal weight, so use weighted pseudo-counts:

```text
alpha = alpha_0 + sum_i w_i * positive_i
beta  = beta_0  + sum_i w_i * negative_i
```

Probability estimate:

```text
P(z | E) = alpha / (alpha + beta)
```

Example for `flag_exists("--format")`:

| Evidence | Sign | Weight |
|----------|------|--------|
| help mentions flag | positive | 1.5 |
| completion suggests flag | positive | 2.0 |
| runtime accepts flag | positive | 3.0 |
| runtime rejects unknown flag form | negative | 3.0 |

Weights should be calibrated using benchmark truth.

Running example:

The current v0 command prior is `0.08`. If `project list` receives a parent layout candidate, usage syntax, and matching command-specific help, the implementation adds positive log-odds updates. Conceptually:

```text
logit(P(command_exists)) starts at logit(0.08)
layout candidate        adds positive evidence
usage syntax            adds positive evidence
matching runtime help   adds strong positive evidence
```

The resulting confidence is what appears on the command candidate in `shape.json` and on the harness row in `command-index.json`. A future calibrated model can replace these hand-authored log-odds weights with benchmark-derived pseudo-counts while preserving the same evidence trail.

---

## Categorical Claim Model

Use Dirichlet-Categorical for categorical claims.

For output kind:

```text
K = {json, ndjson, yaml, csv, table, text, mixed, empty, unknown}
theta ~ Dirichlet(alpha_0)
P(K = k | E) = alpha_k / sum_j alpha_j
```

Evidence adds pseudo-counts:

- parseable JSON stdout adds weight to `json`
- valid JSON plus spinner text adds weight to `mixed`
- consistent header/rows adds weight to `table`
- empty stdout with success adds weight to `empty`
- truncated output adds uncertainty

For side-effect class:

```text
K = {metadata, read, cache_write, local_write, remote_write, destructive, auth, interactive, unknown}
```

Evidence examples:

- help command with no fs/net effects: `metadata`
- `list` command with no fs writes: `read`
- created cache file: `cache_write`
- modified fixture file: `local_write`
- network connect attempt: `remote_write` or `network`
- verb `delete` plus `--yes`: `destructive`
- login URL or token file write: `auth`

Running example:

`e_0005` adds evidence that `--format json` can produce parseable JSON. `e_0004` adds evidence that the same output contract can be blocked by `auth_required` in an uncredentialed sandbox. `e_0007` adds evidence to the side-effect category because a help probe created a cache file.

Current v0 does not yet maintain a full Dirichlet posterior for every categorical claim. It records categorical states directly: output contract status, observed output kind, runtime preconditions, command runtime state, and gap kind. Those states are defined and serialized in [`src/shape.rs`](../src/shape.rs), with output classification in [`src/output.rs`](../src/output.rs).

---

## Hierarchical Priors

CLIs often follow framework conventions. CLIARE can detect framework fingerprints:

- clap
- Cobra
- Click
- argparse
- Typer
- oclif
- yargs
- custom

Let `H` be the framework class.

```text
P(z | E) = sum_h P(z | E, H=h) P(H=h | E)
```

Framework-specific priors improve early inference.

Examples:

- clap-like CLIs commonly support `--help`.
- Cobra CLIs often expose `completion`.
- Click completion can be triggered by environment variables.
- argparse help often includes `usage:` and standard `-h, --help`.

These are priors, not truth. Runtime evidence can override them.

Running example:

If `acmectl --help` looks Cobra-like, CLIARE may expect `completion` conventions and `Usage:` blocks to be common. It still must not assume that `project list --format json` works because Cobra, Clap, Click, or any other framework is suspected. The actual command path, flag grammar, precondition, and output contract still come from evidence.

Current v0 intentionally keeps extraction framework-neutral in [`src/layout.rs`](../src/layout.rs) and claim-based in [`src/claims.rs`](../src/claims.rs). Framework fingerprints are a possible prior, not a parser switch.

---

## Discovery Coverage

Discovery scoring is difficult when the total command set is unknown.

Use a capture-recapture estimate from multiple discovery channels.

Let:

- `A` = commands found from help traversal
- `B` = commands found from completion
- `C` = commands found from error suggestions
- `D` = commands found from docs/man pages if available

For two sources, the Lincoln-Petersen estimate:

```text
N_hat = |A| * |B| / |A intersect B|
```

For multiple sources, use log-linear capture-recapture models or a conservative lower-bound estimator.

Coverage:

```text
S_discovery = observed_command_mass / estimated_command_mass
```

Command mass can be weighted by importance rather than count alone.

Important caveat:

If overlap is tiny, the estimate is unstable. The score should show wide confidence intervals instead of pretending certainty.

Running example:

Suppose help traversal discovers 14 `acmectl` command candidates, command-specific help confirms 10, invalid-child diagnostics confirm 1 more parent boundary, and 3 remain unconfirmed. The current v0 scorecard reports discovered commands, runtime-confirmed commands, precondition-blocked commands, observed maximum depth, frontier remaining, candidates skipped by depth, and budget pressure. It does not yet compute a capture-recapture estimate, but the reported coverage fields give the inputs needed for that model.

The relevant implementation path is [`src/planner.rs`](../src/planner.rs) for traversal, [`src/claims.rs`](../src/claims.rs) for runtime confirmation, and [`src/score.rs`](../src/score.rs) for coverage metrics and discovery subscore.

---

## Command Importance

Not every command should weigh equally.

Let `I(c)` be command importance with:

```text
sum_c I(c) = 1
```

Default importance can combine:

- root/help prominence
- command category
- examples frequency
- name heuristics
- observed successful execution
- workload hints
- user-provided weights

Example:

```text
list/show/get/status commands often matter more for agents than obscure admin internals.
deploy/delete may matter more in platform workflows but carry higher risk.
```

CLIARE should publish both:

- unweighted surface score
- importance-weighted agent-readiness score

This prevents hiding a broken high-value command among many good low-value commands.

Running example:

`acmectl project list` is likely high value for agents because it is read-oriented, appears in help, and supports structured output. `acmectl project delete` may also be important, but its agent utility should be risk-adjusted because destructive behavior matters. A workload that focuses on inventory should weight `project list` above `project delete`; a platform operations workload might weight both but require stronger safety gates.

Current v0 does not yet expose user-supplied command importance weights in the total score. It does, however, emit enough command-level state in `command-index.json` for a harness or benchmark runner to layer workload weights on top of the CLIARE artifacts.

---

## Agent-Readiness Utility

Define utility for command `c`:

```text
U(c) =
  D(c)^a
* G(c)^b
* X(c)^c
* O(c)^d
* Safe(c)^e
* R(c)^f
```

Where:

- `D(c)` = discoverability
- `G(c)` = grammar shape quality
- `X(c)` = execution reliability
- `O(c)` = output contract quality
- `Safe(c)` = safety readiness
- `R(c)` = recovery quality

The multiplicative form is useful because a command that is totally unsafe or undiscoverable should not receive a high utility because other dimensions are strong.

For CLI-level score:

```text
U(G, T) = sum_c I_T(c) * U(c)
```

Then:

```text
Score = 100 * E[U(G, T) | E]
```

The public score can use default weights. Teams can supply workload-specific weights.

Running example:

For `acmectl project list`, the command utility is high only if the command is discoverable, its `--format` grammar is known, it can be invoked in a documented precondition state, JSON output parses, help and diagnostics are safe, and invalid inputs fail cleanly. If auth is required but clearly represented as a precondition, the command may be `conditional` rather than unusable. If JSON is advertised but never parseable, output utility should remain low.

Current v0 approximates this at the CLI level through dimension subscores in [`src/score.rs`](../src/score.rs), and at the command level through `agent_suitability`, gaps, preconditions, and output contract status in [`src/shape.rs`](../src/shape.rs).

---

## Subscore Definitions

### Discovery

```text
D(c) = P(command_exists(c) and command_reachable(c) and command_visible(c) | E)
```

For hidden/plugin commands, visibility may be lower but existence can be high.

### Grammar

```text
G(c) =
  P(required_positionals_known | E)
* P(flags_known | E)
* P(flag_arities_known | E)
* P(value_domains_known | E)
* P(no_syntax_contradictions | E)
```

### Execution

```text
X(c) =
  P(valid_invocation_succeeds | E)
* P(invalid_invocation_fails_cleanly | E)
* P(exit_codes_stable | E)
* P(no_hidden_required_state | E)
```

### Output

```text
O(c) =
  P(machine_readable_output_available | E)
* P(output_schema_stable | E)
* P(non_tty_output_clean | E)
* P(stderr_stdout_separation_clear | E)
```

### Safety

Let:

```text
Risk(c) = P(mutating(c) | E) * Impact(c)
Mitigation(c) = P(dry_run_or_confirmation_or_fixture_safe(c) | E)
```

Then:

```text
Safe(c) = 1 - Risk(c) * (1 - Mitigation(c))
```

Clamp to `[0, 1]`.

### Recovery

```text
R(c) =
  P(error_identifies_bad_input | E)
* P(error_suggests_fix | E)
* P(error_lists_valid_values | E)
* P(auth_errors_actionable | E)
```

Running example:

The same `project list` evidence touches every subscore:

| Dimension | Example Evidence | Effect |
|---|---|---|
| Discovery | `project list --help` matches the probed path | Raises command confidence and runtime confirmation. |
| Grammar | Usage shows `--format <json|table>` and `--org <ORG>` | Raises flag and positional grammar quality. |
| Execution | Help probes complete quickly; functional probe is auth-blocked | Does not count as an unknown command; records precondition. |
| Output | Fixture probe emits parseable JSON | Confirms a machine-readable contract. |
| Safety | Help probe creates `home/.acme/cache/help.json` | Lowers safety unless the path is allowed by policy. |
| Recovery | Unknown flag exits nonzero with a diagnostic | Improves recovery. |

Current v0 formulas are implemented in [`src/score.rs`](../src/score.rs). Persona reports then translate the same evidence into actionable issue rows through [`src/report.rs`](../src/report.rs).

---

## Total Score

Candidate public default:

```text
S_total =
  0.20 * S_discovery
+ 0.20 * S_grammar
+ 0.20 * S_execution
+ 0.15 * S_output
+ 0.15 * S_safety
+ 0.10 * S_recovery
```

Where each subscore is a command-importance weighted average.

Current v0 uses deterministic declared-dimension weights from [`score-models/cliare-score-v0.json`](../score-models/cliare-score-v0.json), loaded and validated through [`src/score_model.rs`](../src/score_model.rs):

| Dimension | Current v0 Weight |
|---|---:|
| Discovery | `0.35` |
| Grammar | `0.20` |
| Execution | `0.20` |
| Recovery | `0.15` |
| Output | `0.05` |
| Safety | `0.05` |

These v0 weights intentionally emphasize whether CLIARE can discover and confirm the command surface before making stronger claims about output and safety. A calibrated v1 model can rebalance these weights once benchmark truth sets are mature.

The current v0 display rounds readiness scores to whole points. That avoids implying decimal precision before calibration. A future certified public score should include confidence intervals:

```text
84 [80.1, 87.6]
```

Intervals can be estimated by posterior sampling:

1. sample claim probabilities from posterior distributions
2. compute utility
3. repeat
4. report quantiles

Running example:

If `acmectl` has strong discovery and grammar evidence for `project list`, parseable JSON under fixture, clean invalid-flag recovery, but a cache write during help, the total score should rise in discovery, grammar, output, and recovery while safety reflects the cache side effect. If a later release stops writing the cache file and all else remains the same, the safety subscore and total score should improve.

---

## Proper Scoring Rules

For benchmark calibration, CLIARE should use proper scoring rules against ground truth.

Binary claim Brier score:

```text
Brier = mean_i (p_i - y_i)^2
```

Log loss:

```text
LogLoss = -mean_i [ y_i log(p_i) + (1 - y_i) log(1 - p_i) ]
```

Categorical log loss:

```text
LogLoss = -mean_i log(p_{i,true})
```

Why this matters:

- It rewards calibrated confidence.
- It punishes false certainty.
- It lets the inference model improve scientifically.

The public score is expected utility. The benchmark score for inference quality is proper scoring.

These must remain separate.

Running example:

For calibration, a benchmark maintainer can label the truth for `acmectl project list`:

```text
command_exists = true
--format exists = true
--format json parseable = true under fixture
auth_required_without_token = true
safe_help_side_effect = cache_write
unknown_flag_rejected = true
```

The inference model is then scored against these labels. If CLIARE predicts `P(command_exists)=0.96`, that is a good calibrated prediction only if similarly confident predictions are true roughly 96 percent of the time across the benchmark corpus. This calibration workflow is prepared by [`src/benchmark.rs`](../src/benchmark.rs); certified scoring rules are still a model maturity requirement rather than a completed public leaderboard guarantee.

---

## Monotonic Improvement

The user requirement is important:

> If improvements are achieved, the score should get better.

This requires careful score design.

### Local Monotonicity

For a fixed command and fixed risk profile:

- adding reliable JSON output evidence should not reduce output score
- adding dry-run support should not reduce safety score
- improving error suggestions should not reduce recovery score
- making a flag arity clearer should not reduce grammar score

Formal condition:

If evidence `e+` increases posterior probability of a positive property `q` and does not increase posterior probability of a negative property, then:

```text
S_dimension(E union {e+}) >= S_dimension(E)
```

### Revealed Risk Exception

Sometimes new evidence reveals a hidden risk.

Example:

Before:

```text
P(command mutates remote state) = 0.10
```

After deeper probing:

```text
P(command mutates remote state) = 0.85
```

The score can decrease because the previous score was uncertain, not because the CLI worsened.

Reports must distinguish:

- actual regression
- uncertainty reduction
- newly discovered surface

### Growth Penalty Problem

Adding new commands can lower whole-surface score if the new commands are poorly documented or unsafe.

That is mathematically correct for whole-surface quality, but it can discourage growth.

Therefore CLIARE should publish three related scores:

1. **Agent Readiness Score**
   - weighted by intended workload
   - primary public badge

2. **Surface Quality Score**
   - all discovered public commands
   - penalizes broad poor surfaces

3. **Capability-Adjusted Score**
   - normalizes for useful capability growth
   - avoids punishing a CLI solely for adding well-scoped features

Running example:

If `acmectl` adds `--format json` to `project list` and the fixture probe parses it successfully, output utility should improve. If the same release also starts creating `home/.acme/token` during `--help`, the safety subscore may decrease because the new evidence reveals a serious discovery-time side effect. The report should explain both: the CLI improved its output contract and worsened safe-probe behavior.

Current v0 supports this distinction through separate output and safety findings in [`src/score.rs`](../src/score.rs), side-effect evidence from [`src/sandbox.rs`](../src/sandbox.rs), and persona-specific issue wording in [`src/report.rs`](../src/report.rs).

---

## Score Delta Attribution

For every score change:

```text
Delta = S_new - S_old
```

Decompose by dimension and finding:

```text
+4.2 output: added JSON output to project list/show
+2.1 safety: deploy now supports --dry-run
-1.3 grammar: new command export has unknown positional arity
-0.8 recovery: unknown flag errors no longer include suggestions
```

Mathematically:

Use contribution analysis:

```text
Delta_d = w_d * (S_d_new - S_d_old)
```

For individual findings, use approximate Shapley values or simpler one-at-a-time recomputation:

```text
impact(f) = Score(E with f) - Score(E without f)
```

The initial implementation can use one-at-a-time recomputation.

Running example:

Across two `acmectl` releases, a useful delta explanation would be:

```text
+3.6 output: project list now emits parseable JSON under the read-only fixture
+1.1 recovery: project list rejects unknown flags with a nonzero exit
-0.8 safety: help probes still create home/.acme/cache/help.json
```

Current v0 guard behavior compares scorecards and policy constraints through [`src/guard.rs`](../src/guard.rs) and [`src/policy.rs`](../src/policy.rs). Fine-grained Shapley-style attribution remains future work; the current artifacts already expose enough per-dimension and per-issue evidence to produce reviewable human deltas.

---

## Calibration

A raw Bayesian model with hand-picked weights is not enough for a standard.

CLIARE needs calibration over the model spec:

1. Build benchmark CLIs with known truth.
2. Assign CLI families to train, validation, and holdout splits before tuning.
3. Run probes under fixed certified profiles.
4. Compare inferred claims, output contracts, preconditions, safety classifications, and extraction-quality flags to truth.
5. Fit priors, evidence weights, thresholds, and score coefficients on the train split.
6. Select candidate model revisions using validation metrics.
7. Report final performance once on holdout.
8. Freeze the model file and publish its hash.

Calibration plots:

```text
Claims predicted 0.9 should be true about 90 percent of the time.
Claims predicted 0.6 should be true about 60 percent of the time.
```

Use expected calibration error:

```text
ECE = sum_bins |acc(bin) - conf(bin)| * n_bin / n
```

Certified model versions should publish calibration metrics and the exact score-model artifact.

Running example:

`acmectl` can be included in a calibration corpus only after its truth labels are written down: actual command tree, expected flags and arities, which commands require auth, which output modes parse, and which side effects are expected. CLIARE can then compare inferred confidence values from `shape.json` and `scorecard.json` against those labels. A public leaderboard should not treat the resulting score as authoritative until the model version publishes calibration error and benchmark coverage.

The current benchmark runner in [`src/benchmark.rs`](../src/benchmark.rs) executes real CLI corpuses and aggregates scores. Calibration metrics and authoritative leaderboard normalization are documented as requirements, not as completed certification.

---

## Handling Unknowns

Unknown is not failure. Unknown is uncertainty.

A good report says:

```text
The shape of `deploy ENV` is unknown because no safe invocation was found.
```

Not:

```text
deploy is bad.
```

Unknowns affect confidence intervals and may reduce score if they matter for agent utility.

Running example:

If `acmectl project delete --help` is listed under `project --help` but command-specific help is auth-gated or never probed, CLIARE should not say "`project delete` is broken." It should mark the command as a candidate, conditional, blocked, or needing a fixture depending on the evidence. A harness should read `command-index.json`, see that the command is not ready for blind routing, and either request a precondition or avoid it.

Current v0 encodes this in command runtime states, agent suitability, and gap kinds in [`src/shape.rs`](../src/shape.rs), then renders persona guidance in [`src/report.rs`](../src/report.rs).

---

## Gaming Resistance

Any public score can be gamed. Mathematical design can reduce obvious gaming.

Potential gaming:

- expose fake help commands that do not work
- add `--json` flag that outputs invalid JSON
- mark destructive commands as dry-run but still mutate
- hide commands from discovery
- optimize only for safe probes

Countermeasures:

- runtime validation beats help claims
- dry-run must be side-effect checked
- output must parse and remain stable
- hidden discovered commands affect surface score
- certified profiles include negative probes
- evidence hashes can be audited
- verification tiers disclose provenance

Running example:

If `acmectl project list --help` advertises `--format json` but `acmectl project list --format json` emits a table, the help text raises an advertised-output claim but the runtime parse probe prevents CLIARE from treating the JSON contract as confirmed. If `acmectl --help` lists fake commands, command-specific probes and invalid-child diagnostics should keep those candidates low confidence or unconfirmed.

This is why the implementation keeps help-derived layout candidates in [`src/layout.rs`](../src/layout.rs) separate from runtime confirmation and output validation in [`src/claims.rs`](../src/claims.rs), [`src/output.rs`](../src/output.rs), and [`src/shape.rs`](../src/shape.rs).

---

## Model Versioning

A stable model envelope should include:

```json
{
  "score_model": "cliare-score-v1",
  "inference_model": "cliare-infer-v1",
  "calibration_set": "cliare-bench-2026-06"
}
```

When models change, old scorecards remain valid under their original model. They should be rescorable from preserved evidence:

```sh
cliare rescore evidence.jsonl --model cliare-score-v2
```

Leaderboards should group or normalize by score model.

Running example:

An `acmectl` scorecard generated today should keep `cliare-score-v0` attached to the artifact. If a future `cliare-score-v1` changes likelihood weights, output weighting, or calibration intervals, the old evidence remains useful but the old number should not be compared naively against v1 scores. Rescoring from preserved `evidence.jsonl` is the correct path once model-stable rescoring is implemented.

Current v0 writes score and inference model names through [`src/score.rs`](../src/score.rs) and [`src/shape.rs`](../src/shape.rs). The `rescore` command shown above is part of the model-versioning design, not a completed current command.

---

## Initial Mathematical Scope

The current v0 implementation has shipped:

- log-odds binary beliefs for command and flag claims
- hand-authored evidence weights
- deterministic score formula
- runtime-derived shape and command-index artifacts
- explicit findings and persona issue ledgers
- benchmark corpus execution for calibration inputs

The v1 computational target is:

- calibrated weighted Beta-Bernoulli binary claims
- calibrated Dirichlet categorical claims
- benchmark-derived likelihood weights
- bootstrap confidence intervals or posterior sampling
- Brier score and log-loss evaluation on labeled fixtures
- score delta attribution by subscore and finding

Do not ship a public "standard score" until calibration exists.

Ship early as:

```text
Experimental CLIARE Score
```

Then graduate to:

```text
CLIARE Score v1
```

Running example:

For `acmectl`, the experimental score is useful when it tells a maintainer exactly what changed: `project list` became runtime-confirmed, `--format json` became parseable, the auth precondition is explicit, and help still writes a cache file. It should not yet be marketed as a certified rank against every other CLI. The path from experimental to standard is calibration: the same evidence, claims, and score formulas must be tested against labeled truth across many CLIs and releases.
