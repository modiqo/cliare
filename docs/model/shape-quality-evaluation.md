# Shape Quality Evaluation

> **Scope:** Current fixture truth-set evaluator for `shape.json`.
> **Status:** Current implementation.

---

## Purpose

`cliare eval shape-quality` compares an inferred CLIARE shape artifact against a
human-authored fixture truth set. It is the first concrete evaluation artifact
for the agent-shape product use case:

- maintainers can see which inferred shape claims are missing or unexpected;
- benchmark authors can track fixture accuracy independently from the headline
  readiness score;
- future calibration work can compare confidence and score movement against
  truth labels.

The evaluator is intentionally exact-match oriented in v1. It does not try to
decide that two different command names, aliases, flags, or output contracts are
semantically equivalent. Fixture truth files should spell out the expected
surface explicitly.

---

## Command

```sh
cliare eval shape-quality \
  --shape .cliare/<target>/shape.json \
  --truth benchmarks/truth/<target>.shape-truth.json \
  --out .cliare-eval/<target>
```

The command writes:

```text
shape-quality.json
shape-quality.md
```

The command does not fail on low quality. It produces evaluation artifacts that
benchmark and calibration workflows can consume.

---

## Truth Schema

Truth files use:

```json
{
  "schema_version": "cliare.shape-truth.v1",
  "target_id": "fixture-cli",
  "commands": [
    {
      "path": ["project", "list"],
      "aliases": ["ls"],
      "positionals": [],
      "flags": [
        {
          "name": "--format",
          "short": "-f",
          "value_kind": "required",
          "value_name": "FORMAT",
          "required": false,
          "repeatable": false
        }
      ],
      "output_contracts": [
        {
          "mode": "json",
          "flag_name": "--format",
          "argv_fragment": ["--format", "json"],
          "parseable": true
        }
      ],
      "preconditions": []
    }
  ]
}
```

The root command is represented with an empty path:

```json
{ "path": [] }
```

Current precondition values should use the same labels as `shape.json`, such as
`auth_required`, `local_context_required`, `fixture_required`,
`network_unavailable`, and `runtime_dependency_unavailable`.

---

## Metrics

`shape-quality.json` reports exact-match precision, recall, and F1 for:

| Metric | Match key |
|---|---|
| `commands` | command path |
| `aliases` | command path plus alias |
| `flags` | command path plus long flag name |
| `flag_grammar` | command path, flag name, short flag, value kind, value name, requiredness, and repeatability |
| `positionals` | command path, positional index, name, requiredness, and variadic marker |
| `output_contracts` | command path, mode, flag name, argv fragment, and parseability |
| `preconditions` | command path plus precondition kind |

Each metric includes:

```json
{
  "expected": 3,
  "observed": 2,
  "matched": 2,
  "precision": 1.0,
  "recall": 0.6667,
  "f1": 0.8,
  "missing": [],
  "unexpected": []
}
```

`overall.score` is the mean F1 across non-empty metric families, scaled to
`0..100`.

---

## Provenance Checks

The report also records provenance completeness that does not require a truth
file:

| Field | Meaning |
|---|---|
| `target_present` | Whether the shape includes target fingerprint metadata. |
| `model_present` | Whether the shape includes inference model metadata. |
| `command_evidence` | Fraction of command entries with at least one evidence reference. |
| `flag_evidence` | Fraction of flag entries with at least one evidence reference. |
| `output_contract_evidence` | Fraction of output-contract entries with at least one evidence reference. |

This keeps shape accuracy and shape explainability visible as separate signals.

---

## Current Limitations

The current evaluator is a fixture truth-set evaluator, not a full harness A/B
runner. It does not yet:

- evaluate agent task success;
- compare shape-assisted and raw-terminal harness conditions;
- score safety side-effect truth labels beyond precondition and artifact
  provenance checks;
- calibrate claim confidence probabilities against truth labels.

Those remain part of the broader evaluation roadmap.
