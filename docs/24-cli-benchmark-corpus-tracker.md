# 24 - CLI Benchmark Corpus Tracker

> **Scope:** Candidate vendor and popular open-source CLIs for the CLIARE benchmark corpus.
> **Status:** Planning Tracker

---

## Purpose

This tracker records the real-world CLI corpus used to mature CLIARE's benchmark suite. The emphasis is not raw popularity alone. The corpus represents CLIs that agent harnesses commonly need when operating real software projects: source control platforms, clouds, containers, Kubernetes, infrastructure-as-code, deployment platforms, databases, package managers, secrets, observability, and AI-application infrastructure.

This tracker intentionally excludes basic shell staples such as `python`, `grep`, `sed`, `awk`, `jq`, `curl`, `bash`, and coreutils. Those tools matter, but agents are already heavily pretrained on them and they are not the primary benchmark pressure point for CLIARE v0. It also excludes AI-agent harness CLIs from the main product corpus; those should be measured in a separate agent-harness corpus so they do not blur the claim that CLIARE helps agents operate ordinary operational CLIs.

The launch corpus should deliberately balance familiar anchors with newer or faster-moving CLIs that are less likely to be memorized by pretrained models. A useful initial mix is:

| Slice | Target Share | Purpose |
|---|---:|---|
| Legacy/high-pretraining anchors | 20% | Keep stable baselines such as `git`, `gh`, `docker`, `kubectl`, `aws`, `npm`, and `cargo` for regression and comparability. |
| Modern fast-moving developer/platform CLIs | 50% | Stress agent navigation on command surfaces that change quickly and are less likely to be fully internalized by model pretraining. |
| Auth/context-heavy SaaS CLIs | 20% | Measure preconditions, fixtures, profile state, sensitive output, and agent-routing constraints. |
| Dynamic/plugin/weird CLIs | 10% | Exercise plugin dispatch, project-local commands, generated help, local daemons, and framework-specific edge cases. |

For launch planning, tag each corpus entry with:

- `pretraining_exposure`: `legacy_high`, `medium`, or `new_low`
- `release_velocity`: `stable`, `fast`, or `very_fast`
- `context_shape`: `clean`, `auth`, `repo`, `daemon`, `cloud`, or `fixture`
- `skill_value`: `high` when an evidence-backed command index or agent skill is likely to prevent trial-and-error command discovery

The blank findings columns are intentional. As benchmark artifacts are generated, each row records the artifact folder, score, traversal status, and the highest-value findings from the persona and issue reports.

Reference artifact layout:

```text
benchmarks/corpus/<cli-id>/
  README.md
  expected.json
  fixtures/
  notes.md

.cliare-bench/<cli-id>/
  artifact-map.md
  command-index.json
  command-index.md
  scorecard.json
  issues.md
  persona-harness.md
  persona-maintainer.md
```

Reference measurement command:

```sh
cliare measure <command> --out .cliare-bench/<cli-id> --profile deep --max-depth 12 --max-probes 5000 --refresh
cliare describe .cliare-bench/<cli-id> --write
```

Reference vendor calibration manifest:

```sh
cliare benchmark --manifest benchmarks/vendor-calibration-corpus.json --out .cliare-vendor-calibration --refresh
```

Reference low-pretraining launch manifest:

```sh
cliare benchmark --manifest benchmarks/launch-low-pretraining-corpus.json --out .cliare-launch-low-pretraining --refresh
```

---

## Named Vendor Calibration Set

This set records vendor CLIs that are especially relevant to agent harnesses because they expose business-critical SaaS, deployment, data, communication, and AI-media workflows. Some entries already appear in the broader P0 corpus. They are repeated here so calibration work can track them as a deliberate product-facing benchmark set with train/validation/holdout labels.

| Priority | CLI / Product | Command | Primary Surface | Benchmark Folder | Split | Truth Labels | Latest Score | Follow-Up |
|---:|---|---|---|---|---|---|---:|---|
| 1 | GitHub CLI | `gh` | GitHub repositories, pull requests, issues, Actions, releases, auth, and API workflows. | `benchmarks/corpus/gh` | holdout candidate | partial |  | Existing row remains the reference; add human-verified command, flag, JSON-field, auth, and local-repository labels before certification use. |
| 2 | Stripe CLI | `stripe` | Payments setup, payment events, fixtures, local webhook forwarding, auth, and API-backed resource workflows. |  | train candidate |  |  | Needs fixture strategy for webhook listener commands and safe event-producing probes. |
| 3 | Supabase CLI | `supabase` | Local Postgres stack, auth, storage, migrations, functions, secrets, projects, and local service lifecycle. |  | validation candidate |  |  | Needs clean, linked-project, and local-stack contexts to separate auth, workspace, and runtime-dependency behavior. |
| 4 | Valyu CLI | `valyu` | Web search and real-time specialized data access from the terminal. |  | train candidate |  |  | Verify installed command name, auth model, query fixture requirements, and machine-readable output contracts. |
| 5 | PostHog CLI | `posthog` | Analytics setup, project instrumentation, deployment/self-hosting support, auth, and event/debug workflows. |  | train candidate |  |  | Verify command availability and whether the surface is official, plugin-backed, or package-manager mediated. |
| 6 | ElevenLabs CLI | `elevenlabs` | Text-to-speech, speech-to-text, voice cloning, model selection, files, auth, and media output workflows. |  | validation candidate |  |  | Needs safe fixture media inputs and output-file side-effect expectations. |
| 7 | Ramp CLI | `ramp` | Expense, card, vendor, reimbursement, and finance-operation workflows. |  | holdout candidate |  |  | Verify official CLI availability, auth constraints, and sensitive-output policy before probing beyond help. |
| 8 | Google Workspace CLI | `google-workspace` | Gmail, Drive, Calendar, Docs, Sheets, Admin, contacts, and workspace automation from terminal. |  | holdout candidate |  |  | Resolve canonical CLI/package, define scoped fixture accounts, and label auth-sensitive commands separately. |
| 9 | AgentMail CLI | `agentmail` | Email inboxes, transactional email, local webhook testing, address management, auth, and message workflows. |  | train candidate |  |  | Needs safe mailbox fixture and webhook/listener classification. |
| 10 | Vercel CLI | `vercel` | App deployment, projects, environment variables, domains, logs, teams, auth, and frontend platform workflows. |  | validation candidate |  |  | Needs project-context and auth-context runs; existing P0 row remains the general corpus reference. |

Calibration split assignments are provisional. Final split placement should avoid leakage: a CLI family used to tune model weights should not be used to claim holdout performance for the same model version.

---

## P0 Corpus

These are the first CLIs to measure. They cover the most common operational surfaces an agent harness must navigate.

| Priority | CLI | Category | Why Harnesses Use It | Benchmark Folder | Latest Score | Traversal Status | Findings / Follow-Up |
|---:|---|---|---|---|---:|---|---|
| 1 | `gh` | Source control / GitHub | Repositories, pull requests, issues, Actions, releases, auth, and API calls. | `benchmarks/corpus/gh` | `99` | Host-context verification; 185 commands indexed; 177 runtime-confirmed; 560/560 probes; depth 3/4; budget exhausted with 87 frontier candidates; no side effects. | No output parse-failure findings. 19 advertised output contracts are precondition-blocked by auth, local repository context, or fixture-required operands/flags. 7 command candidates still need runtime confirmation. |
| 2 | `aws` | Cloud | AWS resource inspection, provisioning, service APIs, auth profiles, and deployment workflows. |  |  |  |  |
| 3 | `gcloud` | Cloud | Google Cloud resource management, auth, projects, config, deployments, and logs. |  |  |  |  |
| 4 | `az` | Cloud | Azure resource management, deployments, identity, storage, containers, and app services. |  |  |  |  |
| 5 | `docker` | Containers | Images, containers, logs, networks, volumes, build, compose, and local runtime inspection. |  |  |  |  |
| 6 | `kubectl` | Kubernetes | Kubernetes inspection, rollout, logs, exec, events, YAML workflows, and cluster state. |  |  |  |  |
| 7 | `helm` | Kubernetes | Kubernetes package installation, upgrades, chart inspection, values, and release state. |  |  |  |  |
| 8 | `terraform` | Infrastructure as code | Plan, apply, state, providers, modules, workspace, and drift-management workflows. |  |  |  |  |
| 9 | `tofu` | Infrastructure as code | OpenTofu workflows compatible with Terraform-style IaC and provider surfaces. |  |  |  |  |
| 10 | `pulumi` | Infrastructure as code | Stack, config, preview, deploy, state, plugin, and cloud resource workflows. |  |  |  |  |
| 11 | `wrangler` | Edge / Cloudflare | Workers, Pages, KV, R2, D1, secrets, deployments, and local dev workflows. |  |  |  |  |
| 12 | `vercel` | Deployment platform | Projects, deployments, env vars, logs, domains, teams, and frontend platform workflows. |  |  |  |  |
| 13 | `netlify` | Deployment platform | Deploys, sites, functions, env vars, logs, local dev, and build workflows. |  |  |  |  |
| 14 | `flyctl` | Deployment platform | Apps, machines, volumes, secrets, regions, logs, deploys, and platform state. |  |  |  |  |
| 15 | `supabase` | Database / app platform | Local stack, database, migrations, functions, secrets, projects, auth, and storage. |  |  |  |  |
| 16 | `firebase` | App platform | Hosting, emulators, functions, deploys, projects, auth, and app backend workflows. |  |  |  |  |
| 17 | `stripe` | Payments / SaaS | Webhook testing, event forwarding, fixtures, resources, auth, and API-backed workflows. |  |  |  |  |
| 18 | `sentry-cli` | Observability | Releases, deploys, sourcemaps, debug files, projects, orgs, and CI integration. |  |  |  |  |
| 19 | `datadog-ci` | Observability | CI visibility, synthetics, sourcemaps, test visibility, and monitoring automation. |  |  |  |  |
| 20 | `heroku` | PaaS | Apps, config vars, add-ons, deploys, logs, dynos, pipelines, and platform state. |  |  |  |  |
| 21 | `neon` | Database | Postgres projects, branches, databases, connection strings, auth, and cloud DB state. |  |  |  |  |
| 22 | `prisma` | Database tooling | Schema, migrations, generate, introspection, studio, and application DB workflows. |  |  |  |  |
| 23 | `dbt` | Data tooling | Data builds, tests, docs, models, profiles, and analytics engineering pipelines. |  |  |  |  |
| 24 | `atlas` | Database / MongoDB | MongoDB Atlas project, cluster, database, auth, and cloud resource workflows. |  |  |  |  |
| 25 | `npm` | Package manager | Scripts, package install, audit, publish, workspaces, and dependency inspection. |  |  |  |  |
| 26 | `pnpm` | Package manager | Fast JS package management, workspaces, scripts, lockfiles, and monorepo workflows. |  |  |  |  |
| 27 | `yarn` | Package manager | JS package management, workspaces, scripts, plugins, and dependency workflows. |  |  |  |  |
| 28 | `bun` | Runtime / package manager | JS runtime, package manager, scripts, tests, bundling, and modern app workflows. |  |  |  |  |
| 29 | `deno` | Runtime | Runtime permissions, tasks, format/lint/test, compilation, and deployment-adjacent workflows. |  |  |  |  |
| 30 | `uv` | Package manager / runtime tooling | Python project, environment, dependency, lockfile, and tool execution workflows. |  |  |  |  |
| 31 | `cargo` | Build system / package manager | Rust build, test, check, publish, features, workspaces, and package workflows. |  |  |  |  |
| 32 | `go` | Build system / language tooling | Go build, test, mod, env, generate, tool, and module workflows. |  |  |  |  |
| 33 | `ansible` | Automation | Inventory, playbooks, modules, vault, config, and infrastructure automation. |  |  |  |  |
| 34 | `vault` | Secrets | Secrets, auth methods, policies, tokens, KV, leases, and secure automation. |  |  |  |  |
| 35 | `argocd` | GitOps | Application sync, diff, rollback, status, clusters, repos, and GitOps state. |  |  |  |  |
| 36 | `flux` | GitOps | Sources, Kustomizations, Helm releases, reconciliation, bootstrap, and cluster state. |  |  |  |  |

---

## P0.5 Low-Pretraining / Fast-Moving Launch Corpus

These CLIs should be precreated before launch because they pressure-test CLIARE's core claim better than heavily memorized staples. They are newer, faster moving, auth/context-heavy, workflow-specific, or likely to need a generated command index and agent skill before a harness can use them reliably.

Rows with "resolve canonical command" should not be measured until the official package, installed executable name, and basic safe-probe policy are verified.

| Launch Priority | CLI | Category | Pretraining Exposure | Context Shape | Why Harnesses Use It | Benchmark Folder | Findings / Follow-Up |
|---:|---|---|---|---|---|---|---|
| L1 | `mise` | Toolchain / task manager | new_low | repo | Polyglot tool versions, environment activation, tasks, and project bootstrap in modern repos. |  | Verify task-discovery output and whether project-local config changes command behavior. |
| L2 | `just` | Task runner | medium | repo | Common repository task interface; agents need to discover canonical build/test/lint commands instead of guessing. |  | Add fixture repos with nested and included justfiles. |
| L3 | `task` | Task runner | medium | repo | Go Task workflows, aliases, includes, variables, and monorepo task execution. |  | Add fixture taskfiles and compare machine-readable task listing if available. |
| L4 | `turbo` | Monorepo build orchestration | new_low | repo | JS/TS monorepos, pipeline tasks, cache, affected builds, and workspace graph operations. |  | Needs package-manager fixture and repo context. |
| L5 | `nx` | Monorepo build orchestration | medium | repo | Deep plugin-backed command surface for builds, tests, generators, affected projects, and workspace graph operations. |  | Plugin and workspace context likely change command surface. |
| L6 | `ruff` | Python lint / format | new_low | repo | Fast Python linting, formatting, config discovery, and fix workflows used in coding-agent loops. |  | Track JSON/SARIF output contracts and safe fix/diff modes. |
| L7 | `pixi` | Python / Conda environment manager | new_low | repo | Newer project, environment, lockfile, task, and package workflows. |  | Needs clean and repo-context measurements. |
| L8 | `dagster` | Data orchestration | medium | repo, daemon | Data pipeline development, local dev servers, asset/job commands, definitions validation, and deployment workflows. |  | Separate help-only, project, and daemon-backed contexts. |
| L9 | `airflow` | Data orchestration | legacy_high | daemon, fixture | Established workflow orchestration with scheduler/database state, DAG inspection, backfills, and admin commands. |  | Useful contrast against newer `dagster`; daemon and database fixtures required. |
| L10 | `convex` | Backend platform | new_low | auth, repo, cloud | Modern app backend, dev server, deployment, functions, data, env vars, and project linking. |  | Requires clean, linked-project, and auth-context measurements. |
| L11 | `inngest` | Workflow / events | new_low | repo, cloud | Event-driven app workflows, local dev server, functions, deploys, and cloud integration. |  | Verify canonical command and local fixture strategy. |
| L12 | `trigger` / `trigger.dev` | Workflow / jobs | new_low | repo, cloud | Background jobs, local dev, deploys, environment selection, and observability for app workflows. |  | Resolve canonical command and package before measuring. |
| L13 | `clerk` | Auth platform | new_low | auth, cloud | Auth app configuration, domains, users, environment variables, and project workflows. |  | Resolve official CLI surface and sensitive-output policy. |
| L14 | `linear` | Project management | new_low | auth, cloud | Issues, projects, cycles, comments, and planning workflows that agents often need to coordinate. |  | Verify official CLI availability and auth fixture strategy. |
| L15 | `openai` | AI platform | medium | auth, cloud | Model, file, batch, eval, fine-tuning, and API workflow operations from terminal. |  | Measure as platform tooling, not as an agent harness. |
| L16 | `anthropic` | AI platform | new_low | auth, cloud | Model and API workflows when an official or canonical CLI exists. |  | Resolve canonical command/package before measuring. |
| L17 | `databricks` | Enterprise data / AI | medium | auth, cloud, repo | Jobs, repos, clusters, bundles, secrets, SQL warehouses, and workspace automation. |  | Needs workspace-scoped auth and safe fixture workspace. |
| L18 | `snow` | Data warehouse | new_low | auth, cloud, repo | Snowflake databases, warehouses, stages, Snowpark, apps, and deployment workflows. |  | Verify canonical Snowflake CLI command and auth fixture strategy. |
| L19 | `temporal` | Workflow engine | medium | daemon, cloud | Workflow execution, namespaces, workers, schedules, local dev server, and cloud workflows. |  | Measure local server and cloud-auth contexts separately. |
| L20 | `nomad` | Scheduler / infrastructure | medium | daemon, cloud | Job planning, allocation inspection, deployments, logs, and cluster operations. |  | Requires local or fixture cluster context for deeper probes. |
| L21 | `consul` | Service networking / config | medium | daemon, cloud | KV, service discovery, intentions, config entries, and datacenter operations. |  | Requires local agent or fixture cluster context. |
| L22 | `tailscale` | Networking / identity | medium | auth, daemon | Network status, device identity, SSH, funnel/serve, ACL-adjacent diagnostics, and admin operations. |  | Sensitive; help-only and authenticated contexts should be separate. |
| L23 | `sops` | Secrets / encryption | medium | repo, fixture | Encrypted config files, KMS/age/PGP workflows, edit/decrypt/encrypt, and CI secrets handling. |  | Needs safe encrypted fixture files and side-effect expectations. |
| L24 | `age` | Encryption | medium | fixture | File encryption/decryption workflows, recipient/key handling, and safe local transformations. |  | Needs fixture files and credential-like path policy. |
| L25 | `biome` | JS/TS lint / format | new_low | repo | Modern lint/format/check workflows, config discovery, and machine-readable diagnostics. |  | Track JSON output, safe write modes, and repo config effects. |
| L26 | `vitest` | JS/TS test runner | new_low | repo | Test selection, watch mode, reporters, coverage, and monorepo test workflows. |  | Need non-watch safe probes and reporter output contracts. |
| L27 | `playwright` | Browser testing | medium | repo, daemon | Browser install, test, codegen, trace, report, and web-app verification workflows. |  | Avoid GUI/browser-launching probes unless fixtures declare them. |
| L28 | `astro` | Web framework | new_low | repo | Modern site/app build, dev, preview, integrations, and project scaffolding workflows. |  | Separate scaffold/create commands from safe project commands. |
| L29 | `expo` | Mobile app platform | medium | repo, cloud | React Native app dev, build, prebuild, doctor, credentials, and publish workflows. |  | Auth and project context needed; avoid long-running dev server by default. |
| L30 | `eas` | Mobile app build platform | new_low | auth, repo, cloud | Expo Application Services build, submit, credentials, channels, and deployment workflows. |  | Sensitive auth and cloud actions require explicit policy. |
| L31 | `shopify` | Commerce platform | medium | auth, repo, cloud | Theme, app, extension, store, deployment, and local dev workflows. |  | Needs fixture app/store strategy and sensitive auth classification. |
| L32 | `sanity` | CMS / content platform | medium | auth, repo, cloud | Content studio, schema, deploy, dataset, and project workflows. |  | Measure clean, project, and auth contexts. |
| L33 | `contentful` | CMS / content platform | medium | auth, cloud | Space, environment, content model, migration, and publish workflows. |  | Sensitive content operations require fixtures and policy. |
| L34 | `sst` | App / infrastructure framework | new_low | repo, cloud | Modern full-stack app deployments, local dev, secrets, stages, and AWS-backed workflows. |  | High skill value; command surface depends on project config. |
| L35 | `dvc` | Data / ML versioning | medium | repo, fixture | Data pipelines, remotes, experiments, metrics, and reproducible ML project workflows. |  | Needs fixture repo and safe local data paths. |
| L36 | `wandb` | ML platform | medium | auth, repo, cloud | Experiment tracking, artifacts, sweeps, reports, and project state. |  | Auth-sensitive; verify offline/local modes. |
| L37 | `mlflow` | ML platform | medium | daemon, repo | Experiments, models, tracking server, registry, artifacts, and local workflow automation. |  | Separate local server and file-backed contexts. |
| L38 | `ollama` | Local model runtime | new_low | daemon | Local model pull/run/list/serve workflows used by agent and developer environments. |  | Daemon and model-download behavior must be controlled. |
| L39 | `semgrep` | Security analysis | medium | repo, cloud | Static analysis, rule packs, CI scans, SARIF/JSON output, and autofix workflows. |  | Track machine-readable output and safe autofix behavior. |
| L40 | `trivy` | Security / SBOM | medium | repo, fixture | Vulnerability scanning for images, filesystems, config, SBOM, and CI workflows. |  | Avoid network-heavy probes unless fixture policy allows. |
| L41 | `syft` | SBOM | medium | repo, fixture | SBOM generation from images, filesystems, and packages with JSON/SPDX/CycloneDX output. |  | Good machine-output contract candidate. |
| L42 | `grype` | Vulnerability scanning | medium | repo, fixture | Vulnerability scanning from SBOMs/images/filesystems with machine-readable output. |  | Pair with `syft` fixtures. |
| L43 | `cosign` | Supply-chain signing | medium | auth, fixture | Container signing, verification, attestations, keyless flows, and policy workflows. |  | Sensitive key material and network identity require strict fixtures. |
| L44 | `slsa-verifier` | Supply-chain verification | new_low | fixture | Provenance and artifact verification workflows for release pipelines. |  | Good low-pretraining, high-agent-value release tool. |

---

## P1 Expansion

These should follow once the P0 suite produces stable measurements and the corpus folder layout is in place.

| Priority | CLI | Category | Why Harnesses Use It | Benchmark Folder | Latest Score | Traversal Status | Findings / Follow-Up |
|---:|---|---|---|---|---:|---|---|
| 37 | `podman` | Containers | Docker-compatible OSS container, image, pod, and local runtime workflows. |  |  |  |  |
| 38 | `gitlab-runner` | CI | Runner lifecycle, local execution, job handling, and CI debugging. |  |  |  |  |
| 39 | `glab` | Source control / GitLab | GitLab issues, merge requests, pipelines, releases, auth, and API workflows. |  |  |  |  |
| 40 | `circleci` | CI | Pipeline validation, local job execution, contexts, orbs, and project workflows. |  |  |  |  |
| 41 | `buildkite-agent` | CI | Agent lifecycle, jobs, artifacts, annotations, and Buildkite pipeline execution. |  |  |  |  |
| 42 | `sam` | Serverless | AWS serverless build, package, deploy, local invoke, and template workflows. |  |  |  |  |
| 43 | `serverless` | Serverless | Multi-provider serverless deploys, config, plugins, logs, and function workflows. |  |  |  |  |
| 44 | `cdktf` | Infrastructure as code | Terraform CDK synthesis, deploy, diff, and stack workflows. |  |  |  |  |
| 45 | `kustomize` | Kubernetes | Kubernetes manifest composition, overlays, patches, and YAML generation. |  |  |  |  |
| 46 | `kind` | Kubernetes | Local Kubernetes clusters for tests and reproducible development environments. |  |  |  |  |
| 47 | `minikube` | Kubernetes | Local Kubernetes cluster lifecycle, addons, services, and debugging. |  |  |  |  |
| 48 | `tilt` | Kubernetes / local dev | Local development orchestration for Kubernetes apps and service graphs. |  |  |  |  |
| 49 | `hf` | AI application infrastructure | Hugging Face model, dataset, Space, auth, and job workflows. |  |  |  |  |
| 50 | `modal` | AI application infrastructure | Serverless compute, functions, apps, secrets, deploys, and logs for AI workloads. |  |  |  |  |
| 51 | `replicate` | AI application infrastructure | Model inference, deployment, auth, predictions, and model workflow automation. |  |  |  |  |
| 52 | `render` | Deployment platform | Services, deploys, env vars, logs, jobs, and web service workflows. |  |  |  |  |
| 53 | `railway` | Deployment platform | Projects, services, env vars, deploys, logs, and database workflows. |  |  |  |  |
| 54 | `doppler` | Secrets / config | Secrets, environment config, project/service selection, and CI injection. |  |  |  |  |
| 55 | `infisical` | Secrets / config | Secrets, projects, environments, identity, and config sync workflows. |  |  |  |  |
| 56 | `op` | Secrets / password manager | 1Password secrets, vaults, items, service accounts, and shell injection. |  |  |  |  |

---

## Separate Agent-Harness Corpus

These CLIs are strategically important, but they should be measured outside the main product corpus. The goal of the main corpus is to prove CLIARE helps agents operate ordinary operational CLIs. Agent harness CLIs can otherwise dominate the story with self-referential behavior, rapidly changing product surfaces, and agent-specific safety policies.

Reference manifest:

```sh
cliare benchmark --manifest benchmarks/agent-harness-corpus.json --out .cliare-agent-harness --refresh
```

| Priority | CLI | Category | Why Measure Separately | Benchmark Folder | Findings / Follow-Up |
|---:|---|---|---|---|---|
| A1 | `codex` | Agent harness | Codex CLI exposes agent execution, approvals, MCP, skills, plugins, and local shell behavior. |  | Measure only under explicit harness-safety policy. |
| A2 | `claude` | Agent harness | Claude Code exposes terminal, skills, hooks, MCP, permissions, and project-specific agent workflows. |  | Measure local and managed-skill contexts separately. |
| A3 | `gemini` | Agent harness | Gemini CLI-style tools are relevant to cross-agent skill portability and command-surface comparison. |  | Resolve canonical command and installation channel. |
| A4 | `aider` | Agent harness | Established coding-agent CLI with repo, model, git, and edit workflows. |  | Useful baseline for older agent CLI surfaces. |
| A5 | `goose` | Agent harness | Open-source agent harness with extensions/tools and project workflows. |  | Resolve installed command and safe-probe policy. |
| A6 | `opencode` | Agent harness | Terminal coding-agent workflow and model/tool configuration. |  | Resolve canonical command and contexts. |
| A7 | `swe-agent` | Agent harness / benchmark | Research harness for software-engineering agents and benchmark workflows. |  | Useful for bridge between CLIARE and benchmark infrastructure. |
| A8 | `openhands` | Agent harness | Software-engineering agent platform with CLI/deployment surfaces. |  | Resolve command and safe local execution mode. |

---

## Measurement Notes

For each CLI, record the following after measurement:

- binary path and version
- installation channel
- operating system and architecture
- CLIARE version and score model
- profile, depth, probe budget, and concurrency
- traversal status and frontier pressure
- score and dimension subscores
- command count by `agent_suitability`
- top maintainer findings
- top harness findings
- top security findings
- whether fixtures are needed
- whether auth, local-context, profile, dependency, or other preconditions were observed
- whether public leaderboard use would be appropriate

The first pass should optimize for breadth and comparability. The second pass should add per-CLI fixture notes, expected command families, and human-verified truth labels for calibration.
