# 08 - CI, Publishing, And Leaderboard Direction

> **Scope:** Current CI action, CI artifacts, command-index registry workflow, local publication patterns, and future hosted publishing/leaderboard requirements.
> **Status:** Current Implementation And Future Publishing Direction

---

## Summary

CLIARE is local-first. The current implementation runs the target CLI in the maintainer's own environment, usually CI, and writes an artifact directory that can be uploaded, reviewed, or copied into a repository.

Current reliable workflow:

```text
Project CI builds CLI
Project CI installs or builds cliare
Project CI runs cliare measure or cliare guard
Project CI writes scorecard, command index, issues, SARIF, JUnit, and summary artifacts
Project CI uploads those artifacts or opens a command-index PR
```

Current CLIARE does **not** provide:

- `cliare publish`
- `cliare certify`
- hosted scorecard ingestion
- hosted badge endpoints
- public leaderboard ranking
- calibrated public certification

The public score and leaderboard direction remains valid, but it must stay downstream of calibration, reproducible profiles, model governance, and provenance.

---

## Current GitHub Action

The repository ships a composite GitHub Action in [`action.yml`](../../action.yml).

It supports two modes:

- `measure`
- `guard`

It expects the `cliare` executable to already be available. Install CLIARE in a prior workflow step.

Example:

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

      - name: Install CLIARE
        run: cargo install cliare

      - name: Build CLI
        run: make build

      - name: Measure CLI
        uses: modiqo/cliare@main
        with:
          target: ./dist/mycli
          out: .cliare/mycli
          profile: standard
          extra-args: --refresh
```

The action:

- runs `cliare measure` or `cliare guard`
- appends `.cliare/.../summary.md` to the GitHub step summary
- collects output paths
- optionally uploads the artifact directory with `actions/upload-artifact`

It does not currently upload SARIF to code scanning by itself. A workflow can add a separate upload step using the `sarif` output.

---

## Action Inputs

Current inputs:

| Input | Required | Default | Meaning |
|---|---:|---|---|
| `target` | yes | none | Path or PATH-resolved command name for the CLI to measure. |
| `mode` | no | `measure` | `measure` or `guard`. |
| `baseline` | guard only | none | Baseline scorecard path for guard mode. |
| `policy` | no | none | Optional CLIARE policy JSON file for guard mode. |
| `allowed-drop` | no | `0` | Allowed score drop before guard fails. |
| `out` | no | `.cliare` | Output directory for CLIARE artifacts. |
| `profile` | no | `standard` | Traversal profile: `quick`, `standard`, or `deep`. |
| `concurrency` | no | none | Optional maximum number of probes to run concurrently. |
| `cliare-command` | no | `cliare` | CLIARE executable to invoke. |
| `extra-args` | no | empty | Additional trusted arguments passed to CLIARE. |
| `upload-artifacts` | no | `true` | Whether to upload the artifact directory. |
| `artifact-name` | no | `cliare-artifacts` | Uploaded artifact name. |

`extra-args` is intentionally a trusted escape hatch. Use it for options such as `--refresh`, `--max-depth`, `--max-probes`, `--execution-mode`, or runtime context flags.

---

## Action Outputs

Current outputs:

| Output | Meaning |
|---|---|
| `score` | Numeric score read from `scorecard.json`. |
| `scorecard` | Path to `scorecard.json`. |
| `summary` | Path to `summary.md`. |
| `sarif` | Path to `findings.sarif`. |
| `junit` | Path to `junit.xml`. |

Artifact filenames are:

```text
scorecard.json
summary.md
findings.sarif
junit.xml
```

The old name `sarif.json` is not the current artifact name.

---

## Measure Mode

Use measure mode for first adoption, observation, scheduled audits, or non-blocking PR visibility:

```yaml
- name: Measure CLI
  uses: modiqo/cliare@main
  with:
    target: ./dist/mycli
    mode: measure
    out: .cliare/mycli
    profile: standard
    extra-args: --refresh
```

`measure` writes the artifact directory. It is primarily a reporting and artifact-generation step.

---

## Guard Mode

Use guard mode when you have accepted a baseline scorecard:

```yaml
- name: Guard CLI
  uses: modiqo/cliare@main
  with:
    target: ./dist/mycli
    mode: guard
    baseline: .cliare-baseline/mycli/scorecard.json
    policy: cliare.policy.json
    allowed-drop: "1"
    out: .cliare/mycli
    profile: deep
    extra-args: --refresh
```

`guard`:

1. Runs a fresh measurement.
2. Reads the baseline score from the supplied scorecard.
3. Computes `current_total - baseline_total`.
4. Fails when the drop is larger than `allowed-drop`.
5. Fails when a supplied policy has failures.
6. Writes `summary.md`, `findings.sarif`, and `junit.xml`.

For policy schema details, see [Scoring And Improvement Tracking](../model/scoring-and-improvement-tracking.md).

---

## Uploading SARIF

The action exposes the SARIF path, but does not upload it automatically.

Example:

```yaml
- name: Upload CLIARE SARIF
  if: always()
  uses: github/codeql-action/upload-sarif@<current-major>
  with:
    sarif_file: ${{ steps.cliare.outputs.sarif }}
```

Use the current supported major version of GitHub's CodeQL SARIF upload action for the repository. The CLIARE contract is the `sarif` output path.

If using GitHub code scanning, grant the workflow the required permission:

```yaml
permissions:
  contents: read
  security-events: write
```

---

## CI Artifacts

CLIARE writes a compact artifact directory suitable for CI upload:

| Artifact | Current Use |
|---|---|
| `scorecard.json` | Score, subscores, coverage, findings, runtime context, and model provenance. |
| `summary.md` | GitHub step summary and concise CI review page. |
| `findings.sarif` | Code scanning / SARIF-compatible finding export. |
| `junit.xml` | Test-report compatible finding and guard-policy export. |
| `issues.json` | Full review queue with affected commands, evidence, and verification hints. |
| `issues.md` | Markdown review queue. |
| `command-index.json` | Agent-facing command catalog. |
| `command-index.md` | Human-readable command catalog. |
| `condition-dictionary.csv` | CSV decoder for report labels, condition meanings, examples, and first actions. |
| `shape.json` | Evidence-derived command shape model. |
| `evidence.jsonl` | Runtime evidence log. |
| `report.md` | Human-readable scorecard report. |
| `README.md` | Artifact navigation guide. |
| `AGENT_SKILL.md` | Agent-facing artifact review skill. |

Retention is controlled by the CI platform or workflow configuration. CLIARE itself does not enforce retention windows.

---

## CLIARE On CLIARE

The repository dogfoods the local composite action in `.github/workflows/cliare-on-cliare.yml`.

Current behavior:

- runs on pull requests, pushes to `main`, a weekly schedule, and manual dispatch
- builds `target/debug/cliare`
- measures that freshly built binary with the local action
- uses `quick` on pull requests, `standard` on main pushes, and `deep` on scheduled runs unless overridden
- uploads CLIARE artifacts
- prints score, subscores, issue review queue, and disposition hints into the GitHub job summary
- copies `.github/cliare-on-cliare/issue-dispositions.json` into the run when present

This is the current model for “CI feedback loop”: score, artifacts, issues, and dispositions are visible in the CI job without requiring hosted CLIARE infrastructure.

---

## Current Command-Index Registry Workflow

The repository includes a manual workflow for producing public command-index entries:

```text
Actions -> Extract Command Index PR
```

Workflow file:

```text
.github/workflows/extract-command-index-pr.yml
```

Inputs:

| Input | Meaning |
|---|---|
| `artifact_id` | Stable registry id, for example `gh`, `ruff`, or `supabase`. |
| `target` | Executable name or path to measure. |
| `install_command` | Optional trusted setup command that installs the target CLI. |
| `profile` | `quick`, `standard`, or `deep`. |
| `max_depth` | Optional traversal depth override. |
| `max_probes` | Optional probe budget override. |

Current workflow behavior:

1. Builds CLIARE.
2. Optionally runs a trusted install command for the target CLI.
3. Runs:

```sh
cliare measure "$TARGET" --out ".cliare-index-runs/$ARTIFACT_ID" --profile "$PROFILE" --refresh
```

4. Runs:

```sh
cliare describe ".cliare-index-runs/$ARTIFACT_ID" --write
```

5. Copies selected review artifacts into:

```text
registry/<artifact_id>/
```

6. Opens or updates a pull request with the registry entry.

Current registry files:

| File | Meaning |
|---|---|
| `README.md` | Entry summary generated by the workflow. |
| `command-index.json` | Agent-facing command catalog. |
| `command-index.md` | Human-readable command catalog. |
| `scorecard.json` | Experimental scorecard for the measured run. |
| `summary.md` | Short CI/run summary. |
| `issues.md` | Reviewable findings. |
| `artifact-map.md` | Artifact navigation map. |

This is not a hosted leaderboard. It is a repository-native way to review and publish command indexes through pull requests.

---

## What “Publishing” Means Today

Current publishing options are local and repository-based:

- upload the `.cliare/...` artifact directory from CI
- append `summary.md` to the GitHub job summary
- upload `findings.sarif` with a separate SARIF step
- upload `junit.xml` to a test-reporting surface
- commit or PR selected command-index artifacts into `registry/<artifact_id>/`
- attach artifacts to a release using standard GitHub workflow steps

There is no current CLIARE-hosted ingestion API or badge endpoint.

---

## Data Retention And Privacy

Current CLIARE artifacts can contain command output snippets and filesystem side-effect paths. Treat artifact upload as a project policy decision.

Conservative public artifact set:

- `command-index.json`
- `command-index.md`
- `scorecard.json`
- `summary.md`
- `issues.md`
- `artifact-map.md`

More sensitive artifacts:

- `evidence.jsonl`
- `shape.json`
- `issues.json`
- persona JSON packets
- raw stdout/stderr snippets embedded in process evidence

Before making artifacts public, review whether the measured CLI or context can expose:

- organization names
- repository paths
- usernames
- project ids
- local filesystem paths
- token-like or credential-like paths
- remote resource names

The current registry workflow copies a conservative subset and does not copy `evidence.jsonl`.

---

## Public Badge Semantics

There is no current CLIARE badge endpoint.

If a project wants to advertise current artifacts manually, the language should avoid certified-score claims:

Good:

```text
Evidence-backed command index
```

Good:

```text
CLIARE measured in CI
```

Acceptable with explicit model status:

```text
CLIARE score 84 | experimental_partial | cliare-score-v0
```

Avoid:

```text
Agent-safe
```

Avoid:

```text
Certified CLIARE score
```

---

## Future Hosted Publishing Direction

A hosted publishing layer can be valuable, but it is not part of the current implementation.

Potential hosted capabilities:

- scorecard hosting
- command-index hosting
- historical drift views
- private team dashboards
- policy dashboards
- cross-repo comparison
- release-to-release score history
- badge endpoints
- public catalog pages
- calibrated leaderboards

The hosted layer should not be required for normal OSS adoption. Maintainers should be able to measure, inspect, disposition, guard, and publish command indexes from their own repository.

---

## Future Attestation Direction

Future hosted publishing should prefer CI provenance over hosted execution of arbitrary user binaries.

Useful GitHub provenance fields:

- repository
- commit SHA
- workflow ref
- run id
- tag
- actor
- CLIARE version
- score model id and hash
- artifact hashes

Verification should distinguish:

| Level | Meaning |
|---|---|
| local | Generated outside a recognized CI provenance path. |
| ci-artifact | Generated in CI and uploaded as workflow artifacts. |
| ci-attested | Generated in CI with verifiable repository/run provenance. |
| calibrated | Uses a calibrated score model and reproducible profile. |
| certified | Meets future leaderboard governance requirements. |

These levels are future hosted semantics. Current local artifacts contain useful provenance fields, but CLIARE does not yet issue hosted verification labels.

---

## Future Leaderboard Requirements

Leaderboard ranking should wait until the score model and profiles are mature enough to compare projects fairly.

An authoritative public leaderboard needs:

- calibrated score model
- published model hash
- train/validation/holdout discipline
- proper scoring metrics
- reproducible traversal profiles
- repeated-run stability thresholds
- clear runtime context labels
- provenance and artifact-hash checks
- anti-gaming controls
- policy for stale or old-model scorecards

Leaderboard views can eventually include:

- by CLI category
- by ecosystem
- most improved
- highest output-contract readiness
- highest safety posture
- verified-only lanes

Until then, public pages should emphasize evidence-backed command catalogs, not rank ordering.

---

## Anti-Gaming Direction

Future public publishing should disclose:

- score model id and hash
- model status
- traversal profile
- runtime context
- execution mode
- run date
- target binary fingerprint
- artifact availability
- verification level

Red flags for future hosted review:

- scorecard without a model hash
- scorecard without target fingerprint
- old score model
- missing runtime context
- probe budget too small for the surface
- traversal incomplete but presented as definitive
- public badge that hides `experimental_partial`

Current local reports already expose many of these fields. Hosted publishing should preserve them rather than compressing everything into a bare number.

---

## Initial CI Scope

The initial CI integration that exists today includes:

- composite GitHub Action
- `measure` mode
- `guard` mode
- artifact upload
- GitHub step summary
- `summary.md`
- `scorecard.json`
- `findings.sarif`
- `junit.xml`
- command index artifacts
- issue ledger artifacts
- manual command-index PR workflow

Future work includes hosted publishing, badges, certified profiles, and calibrated leaderboard ranking.
