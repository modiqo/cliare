# 14 - Operational Contracts

> **Scope:** Cache reuse contract, adversarial target assumptions, dependency admission policy, score-model governance, dynamic CLI invalidation, and reproducibility levels.
> **Status:** Hardening Plan
> **Priority:** Post-core. This document is important before public certification and leaderboard operation, but it is not required to begin reference implementation work.

---

## Summary

The core CLIARE design is captured in the earlier documents. This reference records the operational contracts required before CLIARE becomes a public standard:

- when cached shape/evidence can be reused
- how to explain cache hits and misses
- how to treat dynamic CLIs
- how to reason about malicious or hostile target binaries
- how Rust dependencies are admitted
- how scoring model versions are governed
- how reproducibility levels are displayed

These topics sit after the main architecture, scoring, runtime, and CI design. They form the hardening layer for public certification.

---

## Rank and Criticality

| Item | Critical For Initial Release | Critical Before Public Leaderboard | Notes |
|------|------------------|------------------------------------|-------|
| Basic binary fingerprint | yes | yes | Already in core design |
| Evidence replay | yes | yes | Already in core design |
| Cache explain UX | no | yes | Useful for trust and debugging |
| Dynamic CLI invalidation policy | partial | yes | Needed for plugins/auth/remote state |
| Adversarial binary threat model | partial | yes | The initial release can start with a conservative sandbox |
| Dependency admission policy | no | yes | Needed before public trust |
| Score-model governance | no | yes | Required for standard credibility |
| Reproducibility levels | no | yes | Required for leaderboard interpretation |

---

## Cache Reuse Contract

CLIARE should avoid rerunning expensive probes when it can prove the previous run is still applicable. But cache reuse must be explainable.

### Artifact Layers

Do not treat "the cache" as one thing. Cache at four layers:

| Layer | Artifact | Expensive? | Invalidated By |
|-------|----------|------------|----------------|
| Probe evidence | `evidence.jsonl` | yes | target/runtime/profile changes |
| Shape inference | `shape.json` | medium | evidence or inference model changes |
| Scorecard | `scorecard.json` | cheap | score model, shape, evidence, policy changes |
| Report | `report.md` | cheap | report template, scorecard changes |

This enables:

```sh
cliare rescore .cliare/evidence.jsonl
cliare report .cliare/scorecard.json
```

without rerunning the target CLI.

### Cache Commands

Recommended UX:

```sh
cliare measure ./mycli
```

Default: reuse matching cached evidence if the full run fingerprint matches.

```sh
cliare measure ./mycli --refresh
```

Ignore reusable probe evidence and rerun probes.

```sh
cliare measure ./mycli --offline
```

Use existing cache only. Fail if there is no exact cache hit.

```sh
cliare cache explain ./mycli
```

Explain whether CLIARE would reuse or rerun and why.

```sh
cliare cache clean
```

Remove old cached artifacts.

### Cache Disclosure

Scorecard should disclose cache use:

```json
{
  "cache": {
    "used": true,
    "fingerprint": "sha256:...",
    "evidence_reused": true,
    "shape_recomputed": false,
    "score_recomputed": true,
    "reason": "probe fingerprint matched; score model changed"
  }
}
```

### Cache Hit Reasons

Examples:

```text
cache hit: binary hash, profile, sandbox policy, env allowlist, and plugin hash matched
```

```text
partial cache hit: evidence reused; score recomputed because score model changed
```

### Cache Miss Reasons

Examples:

```text
cache miss: binary hash changed
cache miss: probe profile changed from safe to certify
cache miss: plugin directory hash changed
cache miss: sandbox policy now denies network
cache miss: CLIARE version changed probe planner semantics
```

Cache miss reasons should be structured, not only prose.

---

## Run Fingerprint

The run fingerprint should include:

```json
{
  "target": {
    "binary_path": "./dist/mycli",
    "binary_sha256": "...",
    "reported_version": "1.2.3",
    "file_mode": "0755",
    "file_size": 18239120
  },
  "platform": {
    "os": "linux",
    "arch": "x86_64",
    "libc": "glibc"
  },
  "cliare": {
    "version": "0.1.0",
    "probe_planner": "cliare-probe-v1",
    "sandbox_policy": "sha256:...",
    "inference_model": "cliare-infer-v1"
  },
  "profile": {
    "name": "certify",
    "hash": "sha256:..."
  },
  "environment": {
    "allowlist_hash": "sha256:...",
    "fixed_env_hash": "sha256:..."
  },
  "dynamic_inputs": {
    "config_hash": "sha256:...",
    "plugin_hash": "sha256:...",
    "completion_hash": "sha256:...",
    "fixture_hash": "sha256:..."
  }
}
```

The score model version does not invalidate probe evidence. It invalidates only scorecards.

---

## Dynamic CLI Invalidation

CLIs are not always determined by the binary.

### `binary_only`

Shape depends primarily on binary content.

Cache policy:

```text
reuse aggressively when binary hash and profile match
```

### `binary_plus_config`

Config files affect available commands or defaults.

Cache policy:

```text
reuse only when declared config paths hash the same
```

### `plugin_dynamic`

Plugin directories or extension registries affect shape.

Cache policy:

```text
hash declared plugin dirs; warn if plugin dirs cannot be discovered
```

### `auth_dynamic`

Auth scope changes available commands or outputs.

Cache policy:

```text
reuse only under same declared auth fixture or auth scope hash
do not publish auth-profile scorecards as generally representative unless marked
```

### `remote_dynamic`

Remote services affect shape or behavior.

Cache policy:

```text
prefer network-denied certified profiles
mark network-allowed runs as remote-dynamic
expire or revalidate on schedule
```

---

## Adversarial Target Assumptions

CLIARE executes untrusted binaries. Before public certification, the design should explicitly assume the target may be hostile.

Potential behavior:

- read environment variables
- scan filesystem
- write outside intended paths
- create symlinks to escape diffing
- fork many subprocesses
- emit unbounded output
- hang forever
- attempt network exfiltration
- behave differently when it detects CLIARE
- pollute terminal output
- print secrets from inherited config
- exploit parser bugs in CLIARE output classifiers

Minimum mitigations:

- minimal environment
- temp HOME/cwd/XDG
- no shell interpolation
- process timeout
- output byte limits
- bounded channels
- cleanup after run
- redaction
- network denied in certified profile where backend supports it
- container/native sandbox for higher verification levels

Important distinction:

```text
Portable sandbox protects against accidents.
Container/native sandbox is needed for stronger hostile-binary isolation.
```

The leaderboard should disclose the sandbox class.

---

## Dependency Admission Policy

CLIARE should use mature Rust libraries, but every dependency should earn its place.

Admission criteria:

- active maintenance
- compatible license
- reasonable transitive dependency tree
- MSRV compatible with CLIARE policy
- no competing async runtime in core
- no unsafe-heavy crate without a clear reason
- no networked dependency in scoring path
- deterministic behavior where used in artifact generation
- mature error types or clean integration with CLIARE errors

Avoid by default:

- generic graph libraries in scheduler core
- actor frameworks
- distributed execution frameworks
- embedded databases for the initial release
- ML runtimes
- crates that force async runtime mixing

Every dependency added to core should answer:

```text
What invariant does this make easier to maintain?
What failure mode does it introduce?
Can we remove it later without artifact incompatibility?
```

---

## Score-Model Governance

CLIARE score versions need governance before public leaderboard operation.

Rules:

1. Scorecards always include score model version.
2. Leaderboard entries are grouped or normalized by score model.
3. Bug fixes that do not change score semantics can be patch releases.
4. Weight changes require a new minor or major score model.
5. New dimensions or changed public interpretation require a major score model.
6. Old scorecards remain valid under their original model.
7. Evidence can be rescored under newer models when available.

Example:

```text
cliare-score-v1.0: initial calibrated public model
cliare-score-v1.1: bug fix in confidence interval calculation, no ranking change expected
cliare-score-v2.0: new safety weighting and remote-dynamic penalty
```

Public pages should show:

```text
Score: 84
Model: cliare-score-v1
```

If a project has old scores:

```text
This score was computed with cliare-score-v0.experimental and is not comparable to v1 leaderboard entries.
```

---

## Reproducibility Levels

Add a reproducibility label separate from verification.

Verification answers:

```text
Where did this scorecard come from?
```

Reproducibility answers:

```text
How strongly can this run be reproduced?
```

Suggested levels:

| Level | Meaning |
|-------|---------|
| `replayable` | score can be reproduced from stored evidence |
| `fresh_probed` | target was freshly probed in this run |
| `cache_reused` | probe evidence was reused from matching fingerprint |
| `network_denied` | run used a network-denied profile |
| `container_isolated` | run used container isolation |
| `native_sandboxed` | run used OS-native isolation |
| `remote_dynamic` | result depends on remote behavior |
| `auth_scoped` | result depends on auth scope or fixture |

Scorecard example:

```json
{
  "reproducibility": {
    "probe_freshness": "cache_reused",
    "sandbox": "container_isolated",
    "network": "network_denied",
    "dynamic_scope": "binary_only"
  }
}
```

---

## Operational Findings

Operational findings should be distinct from CLI quality findings.

Example:

```text
Operational warning: score was computed from cached evidence.
```

```text
Operational warning: target appears plugin-dynamic, but plugin directories were not declared.
```

```text
Operational warning: portable sandbox cannot enforce network denial on this platform.
```

These should not always reduce CLI score, but they should affect verification and reproducibility labels.

---

## Implementation Timing

This document should be implemented after:

1. local evidence probing works
2. shape inference works
3. scorecard generation works
4. baseline guard works

Implement before:

1. public leaderboard
2. certified badge
3. enterprise/private scoreboards
4. v1 standard freeze
