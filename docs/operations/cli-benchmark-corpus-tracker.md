# 24 - CLI Benchmark Corpus Tracker

> **Scope:** Current benchmark manifests and candidate real-world CLI coverage.
> **Status:** Current tracker, not a calibrated leaderboard.

---

## Purpose

This document tracks the CLI corpora CLIARE uses to exercise real command surfaces. It is a planning and QA document for benchmark coverage. The checked-in source of truth is the JSON manifest set under `benchmarks/`.

The benchmark runner currently:

- reads a `cliare.benchmark-corpus.v1` manifest
- runs `cliare measure` once per target
- writes per-target measurement artifacts under `<out>/<target-id>/`
- writes corpus summaries to `<out>/benchmark.json` and `<out>/benchmark.md`
- treats missing optional targets as skipped
- fails required targets when the binary is missing or measurement fails
- compares scores to broad expected bands as a regression signal

The benchmark runner does not currently certify a public ranking, fit a score model, or validate human truth labels. Those belong to the future calibration workflow described in [Calibration Workflow TODO](calibration-workflow-todo.md) and the authority bar in [Calibration and Leaderboard Authority](calibration-and-leaderboard-authority.md).

---

## Why This Corpus Exists

Agents increasingly use CLIs as their hands for source control, cloud platforms, deployment, databases, package managers, tests, observability, security, and SaaS administration. CLIARE needs a corpus that reflects those real operational surfaces.

The corpus should not only measure old, highly memorized commands. It should deliberately include newer and faster-moving CLIs where model pretraining is less reliable and an evidence-backed command index is more valuable.

Useful coverage slices:

| Slice | Target Share | Purpose |
|---|---:|---|
| Legacy/high-pretraining anchors | 20% | Stable references such as `git`, `gh`, `docker`, `npm`, and `cargo` for regression and comparability. |
| Modern fast-moving developer/platform CLIs | 50% | Command surfaces that change quickly and are less likely to be fully internalized by pretrained models. |
| Auth/context-heavy SaaS CLIs | 20% | Preconditions, fixture needs, profile state, sensitive output, and agent-routing constraints. |
| Dynamic/plugin/contextual CLIs | 10% | Plugin dispatch, project-local commands, generated help, local daemons, and framework-specific behavior. |

Basic shell staples such as `grep`, `sed`, `awk`, `bash`, and coreutils are intentionally not the main pressure point for this corpus. They can be useful baselines, but CLIARE's stronger product claim is around operational CLIs whose surfaces change, require context, or carry side-effect risk.

---

## Current Manifests

| Manifest | Current Role | Default Profile | Target Concurrency | Required Targets | Notes |
|---|---|---|---:|---|---|
| `benchmarks/local-corpus.json` | Local dogfood and popular CLI QA corpus | `standard` | 3 | `cliare`, `rote`, `git`, `supabase` | Includes optional anchors such as `gh`, `cargo`, `npm`, `docker`, and `deno`. |
| `benchmarks/vendor-calibration-corpus.json` | Vendor/SaaS candidate corpus | `deep` | 2 | none | Name is historical; current expected bands are QA bands, not calibrated truth labels. |
| `benchmarks/launch-low-pretraining-corpus.json` | Launch candidate corpus biased toward newer or fast-moving CLIs | `standard` | 2 | none | All targets are optional so developers can run it on partial local installs. |
| `benchmarks/agent-harness-corpus.json` | Separate corpus for agent-harness CLIs | `standard` | 1 | none | Kept separate from ordinary operational CLIs because these tools have sensitive, self-referential behavior. |

Run the default local corpus:

```sh
cliare benchmark --manifest benchmarks/local-corpus.json --out .cliare-benchmark --refresh
```

Run the low-pretraining launch corpus:

```sh
cliare benchmark --manifest benchmarks/launch-low-pretraining-corpus.json --out .cliare-launch-low-pretraining --refresh
```

Run the vendor candidate corpus:

```sh
cliare benchmark --manifest benchmarks/vendor-calibration-corpus.json --out .cliare-vendor-candidates --refresh
```

Run the agent-harness corpus:

```sh
cliare benchmark --manifest benchmarks/agent-harness-corpus.json --out .cliare-agent-harness --refresh
```

Inspect the generated corpus summary:

```sh
cat .cliare-benchmark/benchmark.md
jq '.totals, .calibration, .targets[] | {id, status, score, expected_score, issues}' .cliare-benchmark/benchmark.json
```

---

## Manifest Schema

The current manifest schema is intentionally small:

```json
{
  "schema_version": "cliare.benchmark-corpus.v1",
  "name": "local-popular-deep-cli-corpus",
  "defaults": {
    "target_concurrency": 3,
    "profile": "standard",
    "max_depth": 6,
    "max_probes": 384,
    "min_expected_value": 75,
    "concurrency": 8,
    "timeout_ms": 5000,
    "output_limit_bytes": 262144
  },
  "targets": [
    {
      "id": "gh",
      "target": "gh",
      "required": false,
      "tags": ["popular", "developer-platform", "json-friendly"],
      "profile": "deep",
      "max_depth": 6,
      "max_probes": 512,
      "min_expected_value": 50,
      "expected_score": { "min": 75.0, "max": 100.0 },
      "max_duration_ms": 300000
    }
  ]
}
```

Supported target fields:

| Field | Meaning |
|---|---|
| `id` | Stable target identifier and output directory name after sanitization. |
| `target` | Executable path or command name. Relative paths resolve relative to the manifest file. |
| `required` | When `true`, a missing binary or failed measurement fails the corpus. When `false`, a missing binary is skipped. |
| `tags` | Descriptive labels for corpus slicing. Tags are not interpreted by the runner. |
| `profile` | Optional override for the traversal profile. Defaults to corpus `defaults.profile` or `quick`. |
| `max_depth`, `max_probes`, `min_expected_value`, `concurrency`, `timeout_ms`, `output_limit_bytes` | Per-target measurement controls passed into `cliare measure`. |
| `expected_score` | Broad score band used for benchmark pass/fail checks. It is not a truth label. |
| `max_duration_ms` | Optional runtime ceiling used by the benchmark report. |

---

## Local Corpus

`benchmarks/local-corpus.json` is the main dogfood corpus. It should remain small enough to run in developer machines and CI, while broad enough to catch regressions in traversal, scoring, issue generation, side-effect detection, and benchmark reporting.

| ID | Target | Required | Tags | Expected Band |
|---|---|---:|---|---:|
| `cliare` | `../target/debug/cliare` | yes | `self`, `rust`, `clap`, `policy`, `benchmark` | `85..100` |
| `rote` | `rote` | yes | `modiqo`, `agent-harness`, `deep-subcommands` | `50..100` |
| `git` | `git` | yes | `popular`, `deep-subcommands`, `sparse-machine-output` | `80..100` |
| `supabase` | `supabase` | yes | `popular`, `developer-platform`, `json-friendly` | `75..100` |
| `gh` | `gh` | no | `popular`, `developer-platform`, `json-friendly` | `75..100` |
| `cargo` | `cargo` | no | `popular`, `rust`, `deep-subcommands` | `55..100` |
| `npm` | `npm` | no | `popular`, `javascript`, `sparse-help` | `25..100` |
| `docker` | `docker` | no | `popular`, `developer-platform`, `daemon-backed` | `50..100` |
| `deno` | `deno` | no | `popular`, `javascript`, `json-friendly` | `50..100` |

Maintenance rule: do not record transient latest scores in this document. Scores belong in generated benchmark artifacts so they can be traced to a CLIARE version, score model, binary version, host, and run configuration.

---

## Vendor Candidate Corpus

`benchmarks/vendor-calibration-corpus.json` tracks SaaS and platform CLIs that matter to agent harnesses because they expose real business workflows. All entries are optional because many require installation, authentication, or fixture accounts.

| ID | Target | Tags | Expected Band | Follow-Up |
|---|---|---|---:|---|
| `gh` | `gh` | `vendor`, `github`, `source-control`, `holdout-candidate`, `json-friendly` | `70..100` | Useful anchor for source-control and API workflows. |
| `stripe` | `stripe` | `vendor`, `payments`, `webhooks`, `train-candidate` | `40..100` | Needs safe webhook and fixture strategy. |
| `supabase` | `supabase` | `vendor`, `database`, `local-stack`, `validation-candidate` | `50..100` | Needs clean, linked-project, and local-stack contexts. |
| `valyu` | `valyu` | `vendor`, `search`, `realtime-data`, `train-candidate` | `35..100` | Verify command name, auth model, and output contracts. |
| `posthog` | `posthog` | `vendor`, `analytics`, `self-hosting`, `train-candidate` | `35..100` | Verify official surface and package channel. |
| `elevenlabs` | `elevenlabs` | `vendor`, `ai-media`, `voice`, `validation-candidate` | `35..100` | Needs safe media fixtures and output-file expectations. |
| `ramp` | `ramp` | `vendor`, `finance`, `sensitive`, `holdout-candidate` | `35..100` | Verify official CLI availability and sensitive-output policy. |
| `google-workspace` | `google-workspace` | `vendor`, `google-workspace`, `productivity`, `holdout-candidate` | `35..100` | Resolve canonical CLI/package and fixture account strategy. |
| `agentmail` | `agentmail` | `vendor`, `email`, `webhooks`, `train-candidate` | `35..100` | Needs safe mailbox fixture and webhook/listener classification. |
| `vercel` | `vercel` | `vendor`, `deployment`, `frontend-platform`, `validation-candidate` | `50..100` | Needs project-context and auth-context runs. |

The `train-candidate`, `validation-candidate`, and `holdout-candidate` tags are planning tags only. They do not make the current benchmark output calibrated.

---

## Low-Pretraining Launch Corpus

`benchmarks/launch-low-pretraining-corpus.json` is the strongest launch-story corpus because it focuses on newer, fast-moving, repo-contextual, or auth-heavy CLIs where a command index can save agent exploration cost.

Current target IDs:

```text
mise, just, task, turbo, nx, ruff, pixi, dagster, airflow, convex, inngest,
trigger-dev, clerk, linear, openai, anthropic, databricks, snow, temporal,
nomad, consul, tailscale, sops, age, biome, vitest, playwright, astro, expo,
eas, shopify, sanity, contentful, sst, dvc, wandb, mlflow, ollama, semgrep,
trivy, syft, grype, cosign, slsa-verifier
```

Selection principles:

- prefer CLIs whose command surface changes quickly
- prefer CLIs where repo configuration, auth state, plugins, or daemons affect behavior
- prefer CLIs where machine-readable output matters to agents
- include safe security and supply-chain tools with clear JSON/SARIF/SBOM modes
- keep every target optional until installation and fixture requirements are standardized

Targets tagged with `resolve-canonical-command` or `resolve-official-surface` should not be treated as validated until the package, executable name, and safe-probe policy are confirmed.

---

## Agent-Harness Corpus

`benchmarks/agent-harness-corpus.json` is deliberately separate from the operational CLI corpora. Agent harnesses are strategically important, but they are sensitive, self-referential, and policy-heavy. Mixing them into the main corpus would blur CLIARE's claim that it helps agents operate ordinary CLIs.

Current target IDs:

```text
codex, claude, gemini, aider, goose, opencode, swe-agent, openhands
```

Measure this corpus only under explicit local safety expectations. Many harness CLIs can edit files, call tools, invoke models, use credentials, or run long-lived interactive sessions.

---

## Candidate Backlog

These candidates are not currently encoded in a checked-in manifest. Add them only when the executable name, install source, safe-probe posture, and expected fixture/context needs are clear.

| Category | Candidate CLIs |
|---|---|
| Cloud | `aws`, `gcloud`, `az` |
| Containers and Kubernetes | `kubectl`, `helm`, `podman`, `kind`, `minikube`, `kustomize`, `tilt` |
| Infrastructure as code | `terraform`, `tofu`, `pulumi`, `cdktf`, `ansible` |
| Deployment and PaaS | `wrangler`, `netlify`, `flyctl`, `heroku`, `render`, `railway`, `firebase` |
| Databases and data | `neon`, `prisma`, `dbt`, `atlas` |
| Package and language tooling | `pnpm`, `yarn`, `bun`, `uv`, `go` |
| Secrets and security | `vault`, `doppler`, `infisical`, `op` |
| GitOps and CI | `argocd`, `flux`, `glab`, `gitlab-runner`, `circleci`, `buildkite-agent` |
| AI infrastructure | `hf`, `modal`, `replicate` |

---

## What To Record Per Measurement

Generated artifacts should carry the measurement facts. When summarizing a run, prefer links to artifacts over hand-maintained score values.

Record:

- target binary path and version
- installation channel
- operating system and architecture
- CLIARE version and score model version
- manifest path and target ID
- profile, depth, probe budget, expected-value threshold, timeout, and concurrency
- measured score and dimension subscores
- traversal status, observed depth, completed probes, and budget exhaustion
- command count by agent suitability
- precondition counts for auth, local context, fixture data, network, and runtime dependencies
- output contracts discovered and parse successes
- side effects and credential-like side-effect counts
- top maintainer, harness, and security findings
- disposition decisions, when a maintainer has reviewed issues

---

## Maintenance Rules

1. Keep manifests as the source of truth for targets and budgets.
2. Keep this document focused on corpus intent, target status, and gaps.
3. Do not paste stale scores into this document.
4. Do not call an expected score band a truth label.
5. Do not call a corpus calibrated until a separate truth set and calibration report exist.
6. Keep missing local binaries optional unless CI depends on them.
7. Add fixture-heavy targets only after defining safe operands, expected side effects, and context labels.
