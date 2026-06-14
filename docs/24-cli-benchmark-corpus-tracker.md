# 24 - CLI Benchmark Corpus Tracker

> **Scope:** Candidate vendor and popular open-source CLIs for the CLIARE benchmark corpus.
> **Status:** Planning Tracker

---

## Purpose

This document tracks the real-world CLI corpus CLIARE should measure as the benchmark suite matures. The emphasis is not raw popularity alone. The corpus should represent CLIs that agent harnesses commonly need when operating real software projects: source control platforms, clouds, containers, Kubernetes, infrastructure-as-code, deployment platforms, databases, package managers, secrets, observability, and AI-application infrastructure.

This tracker intentionally excludes basic shell staples such as `python`, `grep`, `sed`, `awk`, `jq`, `curl`, `bash`, and coreutils. Those tools matter, but agents are already heavily pretrained on them and they are not the primary benchmark pressure point for CLIARE v0. It also excludes AI-agent harness CLIs for this pass.

The blank findings columns are intentional. When benchmark artifacts are generated, each row should be updated with the artifact folder, score, traversal status, and the highest-value findings from the persona and issue reports.

Suggested artifact layout:

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

Suggested measurement command:

```sh
cliare measure <command> --out .cliare-bench/<cli-id> --profile deep --max-depth 12 --max-probes 5000 --refresh
cliare describe .cliare-bench/<cli-id> --write
```

---

## P0 Corpus

These are the first CLIs to measure. They cover the most common operational surfaces an agent harness must navigate.

| Priority | CLI | Category | Why Harnesses Use It | Benchmark Folder | Latest Score | Traversal Status | Findings / Follow-Up |
|---:|---|---|---|---|---:|---|---|
| 1 | `gh` | Source control / GitHub | Repositories, pull requests, issues, Actions, releases, auth, and API calls. | `benchmarks/corpus/gh` | `94.4` | Complete: converged; 185 commands indexed; 184 runtime-confirmed; 792/5000 probes; depth 3/12; frontier 0; no side effects. | High: 34 advertised output modes did not parse. Medium: 8 output modes need fixtures; 1 help-unavailable command. Low: 1 flag-grammar gap. |
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
- whether auth/profile/project preconditions were observed
- whether public leaderboard use would be appropriate

The first pass should optimize for breadth and comparability. The second pass should add per-CLI fixture notes, expected command families, and human-verified truth labels for calibration.
