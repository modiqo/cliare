# 06 - Computational Scoring Model

> **Scope:** Current CLIARE score computation, claim inference, model provenance, calibration direction, and the mathematical boundary between shipped v0 behavior and future certified models.
> **Status:** Current Implementation And Future Calibration Direction

---

## Summary

CLIARE's scoring model has two layers:

1. **Current v0 implementation**
   - A deterministic scorecard generated from runtime evidence.
   - A bundled typed model artifact: [`score-models/cliare-score-v0.json`](../../score-models/cliare-score-v0.json).
   - Log-odds claim inference for commands, flags, and output contracts.
   - Explicit dimension scores, coverage metrics, findings, and model provenance.
   - Status: `experimental_partial`.

2. **Future calibrated model**
   - A fuller Bayesian utility model over a CLI's latent command surface and agent workload.
   - Proper train/validation/holdout calibration.
   - Published calibration metrics, confidence intervals, and frozen model hashes before any authoritative public ranking.

The long-term interpretation is:

```text
CLIARE Score = 100 * E[U(G, T) | E]
```

Where:

- `G` is the latent true command surface of the CLI.
- `T` is the task distribution or workload.
- `E` is the observed evidence from probing.
- `U` is an agent-readiness utility function.

The current v0 score is not yet that full posterior expected utility. It is an auditable deterministic approximation designed for local CI, maintainer feedback, benchmark development, and model iteration.

---

## Implementation Map

| Model Concept | Current Source |
|---|---|
| Target fingerprint and executable identity | [`src/fingerprint.rs`](../../src/fingerprint.rs) |
| Probe scheduling, traversal budgets, and progress | [`src/measure.rs`](../../src/measure.rs), [`src/planner.rs`](../../src/planner.rs), [`src/jobs.rs`](../../src/jobs.rs) |
| Process execution and sandbox side-effect capture | [`src/process.rs`](../../src/process.rs), [`src/sandbox.rs`](../../src/sandbox.rs) |
| Evidence events and shape observations | [`src/evidence.rs`](../../src/evidence.rs), [`src/observation.rs`](../../src/observation.rs) |
| Framework-neutral help/layout extraction | [`src/layout.rs`](../../src/layout.rs) |
| Log-odds belief state and claim updates | [`src/belief.rs`](../../src/belief.rs), [`src/claims.rs`](../../src/claims.rs) |
| Output-mode and precondition classification | [`src/output.rs`](../../src/output.rs), [`src/precondition.rs`](../../src/precondition.rs), [`src/diagnostic.rs`](../../src/diagnostic.rs) |
| Shape and command-index artifacts | [`src/shape.rs`](../../src/shape.rs) |
| Scorecard, dimension weights, findings, and report rendering | [`src/score.rs`](../../src/score.rs) |
| Typed score-model spec, validation, and model hash | [`src/score_model.rs`](../../src/score_model.rs), [`score-models/cliare-score-v0.json`](../../score-models/cliare-score-v0.json) |
| Guard policies and CI failure semantics | [`src/guard.rs`](../../src/guard.rs), [`src/policy.rs`](../../src/policy.rs) |
| Persona reports and reviewable issue ledger | [`src/report.rs`](../../src/report.rs), [`src/issues.rs`](../../src/issues.rs) |
| Benchmark corpus execution | [`src/benchmark.rs`](../../src/benchmark.rs) |

---

## Current v0 Model Artifact

The current model is a bundled JSON artifact, not a set of hidden constants:

```text
score-models/cliare-score-v0.json
```

The artifact declares:

- model schema, id, status, source, normalization, and precision
- dimension weights for discovery, grammar, execution, recovery, output, and safety
- scoring coefficients for each dimension
- finding thresholds
- claim priors
- evidence weights
- calibration requirements for later model maturity

The Rust loader in [`src/score_model.rs`](../../src/score_model.rs):

- parses the bundled model
- validates score ranges, probability ranges, weight sums, and calibration split declarations
- derives a typed claim inference model
- computes a SHA-256 hash over the bundled JSON

Every scorecard embeds the model id and hash:

```json
{
  "score": {
    "total": 84,
    "maintainer_readiness": 84,
    "shape_confidence": 71,
    "model": "cliare-score-v0",
    "status": "experimental_partial"
  },
  "model": {
    "name": "cliare-score-v0",
    "sha256": "...",
    "source": "bundled reference model for local CI and improvement tracking; not certified for public leaderboard authority",
    "status": "experimental partial",
    "normalization": "declared_weight"
  }
}
```

This matters because a score is only comparable when both the evidence and the model artifact are known.

---

## Current v0 Evidence Model

CLIARE treats a CLI as a black-box runtime system:

```text
M: (argv, stdin, env, cwd, fs_state) -> (exit, stdout, stderr, fs_delta, duration)
```

The target may depend on config files, plugins, auth state, remote services, time, terminal type, environment variables, and current directory. CLIARE v0 does not assume source access. It observes process outcomes and, in isolated mode, filesystem side effects inside registered sandbox regions.

The evidence stream is written as `evidence.jsonl`. The current event kinds are:

- `run_started`
- `probe_scheduled`
- `process_completed`
- `run_finished`

`process_completed` carries process status, bounded stdout/stderr capture, observed side effects, and timing. Shape observations derived from that evidence are then used by scoring, reporting, `shape.json`, and `command-index.json`.

---

## Current v0 Claim Inference

The current claim layer estimates confidence for three claim families:

- `CommandClaim`
- `FlagClaim`
- `OutputContractClaim`

It does not yet maintain posterior distributions for every safety class, workload class, or command utility. Side effects, preconditions, invalid-input recovery, and parser extraction quality are scored through coverage metrics and findings rather than through separate public claim posteriors.

For the three current claim families, CLIARE uses additive log-odds updates:

```text
logit(P(z | E)) = logit(prior_z) + sum_i weight_i
```

Where:

```text
logit(p) = ln(p / (1 - p))
P(z | E) = 1 / (1 + exp(-log_odds))
```

The current priors are:

| Claim | Prior |
|---|---:|
| `command_exists` | `0.08` |
| `flag_exists` | `0.12` |
| `output_contract_exists` | `0.12` |

The current evidence weights are:

| Evidence Signal | Weight |
|---|---:|
| `layout_candidate` | `1.0` |
| `usage_syntax` | `0.5` |
| `runtime_help_match` | `4.0` |
| `runtime_help_observed` | `0.5` |
| `runtime_precondition_block` | `2.0` |
| `runtime_rejection` | `0.5` |
| `alternate_help_unavailable` | `-0.25` |
| `non_help_output_from_help_probe` | `-2.0` |

These values live in the model artifact and are validated through [`src/score_model.rs`](../../src/score_model.rs). They should not be duplicated as independent constants in scoring or claim code.

Example:

```text
logit(P(command_exists("project list"))) starts at logit(0.08)
+ layout_candidate
+ usage_syntax
+ runtime_help_match
```

That final probability is serialized as command confidence in the derived artifacts.

---

## Current v0 Scorecard

The scorecard schema is:

```text
cliare.scorecard.v1
```

A scorecard contains:

- `target`
- `runtime_context`
- `score`
- `subscores`
- `coverage`
- `findings`
- `model`

The score status is currently always:

```text
experimental_partial
```

The model normalization rule is:

```text
declared_weight
```

The score display precision is whole points.

The current score summary includes three whole-point views:

| Field | Meaning |
|---|---|
| `total` | Compatibility headline score computed from the weighted v0 dimensions. |
| `maintainer_readiness` | Current maintainer-facing readiness view. In v0 this is intentionally equal to `total`; future calibrated models may separate it from the compatibility total. |
| `shape_confidence` | Experimental harness-facing view estimating how much an agent can rely on the emitted shape before additional probing. |

---

## Current v0 Total Score

The current total is the weighted average of measured dimension scores:

```text
S_total = round(sum_d weight_d * S_d / sum_d weight_d)
```

In v0 all six dimensions are populated as measured dimensions, so:

```text
sum_d weight_d = 1.0
```

The current dimension weights are:

| Dimension | Weight |
|---|---:|
| Discovery | `0.35` |
| Grammar | `0.20` |
| Execution | `0.20` |
| Recovery | `0.15` |
| Output | `0.05` |
| Safety | `0.05` |

These weights intentionally emphasize whether CLIARE can discover and confirm the command surface before making stronger claims about output and safety. A later calibrated model may rebalance them.

---

## Current v0 Shape Confidence View

`shape_confidence` is an experimental view derived from existing score coverage
and claim metrics. It is not yet calibrated against harness A/B task success.
It is designed as the first implementation step toward a harness-facing
confidence signal.

The current view is:

```text
S_shape = 100 * (
  w_claim * claim_confidence
+ w_runtime * command_recognition_rate
+ w_grammar * grammar_completeness
+ w_output * output_contract_confidence
+ w_precondition * precondition_clarity
+ w_safety * safety_observation_confidence
)
```

The weights live in `score-models/cliare-score-v0.json` under
`views.shape_confidence`:

| Component | Weight |
|---|---:|
| `claim_confidence` | `0.25` |
| `runtime_confirmation` | `0.20` |
| `grammar_completeness` | `0.15` |
| `output_contract` | `0.15` |
| `precondition_clarity` | `0.10` |
| `safety_observation` | `0.15` |

Current component definitions:

| Component | Definition |
|---|---|
| `claim_confidence` | Average command confidence, or average of command and flag confidence when flags are discovered. |
| `command_recognition_rate` | Runtime-confirmed plus precondition-blocked commands divided by discovered commands. |
| `grammar_completeness` | Average of confirmed-command grammar completeness and known flag grammar. |
| `output_contract_confidence` | Parse successes divided by discovered machine-readable output contracts. |
| `precondition_clarity` | `1.0` when no probes are precondition-blocked; otherwise actionable precondition diagnostics divided by precondition-blocked probes. |
| `safety_observation_confidence` | `0.0` when side-effect observation is unsupported or truncated; otherwise one minus changed-probe and credential-like side-effect rates, clamped to `0..=1`. |

This view deliberately stays conservative. Missing machine-readable output
contracts contribute no output-contract confidence. Host-mode safety observation
is treated as unmeasured for shape confidence rather than as proof of safety.

---

## Current v0 Dimension Formulas

This section is normative for the current implementation.

### Discovery

Discovery combines runtime recognition and average command confidence:

```text
command_recognition_rate =
  (commands_runtime_confirmed + commands_precondition_blocked)
  / commands_discovered

S_discovery =
  70.0 * command_recognition_rate
+ 30.0 * avg_command_confidence
```

Precondition-blocked commands count as recognized because a target that reaches auth, fixture, local-context, network, or runtime-dependency validation has usually recognized the command path.

### Grammar

Grammar returns `0` if no commands are runtime-confirmed. Otherwise:

```text
flag_presence =
  1.0 if flags_discovered > 0
  0.0 otherwise

grammar_gap_rate =
  grammar_gap_count / (commands_runtime_confirmed * 2)

flag_grammar_rate =
  flags_with_known_grammar / flags_discovered

S_grammar =
  30.0 * flag_presence
+ 25.0 * avg_flag_confidence
+ 20.0 * flag_grammar_rate
+ 25.0 * (1.0 - grammar_gap_rate)
```

The current grammar-gap heuristic gives each runtime-confirmed command up to two gap slots: invalid-flag behavior and child/usage clarity.

### Execution

Execution measures whether completed probes avoided timeout and spawn failure:

```text
bad = probes_timed_out + probes_failed_to_spawn

S_execution = 100.0 * (1.0 - bad / probes_completed)
```

If no probes completed, execution returns `0`.

Target CLI nonzero exits are not automatically CLIARE failures. They are evidence. For example, a clean unknown-flag rejection can improve recovery.

### Recovery

Recovery combines invalid-input rejection and actionable precondition diagnostics:

```text
invalid_recovery =
  invalid_probe_rejections / invalid_probe_count

precondition_recovery =
  actionable_precondition_probes / precondition_blocked_probes
```

If both rates exist:

```text
S_recovery = 100.0 * (0.7 * invalid_recovery + 0.3 * precondition_recovery)
```

If only one exists, CLIARE uses that one. If neither exists, recovery returns `0`.

### Output

Output only treats JSON and YAML contracts as machine-readable in v0.

If no machine-readable output contract is discovered:

```text
S_output = 0
```

Otherwise:

```text
non_blocked_probe_count =
  output_mode_probe_count
- output_mode_precondition_blocked
- output_mode_help_text_probes
- output_mode_global_scope_failures

denominator =
  max(output_mode_scored_contracts, non_blocked_probe_count)

S_output =
  40.0
+ 60.0 * (output_mode_parse_successes / denominator)
```

The `40.0` component rewards a discovered advertised machine-readable contract. The `60.0` component rewards successful parsing for non-blocked output-mode probes.

### Safety

Safety measures persistent filesystem side effects from safe probes in the observed sandbox regions:

```text
changed_probe_penalty =
  45.0 * (side_effect_probe_count / probes_completed)

file_penalty =
  min(side_effect_files_total * 8.0, 35.0)

credential_penalty =
  min(credential_like_side_effects * 20.0, 40.0)

S_safety =
  max(100.0 - changed_probe_penalty - file_penalty - credential_penalty, 0.0)
```

If no probes completed, safety returns `0`.

Host execution mode does not provide the same isolated filesystem-diff coverage as isolated mode. A scorecard should be interpreted with its `execution_mode`, `sandbox_profile`, and `sandbox_env_policy` context.

---

## Current v0 Coverage Metrics

The coverage section records the main counters used to interpret the score:

- sandbox root, home, workdir, profile, and env policy
- commands discovered, runtime-confirmed, and precondition-blocked
- command confirmation rate
- help-text extraction counts and parser extraction rate
- flags discovered
- output contracts discovered, machine-readable contracts, output probe counts, parse successes, and precondition blocks
- filesystem side-effect counts and credential-like path counts
- average command and flag confidence
- observed max depth
- traversal profile, max depth, max probes, minimum expected value, concurrency, rounds, scheduled/completed/cancelled probes, frontier state, budget exhaustion, and stop reason
- timeout and spawn-failure counts
- precondition counts for auth, local context, fixture, network, and runtime dependency cases
- actionable precondition recovery rate

The traversal stop reason is one of:

- `frontier_exhausted`
- `converged`
- `depth_budget_exhausted`
- `probe_budget_exhausted`

---

## Current v0 Findings

Findings are generated from coverage metrics and model thresholds. Current finding ids include:

| Finding | Meaning |
|---|---|
| `finding.discovery.low_runtime_confirmation` | Most discovered command candidates were not runtime-confirmed. |
| `finding.discovery.extraction_limited` | Help text was observed but did not yield reliable structural shape. |
| `finding.grammar.unconfirmed_arity` | Runtime-confirmed commands still have unknown grammar details. |
| `finding.execution.timeouts` | Some probes timed out. |
| `finding.recovery.invalid_probe_acceptance` | Invalid probes did not consistently reject with nonzero exit status. |
| `finding.output.no_machine_readable_mode` | No JSON or YAML output mode was discovered. |
| `finding.output.unparseable_mode` | Advertised output modes failed parse checks. |
| `finding.precondition.runtime_blocked` | Some probes were blocked by runtime preconditions. |
| `finding.safety.safe_probe_side_effects` | Safe probes left persistent filesystem side effects. |
| `finding.safety.credential_like_side_effects` | Side-effect paths contained credential-like terms. |

These findings are the bridge between numeric scoring and maintainer action. They also feed the issue ledger and persona reports.

---

## Running Example

Assume a fictional CLI named `acmectl`:

```sh
acmectl project list --format json
```

Suppose one CLIARE run observes:

- `acmectl --help` exits 0 and lists `project`.
- `acmectl project --help` exits 0 and lists `list`.
- `acmectl project list --help` exits 0 and shows `Usage: acmectl project list [--format <json|table>] [--org <ORG>]`.
- `acmectl project list --format json` exits with an auth-required diagnostic in an empty sandbox.
- The same command in an authenticated context exits 0 and emits a JSON array.
- `acmectl project list --__cliare_unknown_flag__` exits nonzero with an unknown-flag diagnostic.
- One help probe creates `home/.acme/cache/help.json` inside the sandbox.

The evidence should be interpreted this way:

| Dimension | Effect |
|---|---|
| Discovery | `project list` receives layout, usage, and runtime-help evidence. The auth-required diagnostic can still support command recognition. |
| Grammar | `--format` and its value hint improve flag grammar. Unknown positional or child behavior can still leave gaps. |
| Execution | Timeouts and spawn failures hurt execution. Ordinary nonzero target exits are classified as evidence. |
| Recovery | Unknown-flag rejection and actionable auth diagnostics improve recovery. |
| Output | JSON is credited only when discovered and parseable under a non-blocked probe. |
| Safety | The cache write lowers safety unless it is expected and dispositioned outside the raw score interpretation. |

The point is not that `acmectl` is good or bad. The point is that CLIARE separates command existence, preconditions, output contracts, side effects, and diagnostic quality instead of collapsing every nonzero exit into failure.

---

## Current Limitations

The current v0 model is deliberately explicit about what it does not yet do:

- It does not compute full Beta-Bernoulli or Dirichlet posteriors.
- It does not expose posterior confidence intervals on the score.
- It does not infer a complete workload distribution.
- It does not apply user-supplied command importance weights to the total score.
- It does not compute capture-recapture estimates for hidden command surface size.
- It does not maintain public categorical posteriors for side-effect classes.
- It does not provide authoritative public leaderboard scoring.
- It does not provide a public rescore command for old artifacts under a new model.
- It does not treat host-mode filesystem behavior as equivalent to isolated sandbox side-effect coverage.

Those are future model and product directions, not current shipped guarantees.

---

## Future Calibrated Model Direction

The future certified model should make the summary equation operational:

```text
Score = 100 * E[U(G, T) | E]
```

Where:

```text
U(G, T) = sum_c I_T(c) * U(c)
```

And command utility can be modeled as:

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

- `D(c)` is discoverability.
- `G(c)` is grammar quality.
- `X(c)` is execution reliability.
- `O(c)` is output-contract quality.
- `Safe(c)` is safety readiness.
- `R(c)` is recovery quality.
- `I_T(c)` is command importance under task distribution `T`.

The multiplicative form is useful because a command that is totally unsafe or undiscoverable should not receive a high utility merely because other dimensions are strong.

### Future Binary Claims

A future calibrated model can replace hand-authored log-odds weights with benchmark-derived pseudo-counts:

```text
theta_z ~ Beta(alpha_0, beta_0)
observations y_i ~ Bernoulli(theta_z)

alpha = alpha_0 + sum_i w_i * positive_i
beta  = beta_0  + sum_i w_i * negative_i

P(z | E) = alpha / (alpha + beta)
```

This is not the current implementation, but it is compatible with the current evidence trail.

### Future Categorical Claims

For output kind, side-effect class, or precondition class, a future model can use a Dirichlet-Categorical form:

```text
theta ~ Dirichlet(alpha_0)
P(K = k | E) = alpha_k / sum_j alpha_j
```

Candidate categorical states include output kinds, precondition kinds, side-effect classes, and recovery quality. Current v0 records many of those states directly, but does not maintain full categorical posterior distributions.

### Future Discovery Coverage

Discovery is hard because the true command count is unknown. Future models can use multiple discovery channels and capture-recapture estimates:

```text
N_hat = |A| * |B| / |A intersect B|
```

Where `A` and `B` might be help traversal, shell completions, error suggestions, docs, or machine-readable catalogs. If overlap is small, the model should report wide uncertainty instead of false precision.

### Future Command Importance

Not every command should weigh equally. A future agent-readiness score can use command importance:

```text
sum_c I(c) = 1
```

Default importance might consider root/help prominence, read-oriented verbs, examples, successful execution, command category, and workload hints. CLIARE v0 already emits `command-index.json`, which lets a harness or benchmark layer command-level workload weights on top of current artifacts.

---

## Calibration Discipline

The bundled v0 model already declares calibration requirements:

- train
- validation
- holdout

The split should be by CLI family, not by individual probe rows. A CLI family used to tune output probing should not also be treated as holdout evidence for the same model.

Future calibration should optimize for trustworthy measurement, not just high scores:

```text
loss =
  alpha * command_existence_log_loss
+ beta  * flag_arity_log_loss
+ gamma * output_contract_log_loss
+ delta * precondition_classification_loss
+ eta   * false_safe_penalty
+ zeta  * extraction_ambiguity_penalty
```

Proper scoring rules should include:

```text
Brier = mean_i (p_i - y_i)^2
```

```text
LogLoss = -mean_i [ y_i log(p_i) + (1 - y_i) log(1 - p_i) ]
```

```text
ECE = sum_bins |acc(bin) - conf(bin)| * n_bin / n
```

A certified model should not be frozen until it has:

- human-reviewed truth labels for synthetic and real CLIs
- reported Brier score, log loss, expected calibration error, false-safe rate, false-confirmed-command rate, and depth-weighted recall
- repeated-run stability across clean environments
- fixed traversal profiles, probe budgets, timeouts, execution-mode rules, and fixture rules
- a published immutable score-model artifact and hash

Until then, v0 scores are suitable for CI improvement loops, maintainer feedback, and benchmark development, not authoritative public ranking.

---

## Monotonic Improvement

The model should reward real improvements:

- adding parseable JSON or YAML output should improve output if no new negative evidence appears
- improving flag arity or value-domain clarity should improve grammar
- making invalid inputs fail cleanly should improve recovery
- removing safe-probe side effects should improve safety
- making command help easier to parse should improve discovery and grammar

However, deeper evidence can reveal hidden risk. If a later run discovers that safe probes write credential-like files, safety can decrease even if the CLI binary did not change. That is not a monotonicity violation; it is uncertainty reduction.

Reports should distinguish:

- actual regression
- newly discovered surface
- measurement-limited evidence
- newly revealed risk
- explicit maintainer disposition

Current v0 supports this distinction with separate dimensions, coverage counters, findings, dispositions, and persona reports.

---

## Handling Unknowns

Unknown is not the same as broken.

Good interpretation:

```text
The shape of `deploy ENV` is unknown because no safe fixture invocation was available.
```

Bad interpretation:

```text
deploy is bad.
```

Unknowns should reduce confidence or readiness only to the extent they matter for agent routing and maintainer action. Current v0 encodes this through command runtime states, output contract statuses, precondition diagnostics, gap kinds, issue dispositions, and `agent_suitability` in the command index.

---

## Gaming Resistance

Any public score can be gamed. The model should make obvious gaming expensive:

- runtime validation should outweigh help-only claims
- advertised JSON or YAML should parse
- safe probes should be checked for persistent filesystem side effects
- unknown commands and flags should reject cleanly
- hidden or plugin surfaces should be represented as uncertainty rather than ignored
- model status and score provenance should be visible

Example:

If help advertises `--format json` but the runtime probe emits a table, the help text can create an advertised output contract, but the parse probe prevents CLIARE from treating that JSON contract as confirmed.

---

## Model Versioning

A score model must be immutable once published. Revisions should create a new model id or versioned model artifact. Old scorecards preserve:

- score model id
- score model hash
- score status
- normalization rule
- target fingerprint
- runtime context
- evidence-derived coverage

That allows later tooling to compare runs honestly:

- same target, same evidence, same model
- same target, different evidence, same model
- same target, same evidence, different model
- different target binary, different evidence, same model

CLIARE currently embeds the bundled model hash in `scorecard.json`. Future tooling can add explicit rescoring workflows, but this document should not claim a command surface that does not exist.

---

## Operational Reading

For maintainers, the score is not the product by itself. The actionable loop is:

1. Run `cliare measure`.
2. Review `scorecard.json`, `report.md`, `issues.json`, `shape.json`, and `command-index.json`.
3. Use `cliare issues list` to inspect unresolved findings.
4. Fix command shape, help consistency, output contracts, recovery diagnostics, and safe-probe side effects where appropriate.
5. Disposition findings that are expected product behavior.
6. Measure again in the same context and compare.
7. Add the measurement to CI once the baseline is understood.

For harnesses, the most important artifact is usually `command-index.json`: it is the map that helps an agent route through a CLI deliberately instead of rediscovering command syntax by trial and error.
