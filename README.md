# CLIARE

CLIARE turns black-box command-line tools into evidence-backed command indexes for agents.

Agents increasingly operate through terminals, but most CLIs were designed for humans reading help text, not for tool routers that need to know which commands exist, which flags are safe, which outputs are parseable, and which paths require auth, project context, fixtures, or local daemons.

CLIARE measures the CLI that users actually install. It probes the executable under bounded runtime controls, records the evidence, and emits a command index that an agent harness can use before trying commands by trial and error.

CLIARE stands for **CLI Agent Readiness Evaluation**.

## What You Get

The primary artifact is the command index:

```text
.cliare/<cli>/
  command-index.json    # agent-facing command catalog
  command-index.md      # human-readable command catalog
  scorecard.json        # readiness score and subscores
  summary.md            # short run summary
  issues.md             # reviewable findings
  persona-harness.md    # agent-routing review packet
  persona-maintainer.md # CLI implementation review packet
  evidence.jsonl        # raw runtime evidence
```

`command-index.json` is the first artifact to integrate. It records command paths, argv forms, summaries, confidence, runtime state, agent suitability, flags, positionals, preconditions, output contracts, gaps, and evidence pointers.

Scoring, persona reports, SARIF, JUnit, benchmark manifests, and guard policies are already implemented, but they support the main workflow: build a reliable command index that lets agents route through CLIs deliberately.

## Install

From source:

```sh
git clone https://github.com/modiqo/cliare.git
cd cliare
cargo install --path .
cliare metadata --format json
```

For local development:

```sh
cargo build --locked --bin cliare
./target/debug/cliare metadata --format json
```

## First Command Index

Measure any CLI available on `PATH`:

```sh
cliare measure gh --out .cliare/gh --profile standard --refresh
cliare describe .cliare/gh --write
```

Open the index:

```sh
less .cliare/gh/command-index.md
jq '.commands[0]' .cliare/gh/command-index.json
```

Use a deeper run for large command surfaces:

```sh
cliare measure kubectl \
  --out .cliare/kubectl \
  --profile deep \
  --max-depth 12 \
  --max-probes 5000 \
  --refresh
```

Use context-specific runs when a CLI changes behavior based on login state, project directory, local services, or fixtures:

```sh
cliare measure mycli --out .cliare/mycli --context clean --profile standard --refresh
cliare measure mycli --out .cliare/mycli --context repo --context-workdir /path/to/project --profile deep --refresh
cliare describe .cliare/mycli --context repo --write
```

## Use The Index In An Agent Harness

Read `command-index.json` before executing the target CLI. Prefer commands marked `ready`, treat `conditional` commands as requiring their listed preconditions, and avoid `blocked`, `needs_fixture`, or low-confidence paths unless the harness can satisfy the missing context.

Typical routing flow:

1. Load `command-index.json`.
2. Find commands by path, summary, output contract, or parameter names.
3. Check `agent_suitability`, `runtime_state`, and `preconditions`.
4. Prefer commands with parseable output contracts when the task needs structured data.
5. Use `evidence_refs` to inspect raw proof only when a route is disputed.

For a human review packet focused on harness use:

```sh
cliare report harness --out .cliare/mycli --write
```

## CI Usage

CLIARE ships a composite GitHub Action in this repository. A minimal workflow for a project that wants a command index artifact:

```yaml
name: CLIARE

on:
  pull_request:
  push:
    branches: [main]

jobs:
  command-index:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install --git https://github.com/modiqo/cliare.git
      - uses: modiqo/cliare@main
        with:
          target: mycli
          out: .cliare/mycli
          profile: standard
          extra-args: --refresh
```

The action appends the CLIARE summary to the job summary, uploads the artifact directory, and exposes output paths for `scorecard.json`, `summary.md`, `findings.sarif`, and `junit.xml`.

For release gates, use `guard` with a baseline:

```sh
cliare guard mycli \
  --baseline .cliare/baseline.scorecard.json \
  --policy cliare.policy.json \
  --out .cliare/mycli
```

## Auto-PR Command Index Registry

This repository includes a manual workflow for building a public index entry from CI:

```text
Actions -> Extract Command Index PR
```

Inputs:

| Input | Purpose |
|---|---|
| `artifact_id` | Stable registry id, such as `gh`, `ruff`, or `supabase`. |
| `target` | Executable name or path to measure. |
| `install_command` | Optional setup command that installs the target CLI before measurement. |
| `profile` | `quick`, `standard`, or `deep`. |
| `max_depth` | Optional traversal depth override. |
| `max_probes` | Optional probe budget override. |

The workflow builds CLIARE, optionally installs the target CLI, measures it, copies the public review artifacts into `registry/<artifact_id>/`, and opens or updates a pull request with the measured score.

This is the launch path for the command-index registry: each extracted CLI gets a reviewable PR with the command index first, plus score and findings as supporting evidence.

## Benchmark Corpuses

The repo includes launch planning manifests:

```sh
cliare benchmark --manifest benchmarks/local-corpus.json --out .cliare-bench --refresh
cliare benchmark --manifest benchmarks/vendor-calibration-corpus.json --out .cliare-vendor-calibration --refresh
cliare benchmark --manifest benchmarks/launch-low-pretraining-corpus.json --out .cliare-launch-low-pretraining --refresh
cliare benchmark --manifest benchmarks/agent-harness-corpus.json --out .cliare-agent-harness --refresh
```

The low-pretraining launch corpus focuses on newer and faster-moving CLIs where a generated command index is most likely to help agents. The agent-harness corpus is kept separate so CLIARE's main product claim remains about ordinary operational CLIs.

## Reports And Scores

`cliare measure` writes persona reports automatically. Regenerate any report from an existing artifact directory:

```sh
cliare report maintainer --out .cliare/mycli --write
cliare report harness --out .cliare/mycli --write
cliare report security --out .cliare/mycli --write
cliare report platform --out .cliare/mycli --write
```

Scores are useful for local CI, release-to-release improvement tracking, and regression detection. Public rankings should wait for calibrated corpuses and frozen score models. The evidence and command index are the source of trust.

## Agent Skills

CLIARE can install local artifact-review skills so coding agents know how to inspect a CLIARE output directory:

```sh
cliare skills list
cliare skills install --agent all
```

Use `--agent claude`, `--agent codex`, or `--agent cursor` to install one integration. Use `--scope project --project-dir .` to attach the skill to a repository instead of a user profile.

## Design Packet

The full design and implementation notes live under [`docs/`](docs/00-index.md). Start with:

- [Design index](docs/00-index.md)
- [Runtime evidence for agent-ready CLIs](docs/19-runtime-evidence-for-agent-ready-clis.md)
- [Persona outcome packets](docs/20-persona-outcome-packets.md)
- [Agent-ready CLI standard template](docs/22-agent-ready-cli-standard-template.md)
- [Agent skills installation](docs/23-agent-skills-installation.md)
- [CLI benchmark corpus tracker](docs/24-cli-benchmark-corpus-tracker.md)
- [Calibration workflow TODO](docs/25-calibration-workflow-todo.md)

The previous long-form README is preserved at [README.backup.md](README.backup.md).

## License

Apache-2.0. See [LICENSE](LICENSE).
