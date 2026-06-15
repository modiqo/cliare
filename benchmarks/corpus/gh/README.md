# gh Benchmark Notes

## Target

| Field | Value |
|---|---|
| CLI | `gh` |
| Category | Source control / GitHub |
| Installed path | `/opt/homebrew/Cellar/gh/2.65.0/bin/gh` |
| Version | `gh version 2.65.0 (2025-01-06)` |
| Binary SHA256 | `35cb52f49b0cd2543880375456d28a5918f2ebd1341e177cef3f59e3174d8795` |
| Local artifact folder | `~/.cliare/corpus-runs/20260614T170817Z-gh` |

## Measurement

```sh
cliare measure gh \
  --out ~/.cliare/corpus-runs/20260614T170817Z-gh \
  --profile deep \
  --max-depth 12 \
  --max-probes 5000 \
  --refresh

cliare describe ~/.cliare/corpus-runs/20260614T170817Z-gh --write
```

## Result

| Field | Value |
|---|---:|
| Score | `94/100` |
| Score model | `cliare-score-v0` |
| Status | `experimental_partial` |
| Commands indexed | `185` |
| Runtime-confirmed commands | `184` |
| Precondition-blocked commands | `0` |
| Observed depth | `3 / 12` |
| Probes completed | `792 / 5000` |
| Frontier remaining | `0` |
| Traversal complete | `true` |
| Stop reason | `converged` |
| Side-effect file changes | `0` |

## Command Suitability

| Suitability | Count |
|---|---:|
| `ready` | `140` |
| `conditional` | `36` |
| `needs_fixture` | `8` |
| `blocked` | `0` |
| `candidate` | `1` |

## Findings

| Severity | Issue | Count | Notes |
|---|---|---:|---|
| High | `issue.output_mode_parse_failed` | `34` commands | `gh` advertises many JSON modes, but safe output-mode probes did not produce parseable machine output in this run. |
| Medium | `issue.output_mode_unprobed` | `8` commands | Some advertised output modes need fixture operands or command-local validation. |
| Medium | `issue.help_unavailable` | `1` command | One command did not expose usable help under the safe probe path. |
| Low | `issue.flags_unknown` | `1` command | One command has incomplete flag grammar. |

## Assessment

`gh` is a strong benchmark target for CLIARE: broad real-world surface, many nested command families, clear help, and substantial advertised JSON support. The run completed without exhausting the probe budget and without side effects or auth precondition blocks.

The main follow-up is output validation. CLIARE discovered `120` machine-readable output contracts and completed `102` output-mode probes, but recorded `0` parse successes. This should be investigated before using `gh` as a calibrated positive example for output readiness. It may require fixture operands, command-local examples, or better safe data-producing probes.
