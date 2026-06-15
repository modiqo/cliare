# 26 - Maintainer Issue Dispositions

> **Scope:** Review workflow for maintainers who agree, disagree, defer, or intentionally back off from CLIARE issues.
> **Status:** Product and Implementation Design

---

## Purpose

CLIARE issues are evidence-backed findings, not automatic mandates. A maintainer should be able to say that a finding is real, intentional, not applicable, accepted risk, a false positive, or blocked on fixtures without deleting the observation that produced it.

The disposition workflow turns the generated issue ledger into a review ledger:

```text
issues.json + maintainer review -> issue-dispositions.json -> reports and harness guidance
```

This prevents two failure modes:

- CLIARE keeps repeating advisory findings that the project has intentionally rejected.
- Maintainers suppress findings and lose the evidence trail that agents and CI need.

---

## Design Principle

Dispositions do not rewrite evidence. They annotate it.

An issue remains evidence-backed and reproducible. The disposition records human judgment about how the project wants to treat that issue. Reports should show both:

- CLIARE observation: what was measured or inferred
- Maintainer disposition: what the project decided to do with it

---

## Disposition Statuses

| Status | Meaning | Report Behavior |
|---|---|---|
| `open` | No maintainer decision has been recorded. | Keep in the action queue. |
| `accepted` | Maintainer agrees this is work to do. | Keep in the action queue with the maintainer note. |
| `intentional` | Behavior is deliberate and should not be treated as a defect. | Move to reviewed decisions; keep harness guidance. |
| `not_applicable` | The finding does not apply to this CLI or product domain. | Move to reviewed decisions; do not present as required work. |
| `false_positive` | CLIARE inference is wrong. | Move to reviewed decisions and use as CLIARE improvement feedback. |
| `accepted_risk` | The issue is real, but the project accepts the risk. | Move to reviewed decisions; preserve routing caution. |
| `needs_fixture` | The project cannot judge or fix the finding until safe operands or fixture context exist. | Keep as fixture work, not an implementation defect. |
| `deferred` | The finding is valid but not current priority. | Move to reviewed decisions unless policy asks for open debt. |

---

## Artifact Contract

Maintainer decisions are stored beside the measurement artifacts:

```text
issue-dispositions.json
```

Schema:

```json
{
  "schema_version": "cliare.issue-dispositions.v1",
  "dispositions": [
    {
      "issue_id": "issue.alternate_help_form_unavailable",
      "status": "intentional",
      "reason": "Direct <command> --help is the supported help contract; help <path> is intentionally unsupported."
    }
  ]
}
```

The file is small, reviewable, and suitable for committing into CI artifacts or copying between repeated runs of the same CLI version. If a future run no longer emits the issue, the disposition is harmless historical context. If a future run emits the same issue ID, the report can apply the disposition again.

---

## CLI Workflow

Record a maintainer decision:

```sh
cliare issues mark issue.alternate_help_form_unavailable \
  --out .cliare \
  --status intentional \
  --reason "Direct <command> --help is canonical; help <path> compatibility is not part of this CLI."
```

The CLI accepts kebab-case status values such as `needs-fixture` and `false-positive`; `issue-dispositions.json` stores the same statuses as snake_case values such as `needs_fixture` and `false_positive`.

List current issues with dispositions:

```sh
cliare issues list --out .cliare --format markdown
```

Use a context-specific artifact directory when the measurement came from a context suite:

```sh
cliare issues mark issue.output_mode_unprobed \
  --out .cliare-context \
  --context authenticated \
  --status needs-fixture \
  --reason "Requires a local MCP fixture endpoint and safe adapter id."
```

---

## Report Semantics

Persona reports should split work into two mental buckets:

```text
Action required
  open, accepted, needs_fixture

Reviewed decisions
  intentional, not_applicable, false_positive, accepted_risk, deferred
```

The report should still show evidence and affected commands for dispositioned issues when requested. The distinction is not whether the issue exists; the distinction is whether it should remain a maintainer action item.

---

## Harness Semantics

Agent harnesses should treat dispositions as routing policy hints:

| Disposition | Harness Interpretation |
|---|---|
| `intentional` | Do not penalize the CLI for the project-specific design choice; follow the recorded reason. |
| `not_applicable` | Ignore this issue for routing unless the harness has domain-specific evidence otherwise. |
| `false_positive` | Do not route around this issue, but preserve the evidence for CLIARE model improvement. |
| `accepted_risk` | The issue is real; route only if the harness can tolerate the recorded risk. |
| `needs_fixture` | Do not rely on the affected contract until the harness owns a fixture or safe operand set. |
| `deferred` | Treat as known debt, not an immediate blocker unless local policy says otherwise. |

---

## Why This Matters

The credibility of CLIARE depends on maintainers being able to push back precisely. A tool that treats every finding as a defect will create noise. A tool that lets findings disappear will lose auditability. Dispositions give maintainers a middle path: disagree, defer, or accept risk while keeping the runtime evidence visible.
