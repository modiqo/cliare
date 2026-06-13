# 17 - Scoring Model and Bayesian Confidence

> **Scope:** What CLIARE's current score means, what is Bayesian today, why the model is credible, and what remains before the public standard score is frozen.
> **Status:** Reference implementation note

---

## Short Answer

Yes, CLIARE has a credible scoring model for the current implementation stage.

It is credible because it is:

- evidence backed: every score comes from runtime observations
- replayable: evidence, shape, and scorecard artifacts are deterministic outputs
- probabilistic at the claim layer: command and flag confidence use Bayesian log-odds updates
- decomposed: the total score is a weighted utility over named dimensions
- improvement oriented: obvious CLI improvements move the relevant subscore up
- honest about maturity: the model is published as `cliare-score-v0` with status `experimental_partial`

It is not yet a frozen public leaderboard model.

Before CLIARE should claim a certified universal score, it still needs calibrated likelihood weights, human-verified truth sets, confidence intervals, repeated-run stability reports, and published calibration metrics such as Brier score, log loss, expected calibration error, and false-safe rate.

That distinction matters. The current score is already useful for local CI, regression tracking, and iterative CLI quality improvement. The future v1 score is the standard-worthy model for public certification and leaderboard ranking.

---

## Design Principle

CLIARE should never be a black-box opinion about whether a CLI is "good."

The score is designed as:

```text
CLIARE Score = posterior expected utility of an agent using a CLI,
               given observed runtime evidence
```

In formal shorthand:

```text
Score = 100 * E[U(G, T) | E]
```

Where:

- `G` is the latent true command surface.
- `T` is the task or workload distribution.
- `E` is the evidence collected from probes.
- `U` is an agent-readiness utility function.

The implementation does not yet estimate the full posterior over `G` and `T`. Instead, v0 implements an evidence-only approximation over directly measured dimensions and keeps model status explicit. That gives us a stable engineering loop now without pretending calibration is done.

---

## What Is Bayesian Today

The current Bayesian layer is the claim-confidence model.

CLIARE does not parse help text into truth. It turns runtime observations into candidate claims and updates belief as more evidence arrives.

Examples of claims:

```text
command_exists(["project", "list"])
flag_exists(["project", "list"], "--format")
flag_arity("--format") = required_value
runtime_state(["model"]) = precondition_blocked(auth_required)
output_mode(["list"], "--json") = json
```

For binary command and flag claims, the implementation stores belief as log odds:

```text
logit(P(z | E)) = logit(P(z)) + sum_i w_i
P(z | E)        = sigmoid(logit(P(z | E)))
```

That is equivalent to a Bayesian odds update where each evidence weight is a log likelihood ratio:

```text
posterior odds = prior odds * product_i exp(w_i)
```

Current priors:

```text
P(command_exists) = 0.08
P(flag_exists)    = 0.12
```

Current command evidence weights:

| Evidence | Weight | Effect |
|---|---:|---|
| Structural command row in help/layout | `+1.0` | Weak positive candidate evidence |
| Usage syntax observed | `+0.5` | Small positive grammar evidence |
| Runtime help is reachable and help-like | `+4.0` | Strong positive command evidence |
| Runtime says auth/login/profile is required | `+2.0` | Positive command evidence, but blocked by precondition |
| Runtime help is not help-like and not precondition-blocked | `-2.0` | Negative command evidence |
| Invalid child rejected cleanly | `+0.5` | Small positive parser-boundary evidence |
| Invalid flag rejected cleanly | `+0.5` | Small positive parser-boundary evidence |

Current flag evidence weights:

| Evidence | Weight | Effect |
|---|---:|---|
| Structural flag row in help/layout | `+1.0` | Positive flag evidence |

This is intentionally lightweight. It gives CLIARE explainable confidence values now while leaving room for calibrated Beta-Bernoulli and Dirichlet-Categorical posteriors in v1.

---

## Why This Is Not Regex Scoring

CLIARE's generic processor does not say:

```text
if section_title == "Commands" then parse every row as a command
```

That would be brittle and would overfit to a few frameworks.

The current inference pipeline uses evidence layers:

- layout morphology
- indentation and aligned rows
- compact invocation cells
- token shape
- usage syntax
- runtime confirmation
- invalid command and invalid flag behavior
- output-mode probes
- sandbox side-effect observations
- precondition classification

Help text is only one weak evidence source. Runtime behavior has higher weight. Auth-required output is not treated as command absence; it becomes `runtime_state: precondition_blocked` with `auth_required`.

That matters for real CLIs such as `rote`, where some command-specific help is intentionally auth-gated in a clean environment. CLIARE should represent that accurately instead of scoring the command as nonexistent or broken.

---

## Current Score v0 Formula

Score v0 computes six measured subscores:

- discovery
- grammar
- execution
- recovery
- output
- safety

The total score is a weighted mean over measured dimensions:

```text
total = sum_d score_d * weight_d / sum_d weight_d
```

Current weights:

| Dimension | Weight | Purpose |
|---|---:|---|
| Discovery | `0.35` | Can agents find and recognize the surface? |
| Grammar | `0.20` | Can agents construct valid invocations? |
| Execution | `0.20` | Do probes complete without hangs or spawn failures? |
| Recovery | `0.15` | Do invalid probes reject cleanly? |
| Output | `0.05` | Are machine-readable modes advertised and parseable? |
| Safety | `0.05` | Do safe probes avoid persistent side effects? |

The weights deliberately emphasize discovery, grammar, execution, and recovery in v0 because those are the first-order requirements for agent navigation. Output and safety are already measured, but their current implementations are early and should gain weight only after stronger probes, calibration, and policy semantics exist.

---

## Discovery

Discovery asks whether an agent can find and recognize commands.

Current formula:

```text
recognition_rate = (runtime_confirmed_commands + precondition_blocked_commands)
                   / discovered_commands

discovery = 70 * recognition_rate + 30 * avg_command_confidence
```

The inclusion of `precondition_blocked_commands` is intentional. If a command returns a high-precision auth-required diagnostic, CLIARE has evidence that the command exists, even if the current runtime cannot exercise it further.

Discovery improves when:

- command help becomes reachable
- command rows are structurally clear
- auth-gated paths emit precise precondition diagnostics
- command aliases and usage syntax are visible
- probe budget is sufficient to confirm a deeper surface

Discovery regresses when:

- help output becomes ambiguous
- command candidates cannot be confirmed
- auth failures look like missing commands
- the surface grows faster than the configured traversal profile can cover

---

## Grammar

Grammar asks whether an agent can construct valid invocations.

Current formula:

```text
grammar =
  30 * flag_presence
+ 25 * avg_flag_confidence
+ 20 * flag_grammar_rate
+ 25 * (1 - grammar_gap_rate)
```

Where:

- `flag_presence` is `1` when any flags are discovered, otherwise `0`.
- `avg_flag_confidence` is the mean Bayesian confidence of discovered flags.
- `flag_grammar_rate` is the share of flags with known boolean/value grammar.
- `grammar_gap_rate` tracks unresolved grammar details on runtime-confirmed commands.

Grammar improves when:

- flags are listed consistently
- value arity is clear
- required flags are marked
- repeatable flags are marked
- usage syntax exposes positionals
- invalid value errors list valid values

---

## Execution

Execution asks whether probes can run in CI without hanging or failing to spawn.

Current formula:

```text
execution = 100 * (1 - (timeouts + spawn_failures) / completed_probes)
```

Execution improves when:

- help and diagnostic paths are fast
- commands honor noninteractive CI environments
- process trees exit cleanly
- output stays within limits
- no probe hangs waiting for input

---

## Recovery

Recovery asks whether invalid commands and flags fail cleanly.

Current formula:

```text
recovery = 100 * invalid_probe_rejections / invalid_probe_count
```

Auth/precondition-blocked invalid probes are excluded from recovery accounting. They are not successful recovery and they are not parser failures. They are runtime preconditions that should be reported separately.

Recovery improves when:

- unknown commands exit nonzero
- unknown flags exit nonzero
- diagnostics are clear
- suggestions help the agent repair the invocation
- auth errors name the missing precondition

---

## Output

Output asks whether the CLI advertises and honors machine-readable output modes.

Current formula:

```text
if machine_readable_output_contracts == 0:
    output = 0
else:
    non_blocked_probe_count = output_mode_probe_count - output_mode_precondition_blocked
    denominator = max(machine_readable_output_contracts, non_blocked_probe_count)
    output = 40 + 60 * output_mode_parse_successes / denominator
```

The fixed 40-point base rewards advertised JSON/YAML contracts. The parse component rewards safe probes that actually produce parseable output. Precondition-blocked output probes are reported separately and do not count as parse failures.

Output improves when:

- `--json`, `--format json`, or equivalent modes are advertised
- machine modes parse cleanly
- progress and warnings stay out of machine stdout
- output contracts are stable across versions

---

## Safety

Safety asks whether safe discovery probes leave persistent side effects.

Current formula:

```text
changed_probe_penalty = 45 * side_effect_probe_count / completed_probes
file_penalty          = min(side_effect_files_total * 8, 35)
credential_penalty    = min(credential_like_side_effects * 20, 40)

safety = max(0, 100 - changed_probe_penalty - file_penalty - credential_penalty)
```

This is an initial safety model, not the final destructive-action model. It currently focuses on filesystem side effects observed during safe probes.

Safety improves when:

- help, version, and diagnostic probes are read-only
- unavoidable cache writes are contained and documented
- token, key, credential, and secret paths are never created during discovery
- mutating commands expose dry-run or plan modes

---

## Why The Model Is Credible For CI

The score is credible for CI because it satisfies the properties maintainers need:

1. It is deterministic for a fixed binary, profile, sandbox policy, and evidence set.
2. It is evidence-replayable because the score is derived from observations.
3. It decomposes into subscores and findings.
4. It detects score deltas across releases.
5. It distinguishes missing commands from auth/precondition-blocked commands.
6. It makes traversal pressure visible instead of hiding incomplete exploration.
7. It can run in the maintainer's CI environment without sending binaries to a hosted runner.

This is enough to support:

- PR regression gates
- release-to-release drift checks
- score improvement tracking
- local benchmark corpus runs
- private enterprise scorecards

---

## Why The Model Is Not Yet Frozen For Public Ranking

The current score should not yet be marketed as a final universal ranking of all CLIs.

Missing v1 requirements:

- calibrated evidence weights
- public truth sets for synthetic and real CLIs
- posterior confidence intervals
- repeated-run stability measurements
- Brier score and log loss for binary/categorical claims
- expected calibration error for confidence values
- false-safe rate for safety classification
- profile normalization for quick, standard, deep, and certified runs
- score governance for model changes

The correct public posture is:

```text
v0: experimental partial score for local CI and improvement tracking
v1: calibrated public standard score for certification and leaderboard use
```

---

## Calibration Plan

The calibration path is straightforward:

1. Maintain a synthetic fixture corpus with known ground truth.
2. Maintain a real CLI corpus with human-reviewed truth sets.
3. Run `cliare benchmark` over the corpus on every model change.
4. Compare inferred claims to ground truth.
5. Compute Brier score, log loss, expected calibration error, and false-safe rate.
6. Tune evidence weights and priors.
7. Publish calibration reports by model version.
8. Freeze `cliare-score-v1` only after stability and calibration thresholds are met.

The current benchmark runner already provides the operational substrate: corpus manifests, per-target artifacts, expected score bands, runtime caps, target-level parallelism, and aggregate reports. The next layer is truth-set comparison and proper probabilistic calibration.

---

## Improvement Monotonicity

A score model for CLI quality must reward real improvements.

CLIARE should satisfy these local monotonicity expectations:

| Improvement | Expected score movement |
|---|---|
| Add reachable help for a command | Discovery up |
| Add clear usage syntax | Grammar up |
| Clarify flag arity | Grammar up |
| Add valid JSON output mode | Output up |
| Remove progress text from JSON stdout | Output up |
| Make help probes read-only | Safety up |
| Add dry-run to mutating commands | Safety up in future model |
| Reject unknown flags consistently | Recovery up |
| Replace generic auth failure with precise auth-required diagnostic | Discovery/precondition accuracy up |

Whole-surface scores can still move down when a CLI adds many poorly documented commands. That is not a violation; it means the measured surface grew and the new surface is less agent-ready. For release governance, CLIARE should preserve both:

- known-surface score for comparable previously discovered commands
- whole-surface score for the current CLI as shipped

---

## What To Tell Users

The honest description is:

> CLIARE's current score is an evidence-backed experimental readiness score. It uses Bayesian confidence updates for inferred command and flag claims, deterministic weighted subscores for directly measured dimensions, and explicit coverage/pressure reporting. It is strong enough for CI regression gates and iterative CLI quality improvement. Public certification and leaderboard ranking should wait for the calibrated v1 score model.

That is credible, defensible, and aligned with the implementation.
