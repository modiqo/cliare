# 14 - Operational Contracts

> **Scope:** Current cache, runtime context, guard, policy, sandbox, dependency, score-model, and reproducibility contracts.
> **Status:** Current implementation reference plus future hardening direction.

---

## Summary

This document defines the operational promises CLIARE can make today and the promises it should not make yet.

Current operational contracts:

- artifact-level measurement cache
- internal probe-level checkpoint/resume
- target fingerprinting
- runtime context declaration
- isolated and host execution modes
- configurable filesystem snapshot budgets
- bounded probing and output capture
- detached job tracking
- guard comparison against a saved baseline scorecard
- policy checks for score thresholds and filesystem side effects
- score-model versioning through bundled model JSON
- local artifacts suitable for CI, maintainer review, and agent harness review

Not current:

- public `cliare measure --resume`
- `cliare replay`
- `cliare rescore`
- `cliare cache explain`
- `cliare cache clean`
- `cliare measure --offline`
- `cliare certify`
- `cliare publish`
- public leaderboard verification
- portable network-deny enforcement
- container/native hostile-binary sandboxing

---

## Current Criticality

| Contract | Current status | Launch meaning |
|----------|----------------|----------------|
| Target fingerprint | Implemented | Cache and scorecards are tied to the measured binary |
| Artifact-level cache | Implemented | Completed measurements can be reused when exact identity and artifact digest checks pass |
| Internal probe-level checkpoint/resume | Implemented | Compatible interrupted measurements can resume without public checkpoint selection controls |
| Probe-level replay/rescore | Not implemented | Do not claim old evidence can be replayed or rescored through a command |
| Runtime context declaration | Implemented | Auth, fixture, local context, network, and dependency state are explicit metadata |
| Dynamic CLI invalidation | Partial | Context is captured, but plugin/config/remote-state hashes are not automatic cache keys |
| Sandbox for accidents | Implemented | Isolated mode reduces accidental local writes and captures side effects |
| Hostile-binary isolation | Not implemented | Do not market CLIARE as malware isolation |
| Guard baseline | Implemented | Existing scorecards can be used for local/CI regression checks |
| Policy file | Implemented | Score and side-effect thresholds can fail guard |
| Public certification | Not implemented | Public scores must remain bounded to local evidence |

---

## Cache Reuse Contract

Current cache file:

```text
<artifact-dir>/measure-cache.json
```

Schema:

```text
cliare.measure-cache.v1
```

Current cache identity requires:

- cache schema version matches
- CLIARE package version matches
- measurement engine label matches
- target fingerprint matches
- probe profile matches
- runtime context embedded in the profile matches
- required measurement artifacts exist
- required measurement artifact digests and sizes match the manifest

The cache manifest stores a run ID and per-artifact SHA-256 digests for the required artifact set. If any required artifact is missing or changed, the cache is treated as a miss.

Required artifacts for cache reuse:

| Artifact |
|----------|
| `evidence.jsonl` |
| `shape.json` |
| `command-index.json` |
| `command-index.md` |
| `scorecard.json` |
| `report.md` |
| `summary.md` |
| `findings.sarif` |
| `junit.xml` |

Current cache behavior:

- `cliare measure <target> --out <dir>` may reuse matching completed artifacts.
- `--refresh` bypasses cache and reruns probes.
- Cache hits regenerate lightweight derived files such as runtime context, persona reports, measurement guides, and context-suite metadata.
- Cache miss reasons are not yet emitted as a structured explanation artifact.

This is not current behavior:

```sh
cliare rescore .cliare/evidence.jsonl
cliare cache explain ./mycli
cliare cache clean
cliare measure ./mycli --offline
```

Those are future product directions only.

---

## Cache Disclosure

Current disclosure:

- terminal summaries print `cache: hit` or `cache: miss`
- progress logs record cache-miss context for fresh measurements
- `measure-cache.json` stores the target, profile, engine, CLIARE version, and measurement summary facts

Current limitation:

- `scorecard.json` does not currently contain a rich cache explanation object.
- CLIARE does not currently distinguish partial reuse layers such as reused evidence plus recomputed score.
- Cache reuse is all-or-nothing for completed measurement artifacts.

Future hardening should add structured cache-hit and cache-miss reasons, but docs and reports should not imply that exists today.

---

## Target Fingerprint

Current target fingerprinting records enough identity to tie artifacts to the binary CLIARE actually executed.

Current cache matching uses `TargetFingerprint`, which includes the requested target, resolved target, binary hash, and binary size. The implementation also resolves target paths before detached runs so missing binaries fail before the background worker is spawned.

Current limitations:

- Reported target version is observed separately through probes, not part of the cache fingerprint contract.
- Platform, libc, plugin directories, config hashes, completion output hashes, and remote state hashes are not currently part of the cache identity.
- Score-model changes invalidate the package/version/engine-level cache only when shipped through a new CLIARE version or engine identity; there is no separate rescore path today.

---

## Runtime Context Contract

Runtime context is how CLIARE records that a measurement was run under a specific environment assumption.

Current context profiles:

| Profile | Meaning |
|---------|---------|
| `single` | Default one-off measurement |
| `clean` | Declares auth/local/fixture state absent by default |
| `authenticated` | Declares auth state present by default |
| `local-context` | Declares local context present by default |
| `fixture` | Declares fixture state present by default |
| `custom` | Caller-defined context name and state |

Current context state dimensions:

- auth state
- local context state
- fixture state
- network state
- runtime dependency state
- cwd policy
- optional workdir

Current artifacts:

- `runtime-context.json`
- `context-suite.json`
- `context-compare.md`

Operational rule:

> A scorecard measured under `authenticated`, `host`, `fixture`, or `local-context` conditions should not be presented as a generic clean-environment score without saying so.

---

## Dynamic CLI Contract

Some CLIs change shape based on plugins, config, auth, local repositories, daemons, remote services, or runtime dependencies.

Current CLIARE support:

- records declared context state
- supports `--execution-mode isolated` and `--execution-mode host`
- supports `--context-*` flags for auth, local context, fixture, network, runtime dependency, and workdir
- reports precondition-blocked commands and probes
- can compare persisted contexts through `cliare context compare`

Current limitations:

- CLIARE does not automatically hash plugin directories.
- CLIARE does not automatically hash config inputs.
- CLIARE does not automatically detect all dynamic command sources.
- CLIARE does not automatically expire remote-dynamic measurements.

Operational rule:

> If a CLI's command surface depends on plugins, auth scope, remote services, or local state, measure and publish those as named contexts.

Example:

```sh
cliare measure mycli --out .cliare/mycli --context clean --refresh
cliare measure mycli --out .cliare/mycli --context authenticated --auth-state present --execution-mode host --refresh
cliare context compare .cliare/mycli/contexts/clean .cliare/mycli/contexts/authenticated --out .cliare/mycli --write
```

---

## Sandbox And Threat Model

CLIARE executes target binaries. The target should be treated as untrusted unless the operator already trusts it.

Current isolated mode mitigates accidents by:

- clearing the environment to a CLIARE-controlled allowlist
- using sandbox HOME, cwd, XDG, and temp directories
- giving each probe its own execution directories
- setting stdin to null
- enforcing per-probe timeout
- enforcing retained output limits
- capturing persistent file side effects inside configured sandbox regions
- enforcing configurable snapshot limits for maximum files, directories, and hashed bytes

Measurement artifacts record the active snapshot limits and an explicit `hostile_binary_containment` boolean. The current value is `false` because isolated mode is not a malware or adversarial-code sandbox.

Current host mode intentionally exposes host context:

- host home/config/cache/data/temp
- optional caller-provided workdir
- host auth/config/plugin state as visible to the target

Current limitations:

- no OS-level syscall tracing
- no portable network deny
- no container backend
- no process-tree isolation guarantee
- no prevention of a hostile binary reading files available to its process
- no guarantee that a hostile target cannot detect CLIARE probes

Operational rule:

> Isolated mode is a local safety and evidence boundary, not a hostile-binary containment boundary.

Use host mode only when measuring authenticated or local-context behavior is the point of the run.

---

## Guard And Policy Contract

`cliare guard` is implemented.

Current guard behavior:

```sh
cliare guard mycli \
  --baseline .cliare-baseline/mycli/scorecard.json \
  --out .cliare/mycli \
  --profile deep \
  --allowed-drop 2
```

It:

- runs a measurement using the same measurement pipeline as `cliare measure`
- reads a baseline scorecard
- compares current total score to baseline total score
- passes when the score drop is within `--allowed-drop`
- optionally evaluates a policy file
- rewrites CI artifacts with guard context

Current policy schema:

```text
cliare.policy.v1
```

Current policy supports:

- `min_total_score`
- `min_subscores`
- side-effect `allow_paths`
- side-effect `max_unapproved`
- side-effect `deny_credential_like`

Policy reads:

- `scorecard.json`
- `evidence.jsonl` process side-effect entries

Current limitations:

- There is no `cliare baseline accept` command.
- Baseline management is file-based.
- Guard compares total score directly; dimension-specific enforcement comes from policy thresholds.
- Policy is local to the CI/operator; there is no hosted policy registry.

---

## Dependency Admission Contract

The current dependency set is intentionally small. New dependencies should be admitted only when they preserve CLIARE's operational properties.

Current admission criteria:

- compatible license
- active maintenance
- MSRV compatible with the crate
- no competing async runtime in the runtime path
- deterministic behavior in artifact generation
- source errors can be preserved in `CliareError`
- no networked dependency in scoring or report generation
- no large transitive tree without a clear measured benefit

Current dependency set:

| Dependency | Role |
|------------|------|
| `clap` | CLI parser |
| `miette` | CLI diagnostic boundary |
| `serde`, `serde_json` | Artifact serialization |
| `sha2` | Binary and output hashes |
| `thiserror` | Typed errors |
| `time` | Timestamps |
| `tokio` | Async process/filesystem/timer runtime |

Avoid adding by default:

- generic graph libraries
- actor frameworks
- distributed execution frameworks
- embedded databases
- ML runtimes
- crates that force async runtime mixing
- unsafe-heavy crates without a written reason

Every dependency added to runtime or scoring should answer:

```text
What invariant does this make easier to maintain?
What failure mode does it introduce?
Can we remove it later without artifact incompatibility?
```

---

## Score-Model Governance

Current scoring implementation:

- loads bundled score-model JSON
- validates score-model schema
- records score model metadata in scorecards
- uses model-backed priors and evidence weights
- computes deterministic local scores for improvement and CI regression tracking

Current score use:

- maintainer feedback
- CI summary
- guard regression checks
- local benchmark expected-band checks
- provisional public transparency when clearly labeled

Not current:

- calibrated public certification
- leaderboard-grade comparability
- replay/rescore of old evidence under a new model

Operational rule:

> Public claims should say "local CLIARE scorecard" or "experimental score", not "certified ranking".

Future governance should define model-version compatibility, calibration reports, truth sets, and old-score handling before any leaderboard claim.

---

## Reproducibility And Verification

Current scorecards and summaries expose operational facts that help interpret a run:

- target fingerprint
- traversal profile
- max depth
- max probes
- min expected value
- concurrency limit
- traversal completion
- stop reason
- budget exhaustion
- runtime context
- sandbox profile
- output parse results
- side-effect counts
- precondition counts
- score model

Current limitations:

- no first-class `reproducibility` object in scorecard
- no verification-level labels
- no signed provenance
- no artifact hash manifest
- no replayable score contract
- no hosted validation

Future labels such as `fresh_probed`, `cache_reused`, `network_denied`, `container_isolated`, `remote_dynamic`, or `auth_scoped` should be added only when the implementation records enough evidence to justify them.

---

## Operational Findings

Current CLIARE emits findings and persona reports. Some findings are CLI-quality findings; others are operational interpretation issues.

Examples of current operational-style issues:

- traversal incomplete because probe budget or depth was exhausted
- output contract requires fixtures before validation
- authenticated or local context is required
- safe probes produced filesystem side effects
- credential-like side-effect paths were observed
- public publishing claims should stay provisional

Operational findings should guide trust and routing. They should not always be treated as CLI defects.

Maintainers can record dispositions with:

```sh
cliare issues mark <issue-id> \
  --out .cliare/mycli \
  --status intentional \
  --reason "Documented maintainer rationale."
```

---

## Implementation Timing

Implemented now:

- target fingerprinting
- artifact-level cache
- cache artifact digest manifest
- internal checkpoint/resume
- runtime contexts
- isolated and host execution profiles
- configurable snapshot limits
- guard baseline comparison
- local policy checks
- score-model metadata
- context comparison
- issue dispositions

Future hardening:

- structured cache explanation
- replay/rescore
- public checkpoint/resume controls
- baseline accept workflow
- dynamic plugin/config hashing
- reproducibility labels
- signed provenance
- network-denied/container/native sandbox verification levels
- public leaderboard governance

The future items should be converted into trackers only when they have concrete user demand, launch feedback, or a blocking quality requirement.
