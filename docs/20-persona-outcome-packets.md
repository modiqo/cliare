# 20 - Persona Outcome Packets

> **Scope:** Persona-specific report generation from one CLIARE measurement run.
> **Status:** Product and Implementation Design

---

## Purpose

CLIARE turns a measurement run into persona-specific action. A score is useful for comparison, but durable improvement comes from evidence, prioritization, remediation, and verification:

1. Run CLIARE against a target CLI.
2. Produce evidence, shape, command-index, score, coverage, and artifact-map artifacts.
3. Project those artifacts into the language of a specific user.
4. Give that user a precise packet of actions they can take.
5. Re-run CLIARE and verify that readiness improved.

Persona outcome packets are the report layer that makes this loop operational. They turn one measurement run into concrete runbooks for maintainers, harness builders, platform teams, security reviewers, open-source maintainers, ecosystem teams, and researchers.

Runtime probing remains separate from persona reporting. The expensive and potentially risky operation is `cliare measure`; persona reports are deterministic projections over the measured artifacts:

```text
.cliare/
  artifact-map.json
  artifact-map.md
  evidence.jsonl
  shape.json
  command-index.json
  command-index.md
  scorecard.json
  report.md
  summary.md
  findings.sarif
  junit.xml

        |
        v

cliare report <persona> --out .cliare

        |
        v

.cliare/
  persona-maintainer.json
  persona-maintainer.md
  persona-harness.json
  persona-harness.md
  ...
```

When a measurement is part of a runtime context suite, the packet is scoped to a single context artifact directory:

```text
.cliare/mycli/
  context-suite.json
  context-compare.md
  contexts/
    clean/
      scorecard.json
      command-index.json
      persona-maintainer.md
    workspace/
      scorecard.json
      command-index.json
      persona-maintainer.md
```

In that layout, `cliare report <persona> --out .cliare/mycli --context workspace` and `cliare report <persona> --out .cliare/mycli/contexts/workspace` address the same artifact bundle. If the suite root contains multiple persisted contexts and no context is selected, the command lists the available context names and directories rather than guessing.

This separation matters. It means different teams can consume the same evidence without rerunning the target binary. It also means a public scorecard can be audited against the packet that produced its recommendations.

---

## Persona Enum

The reference implementation should expose a closed, versioned persona enum. The enum is intentionally product-facing rather than organization-specific.

```rust
pub enum Persona {
    Maintainer,
    Harness,
    Platform,
    Security,
    Oss,
    Devrel,
    Research,
}
```

Each persona maps to a different outcome packet view:

| Persona | Primary Question | Output Bias |
|---|---|---|
| `maintainer` | What should we fix in the CLI? | Improvement list, missing contracts, command health, score deltas |
| `harness` | Which subset can agents safely use? | Safe command subset, confidence, schemas, blocked commands |
| `platform` | Can this CLI pass internal automation policy? | Guardrails, thresholds, regression status, CI gate instructions |
| `security` | What did the binary attempt during probing? | Side effects, auth gates, credential-like paths, evidence trail |
| `oss` | How do we publish a credible public readiness signal? | Badge readiness, public scorecard, CI setup, release notes |
| `devrel` | What evidence supports an external readiness story? | Trend narrative, competitive posture, claims safe to make |
| `research` | Can this run be reused for evaluation and calibration? | Corpus fields, replayability, model version, uncertainty, artifacts |

The enum should be part of the public CLI surface:

```sh
cliare report maintainer --out .cliare
cliare report harness --out .cliare --format json
cliare report security --out .cliare --write
```

The enum should also appear inside every generated packet so downstream tools can route the packet without relying on the filename.

---

## Outcome Packet Contract

Persona packets are structured artifacts. Markdown is for humans; JSON is the contract.

The packet schema should be versioned independently from the score model:

```json
{
  "schema_version": "cliare.persona-outcome.v1",
  "persona": "maintainer",
  "target": {
    "requested": "./target/debug/cliare",
    "resolved": "/repo/target/debug/cliare",
    "binary_sha256": "..."
  },
  "source_artifacts": {
    "artifact_dir": ".cliare",
    "artifact_map": ".cliare/artifact-map.json",
    "evidence": ".cliare/evidence.jsonl",
    "shape": ".cliare/shape.json",
    "command_index": ".cliare/command-index.json",
    "command_index_markdown": ".cliare/command-index.md",
    "scorecard": ".cliare/scorecard.json"
  },
  "summary": {
    "score": 97.4,
    "score_model": "cliare-score-v0",
    "status": "experimental_partial",
    "commands_discovered": 5,
    "commands_runtime_confirmed": 5,
    "commands_precondition_blocked": 0,
    "shape_gaps": 4,
    "findings": 0,
    "traversal_complete": true,
    "budget_exhausted": false
  },
  "run_recommendations": [],
  "action_items": [],
  "command_health": [],
  "score": {},
  "coverage": {},
  "notes": []
}
```

The JSON packet should be complete enough for automation. The Markdown packet should be concise enough to paste into a pull request, release checklist, internal review, or security exception ticket.

Markdown persona packets should be table-first. A reviewer should see score context, persona focus, priority findings, command readiness, recommended runs, and working files as scan-friendly tables before opening detail. Issue drill-down belongs behind explicit sections for a selected priority row so the report remains a review surface rather than an information dump.

---

## Packet Sections

Every persona packet should contain the same top-level sections. Persona-specific logic changes priority, wording, and recommended actions, not the underlying data model.

### 1. Identity

The packet must identify:

- target requested by the user
- resolved binary path
- binary SHA-256
- CLIARE version when available
- score model
- shape model
- packet schema version
- artifact directory
- generated timestamp when added later

The target fingerprint is essential. A packet without a binary identity is not actionable evidence.

### 2. Executive Summary

The summary should be short and stable:

- total score
- score status
- measured score model
- high, medium, and low finding counts
- command discovery counts
- runtime-confirmed commands
- precondition-blocked commands
- output contract counts
- machine-readable contract counts
- side-effect counts
- traversal completion state
- budget exhaustion state

This is the section used by CI logs, pull request comments, and release dashboards.

### 3. Run Recommendations

The packet should recommend the next command to run when the current evidence is incomplete or when the persona naturally needs a stricter mode.

Examples:

```sh
cliare measure ./mycli --profile deep --max-depth 8 --max-probes 1000 --concurrency 8 --refresh
cliare guard ./mycli --baseline .cliare/baseline.scorecard.json --policy cliare.policy.json
cliare report security --out .cliare --format markdown
```

Recommendations should be evidence-driven:

| Evidence | Recommendation |
|---|---|
| `traversal_complete = false` | Increase probe budget or use `--profile deep`. |
| `observed_max_depth == max_depth` | Increase `--max-depth`. |
| `budget_exhausted = true` | Increase `--max-probes` or reduce expected-value threshold. |
| many precondition-blocked commands | Provide fixtures or expose unauthenticated help. |
| no machine-readable output contracts | Add JSON/YAML modes and rerun standard profile. |
| side effects during safe probes | Run security report and review write paths. |
| guard policy failure | Fix policy failures before publishing a score. |

Run recommendations are not generic tips. They are a control surface for improving the next CLIARE run.

### 4. Action Items

Action items are the packet's main product value. Each action item should include:

- stable ID
- severity
- category
- title
- detail
- recommendation
- affected command paths
- supporting evidence references
- related score dimension
- persona priority

The system should prefer fewer, sharper items over a long list of low-value advice. For CI, action items must be deterministic so diffing packets across releases remains meaningful.

### 5. Command Health

Command health is the bridge between a score and a fix.

For every discovered command, the packet should be able to show:

- command ID
- path
- argv form
- summary if observed
- confidence
- runtime state
- preconditions
- whether help was available
- whether usage syntax was observed
- discovered flags
- output contracts
- gaps attached to the command
- evidence references

This is where maintainers see exactly which commands need work. It is also where harness builders decide which commands are safe enough to expose.

### 6. Score and Coverage

The score section should reproduce the scorecard's total score, model, status, subscores, rationales, and coverage fields. Persona packets should not recompute score independently. They should consume the scorecard as source evidence.

### 7. Evidence Trail

The packet should not duplicate raw stdout and stderr. It should reference evidence IDs and artifact paths. Deep inspection belongs in `evidence.jsonl`; persona packets provide an index.

---

## Persona-Specific Packet Requirements

### Maintainer Packet

The maintainer packet is the default improvement report. It should answer:

- What lowers the score?
- Which commands are weakly confirmed?
- Which commands have missing usage or flag grammar?
- Which commands need machine-readable output?
- Which safe probes created side effects?
- What should change before the next release?

Required action types:

- add machine-readable output to read/list/show commands
- expose help without auth where feasible
- improve unknown command and unknown flag diagnostics
- add or clarify usage syntax
- make help/version/diagnostic paths read-only
- increase traversal budget when the surface was not fully explored

The maintainer Markdown packet should be suitable for an engineering issue or release-readiness checklist.

### Harness Packet

The harness packet is for agent platform builders. It should answer:

- Which commands have enough confidence to expose?
- Which commands should be excluded?
- Which output contracts are parseable?
- Which commands require auth, local context, profile, or another runtime precondition?
- Which invocations have known grammar and stable diagnostics?

Required action types:

- safe candidate commands
- commands requiring preconditions
- commands with insufficient confidence
- commands lacking parseable machine output
- commands with side effects during safe probes

The harness JSON packet should be intentionally machine-friendly. It should eventually support conversion into tool definitions, router indexes, adapter scaffolds, and agent skills.

### Platform Packet

The platform packet is for internal quality gates. It should answer:

- Does the CLI meet minimum readiness thresholds?
- Which dimensions are below policy?
- Did the score regress against baseline?
- Which findings should block release?
- What CI command should be installed?

Required action types:

- total score below threshold
- subscore below threshold
- guard regression
- unapproved side effects
- missing dry-run or safety evidence when that dimension is implemented
- traversal incomplete under required profile

The platform packet should include recommended CI snippets and policy references once policy profiles mature.

### Security Packet

The security packet is an evidence review. It should answer:

- What files changed during safe probes?
- Did any path look credential-related?
- Which probes were auth-gated, local-context-gated, or otherwise precondition-gated?
- Which command paths should be excluded from agent exposure?
- Is the evidence sufficient for approval, exception, or denial?

Required action types:

- persistent side effects
- credential-like side effects
- auth-required and local-context-required preconditions
- command paths with unknown side-effect profile
- incomplete traversal under security review

The security packet should be conservative. It should not claim absence of risk from absence of evidence.

### OSS Packet

The OSS packet is for public maintainers. It should answer:

- Is the scorecard publishable?
- What README badge or release note is appropriate?
- What should be fixed before announcing readiness?
- How can contributors improve the score?

Required action types:

- publishable score summary
- badge readiness
- public artifact checklist
- top contributor-friendly improvements
- warning when the run is provisional or incomplete

The OSS packet should avoid overclaiming. It should distinguish local CI scores from certified public leaderboard entries.

### DevRel Packet

The DevRel packet is for external narrative and ecosystem positioning. It should answer:

- What claims are supported by evidence?
- What claims are not yet supported?
- Which improvements would produce the most credible public story?
- How does the current score trend across releases?

Required action types:

- evidence-backed public claims
- claims to avoid
- release-note summary
- competitor/leaderboard readiness once calibrated
- improvement themes by score dimension

The DevRel packet must be grounded. It should never turn a provisional score into a certified benchmark claim.

### Research Packet

The research packet is for benchmark and calibration users. It should answer:

- Can this run be replayed?
- Which model versions produced the artifacts?
- Which portions of the surface are incomplete?
- What budget limits affected the run?
- Which evidence can be used for truth-set labeling?

Required action types:

- replayability checklist
- evidence completeness warning
- corpus suitability
- calibration gaps
- profile and budget metadata

The research packet should make limitations easy to cite.

---

## Run Depth and Surface Coverage

Large CLIs require explicit run strategy. A shallow run is useful for pull requests, but it should not be mistaken for whole-surface coverage.

CLIARE should treat depth and probe budget as first-class reporting fields:

- profile
- max depth
- observed max depth
- max probes
- probes completed
- frontier remaining
- highest pending expected value
- candidates skipped by depth
- candidates skipped by convergence
- probes skipped by budget
- traversal stop reason
- traversal complete

Persona packets should translate those fields into run advice.

Suggested guidance:

| Scenario | Guidance |
|---|---|
| Pull request smoke check | `--profile quick` |
| Release readiness | `--profile standard` |
| Large public CLI | `--profile deep --max-depth 8 --max-probes 1000` |
| Very large command tree | increase `--max-probes`, keep concurrency bounded, inspect frontier pressure |
| Known deep nesting | increase `--max-depth` |
| Incomplete traversal with high pending value | rerun with larger budget |
| Complete traversal with no findings | baseline can be promoted |

The packet should never hide incomplete traversal. Incomplete traversal is not failure by itself; it is uncertainty that must be visible.

---

## Continuous Improvement Loop

Persona packets should make improvement measurable. A team should be able to run:

```sh
cliare measure ./mycli --profile standard --out .cliare/current
cliare report maintainer --out .cliare/current --write
```

For context-sensitive CLIs, run the loop against an explicit context:

```sh
cliare measure ./mycli --profile standard --out .cliare/current --context clean
cliare measure ./mycli --profile deep --out .cliare/current --context workspace --context-workdir /path/to/project
cliare report maintainer --out .cliare/current --context workspace --write
```

For large surfaces or manual investigations, run the measurement as a detached job and inspect progress from another terminal or agent session:

```sh
cliare measure ./mycli --profile deep --out .cliare/current --detach
cliare jobs status --out .cliare/current
cliare measure ./mycli --profile deep --out .cliare/current --context workspace --context-workdir /path/to/project --detach
cliare jobs status --out .cliare/current --context workspace
```

Then after fixes:

```sh
cliare measure ./mycli --profile standard --out .cliare/next --refresh
cliare guard ./mycli --baseline .cliare/current/scorecard.json --out .cliare/next
cliare report maintainer --out .cliare/next --write
```

The packet should make improvement visible through:

- total score movement
- subscore movement
- finding count movement
- command confirmation movement
- machine-readable output coverage movement
- side-effect reduction
- precondition-blocked reduction
- traversal completion improvement

Baseline comparison lives primarily in `guard`, but persona packets should explain which packet fields are intended for trend dashboards.

---

## Implementation Direction

Measurement should generate the review packet set by default:

```sh
cliare measure ./mycli --out .cliare
```

At completion, `.cliare` should include `issues.json`, `issues.md`, and one Markdown plus JSON packet for each supported persona. This makes one measurement run directly usable by maintainers, harness builders, platform teams, security reviewers, OSS maintainers, DevRel teams, and researchers.

The report command remains the focused refresh and projection command:

```sh
cliare report <persona> --out .cliare
```

For a context suite, report generation must resolve to one measurement context:

```sh
cliare report <persona> --out .cliare/mycli --context workspace
cliare report <persona> --out .cliare/mycli/contexts/workspace
```

with these output options:

```sh
cliare report maintainer --out .cliare
cliare report maintainer --out .cliare --format json
cliare report maintainer --out .cliare --write
```

Behavior:

- read `.cliare/scorecard.json`
- read `.cliare/artifact-map.json` when present, or run `cliare describe .cliare --write`
- read `.cliare/shape.json`
- read `.cliare/evidence.jsonl`
- reference `.cliare/command-index.json` and `.cliare/command-index.md` as command-level drill-down artifacts
- when `--out` points at a context suite root, require a concrete `--context` unless only one persisted context exists
- build a typed `PersonaOutcomePacket`
- render Markdown or JSON
- when `--write` is passed, refresh `issues.json`, `issues.md`, and the selected `persona-<persona>.md` / `persona-<persona>.json`
- print the selected representation to stdout by default

The first implementation should focus on deterministic packet quality, not on adding new probes. It should use the existing artifacts and expose the information that is already present.

The packet builder should have this shape:

```rust
pub trait PacketBuilder {
    fn persona(&self) -> Persona;
    fn build(&self, artifacts: &MeasuredArtifacts) -> PersonaOutcomePacket;
}
```

If separate builders become unnecessary, this can remain an internal abstraction. The important part is that persona-specific logic is isolated from artifact loading and rendering.

---

## Quality Bar

Persona reporting becomes part of the public CLIARE contract. It should meet the same quality bar as scoring:

- deterministic output for the same artifacts
- stable schema version
- explicit unknown states
- no fabricated certainty
- no reliance on source code access
- no framework-specific assumptions
- no hidden network calls
- no rerun unless the user explicitly invokes measurement
- complete artifact provenance
- action items grounded in scorecard, shape, or evidence

This is how CLIARE becomes a natural CI choice across personas: one runtime measurement, many precise operational packets, all traceable to evidence.
