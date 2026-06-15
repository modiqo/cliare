# CLIARE

**CLIARE audits command-line interfaces for agent readiness.**

Agents increasingly use terminals as their operating surface, but most CLIs were designed for humans reading help text. An agent harness needs a different contract before it spends tokens trying commands:

- Which commands actually exist?
- Which flags and positionals are safe to use?
- Which commands have parseable JSON/YAML output?
- Which paths require auth, a project directory, fixtures, network access, or a local daemon?
- Which "safe" discovery commands quietly write files?

CLIARE answers those questions by measuring the released CLI binary as a black box. It probes runtime behavior under bounded controls, records evidence, infers the command surface, detects side effects, and emits command indexes, issue ledgers, scorecards, persona reports, CI artifacts, and agent skills.

CLIARE stands for **CLI Agent Readiness Evaluation**.

## Why Run CLIARE?

### For CLI Maintainers

CLIARE turns vague "is this CLI agent-friendly?" feedback into a concrete implementation queue.

It shows where agents will struggle with command discovery, help coverage, diagnostics, output contracts, preconditions, and unsafe discovery behavior. Instead of guessing what to improve, maintainers get evidence-backed issues, affected commands, recommendations, and verification commands.

### For Security Reviewers

CLIARE catches undocumented runtime behavior in binaries.

It snapshots filesystem state around each probe and reports persistent side effects from safe-looking commands such as help, version, invalid flag, and output-mode probes. This helps security teams find cache/config writes, credential-like paths, and host/auth behavior before a CLI is exposed to autonomous agents.

### For Agent Harness Builders

CLIARE creates a command index that agents can read before exploring.

Harnesses can load `command-index.json` to route through known commands, prefer parseable output contracts, honor preconditions, and avoid rediscovering syntax by trial and error. The result is lower token cost, fewer bad invocations, and more deliberate CLI use.

## What You Get

A measurement writes one artifact directory:

```text
.cliare/<target-cli>/
  command-index.json      # agent-facing command catalog
  command-index.md        # human-readable command catalog
  AGENT_SKILL.md          # generated guidance agents can read
  scorecard.json          # readiness score, subscores, coverage, findings
  issues.json             # reviewable issue ledger
  issues.md               # human issue report
  issue-dispositions.json # reviewed decisions, when present
  persona-maintainer.md   # maintainer action packet
  persona-harness.md      # agent harness packet
  persona-security.md     # side-effect and approval packet
  evidence.jsonl          # raw runtime evidence
  shape.json              # inferred command shape
  findings.sarif          # CI/security upload format
  junit.xml               # CI test result format
```

The key artifact is `command-index.json`. It records command paths, argv forms, summaries, confidence, runtime state, agent suitability, flags, positionals, preconditions, output contracts, gaps, and evidence references.

## Install

CLIARE is prepared for crates.io:

```sh
cargo install cliare
```

That command becomes the stable install path after the first crates.io publish. Until then, install from source:

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

Release preparation lives in [RELEASE.md](RELEASE.md). The version history is [CHANGELOG.md](CHANGELOG.md).

## Quick Start: Maintainer Audit

Use this when you maintain a CLI and want a fix list.

```sh
cliare measure mycli --out .cliare/mycli --profile standard --refresh
cliare issues list --out .cliare/mycli --format markdown
cliare report maintainer --out .cliare/mycli --format markdown
```

For launch or release-quality review:

```sh
cliare measure mycli \
  --out .cliare/mycli \
  --profile deep \
  --max-depth 12 \
  --max-probes 5000 \
  --concurrency 8 \
  --refresh
```

If a finding is real, fix the CLI and rerun. If the behavior is intentional, record a disposition so CI keeps the evidence but stops treating it as unreviewed noise:

```sh
cliare issues mark <issue-id> \
  --out .cliare/mycli \
  --status intentional \
  --reason "Direct <command> --help is the canonical help contract."
```

Print the full maintainer walkthrough:

```sh
cliare playbook maintainer --target mycli
```

## Quick Start: Security Side-Effect Review

Use this when you need to know what a CLI does during safe discovery.

```sh
cliare measure mycli --out .cliare/mycli --profile standard --refresh
cliare report security --out .cliare/mycli --format markdown
cliare issues list --out .cliare/mycli --format markdown
```

CLIARE's default execution mode is isolated. It clears most inherited environment variables, uses sandboxed runtime directories, bounds output, bounds time, and records filesystem changes observed around each probe.

For authenticated or host-specific review, run an explicit context so the result is not confused with a clean run:

```sh
cliare measure mycli \
  --out .cliare/mycli \
  --context authenticated \
  --auth-state present \
  --execution-mode host \
  --profile deep \
  --refresh

cliare report security --out .cliare/mycli --context authenticated --format markdown
```

Print the full security walkthrough:

```sh
cliare playbook security --target mycli
```

## Quick Start: Agent Harness Command Index

Use this when you want agents to understand a CLI before they invoke it.

```sh
cliare measure mycli --out .cliare/mycli --profile deep --refresh
cliare describe .cliare/mycli --write
cliare report harness --out .cliare/mycli --write
```

Then point your harness at:

```text
.cliare/mycli/command-index.json
.cliare/mycli/AGENT_SKILL.md
.cliare/mycli/persona-harness.json
```

Harness routing should:

1. Load `command-index.json`.
2. Prefer commands with `agent_suitability` of `ready`.
3. Treat `conditional` commands as requiring their listed preconditions.
4. Prefer parseable output contracts when state needs to be read.
5. Avoid `blocked`, `needs_fixture`, and low-confidence paths unless the harness can satisfy the missing context.
6. Use evidence references only when a route needs audit or debugging.

Print the full harness walkthrough:

```sh
cliare playbook harness --target mycli
```

## Understanding Profiles

Most users should pick a profile first and only tune advanced knobs when the report says traversal pressure exists.

| Profile | Use When |
|---|---|
| `quick` | Fast local smoke pass while editing help, diagnostics, or one command family. |
| `standard` | Normal maintainer/security/harness review loop. |
| `deep` | Release-quality pass, CI baselines, large command surfaces, and agent-surface publishing. |

Advanced knobs:

| Option | Meaning |
|---|---|
| `--max-depth` | Maximum recursive command-path depth. Increase when nested commands are missing. |
| `--max-probes` | Runtime probe budget. Increase when `budget_exhausted` or `frontier_remaining` appears. |
| `--concurrency` | Probes run at the same time. Lower for flaky CLIs, shared state, or rate limits. |
| `--timeout-ms` | Per-probe timeout. Raise for slow, network-backed, or daemon-backed CLIs. |
| `--execution-mode host` | Use host auth/config/plugins/local state. Only use when that context is intentional. |

For long runs:

```sh
cliare measure mycli --out .cliare/mycli --profile deep --refresh --detach
cliare jobs status --out .cliare/mycli
```

## Context-Specific Measurements

Many CLIs change behavior based on auth, project directory, local config, installed plugins, fixtures, network, or daemons. Keep those runs separate:

```sh
cliare measure mycli --out .cliare/mycli --context clean --profile standard --refresh

cliare measure mycli \
  --out .cliare/mycli \
  --context local-context \
  --context-workdir /path/to/project \
  --profile deep \
  --refresh
```

Context artifacts live under:

```text
.cliare/mycli/contexts/<context>/
```

Compare contexts:

```sh
cliare context compare .cliare/mycli/contexts/clean .cliare/mycli/contexts/local-context --write
```

## CI Usage

CLIARE ships a composite GitHub Action in this repository.

```yaml
name: CLIARE

on:
  pull_request:
  push:
    branches: [main]

jobs:
  cliare:
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

The action appends a summary to the job summary, uploads the artifact directory, and exposes output paths for `scorecard.json`, `summary.md`, `findings.sarif`, and `junit.xml`.

For release gates, keep a baseline and use `guard`:

```sh
cliare guard mycli \
  --baseline .cliare-baseline/mycli/scorecard.json \
  --policy cliare.policy.json \
  --out .cliare/mycli \
  --profile deep
```

## Issue Dispositions

CLIARE keeps evidence visible even when a finding is reviewed. That means `issues_total` can remain nonzero while `action_required` is zero.

Useful statuses:

| Status | Use When |
|---|---|
| `intentional` | The behavior is deliberate and documented. |
| `needs-fixture` | Safe operands or fixture data are needed before judging the finding. |
| `not-applicable` | The finding does not apply to this measured profile. |
| `accepted-risk` | Security/platform owners reviewed and accepted the residual risk. |
| `false-positive` | The evidence is misleading for this CLI. |
| `deferred` | The issue is real but scheduled for later. |

Review queue:

```sh
cliare issues list --out .cliare/mycli --format markdown
cliare issues list --out .cliare/mycli --format json
```

Mark a reviewed decision:

```sh
cliare issues mark <issue-id> \
  --out .cliare/mycli \
  --status not-applicable \
  --reason "This command requires a project fixture and is not part of clean CI."
```

## Agent Skills

CLIARE can install local artifact-review skills so coding agents know how to inspect CLIARE outputs:

```sh
cliare skills list
cliare skills install --agent all --scope project
```

Use `--agent claude`, `--agent codex`, or `--agent cursor` for one integration. Use `--scope user` for user-level install.

## Command Index Registry Workflow

This repository includes a manual workflow for building public command-index entries:

```text
Actions -> Extract Command Index PR
```

Inputs include `artifact_id`, `target`, optional `install_command`, `profile`, `max_depth`, and `max_probes`.

The workflow builds CLIARE, optionally installs the target CLI, measures it, copies public review artifacts into `registry/<artifact_id>/`, and opens or updates a pull request with the measured command index, score, and issue ledger.

## Benchmark Corpuses

The repo includes launch and calibration manifests:

```sh
cliare benchmark --manifest benchmarks/local-corpus.json --out .cliare-bench --refresh
cliare benchmark --manifest benchmarks/vendor-calibration-corpus.json --out .cliare-vendor-calibration --refresh
cliare benchmark --manifest benchmarks/launch-low-pretraining-corpus.json --out .cliare-launch-low-pretraining --refresh
cliare benchmark --manifest benchmarks/agent-harness-corpus.json --out .cliare-agent-harness --refresh
```

The low-pretraining launch corpus focuses on newer and faster-moving CLIs where generated command indexes are most likely to help agents.

## Design Packet

The design and implementation notes live under [`docs/`](docs/00-index.md). Start with:

- [Design index](docs/00-index.md)
- [Runtime evidence for agent-ready CLIs](docs/19-runtime-evidence-for-agent-ready-clis.md)
- [Persona outcome packets](docs/20-persona-outcome-packets.md)
- [Agent-ready CLI standard template](docs/22-agent-ready-cli-standard-template.md)
- [Agent skills installation](docs/23-agent-skills-installation.md)
- [CLI benchmark corpus tracker](docs/24-cli-benchmark-corpus-tracker.md)
- [Maintainer playbook](docs/27-maintainer-playbook.md)

## License

Apache-2.0. See [LICENSE](LICENSE).
