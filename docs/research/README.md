# Agent CLI Research Packet

> **Scope:** Research grounding, evaluation design, and scoring-model direction for CLIARE's two primary product use cases.
> **Status:** Research and design proposal.

---

## Product Thesis

CLIARE is an evidence-backed CLI shape layer plus a maintainer feedback system.
Its value is strongest when it serves two connected use cases:

1. **Help developers build agent-effective CLIs.**
   Maintainers need concrete, evidence-backed feedback about the CLI behaviors
   that make agent harnesses succeed or fail: discoverable commands, stable
   argument grammar, machine-readable outputs, actionable diagnostics,
   explicit preconditions, bounded side effects, and reviewable safety signals.

2. **Provide agent harnesses with a trusted shape file.**
   Harnesses should be able to open `shape.json` and `command-index.json`
   before executing a target CLI. Those artifacts should provide a baseline
   command surface, confidence scores, evidence references, output contracts,
   preconditions, and safety constraints so the harness does not have to learn
   everything by probing, reading stale documentation, or guessing flags.

The score should therefore not be only a generic quality grade. It should
answer two separate questions:

- **Maintainer readiness:** How much work remains before this CLI is easy and
  safe for agents to use?
- **Harness shape confidence:** How much can an agent rely on the emitted shape
  before performing additional verification?

---

## Document Map

| Document | Purpose |
|---|---|
| [Agent CLI Citations And Evidence](agent-cli-citations.md) | Primary-source research inventory for terminal agents, software-engineering agents, tool/API agents, and safety benchmarks, with implications for CLIARE. |
| [Agent CLI Evaluation Plan](agent-cli-evaluation-plan.md) | Proposed evaluation protocol for measuring whether CLIARE improves developer feedback and agent harness use of CLIs. |
| [Agent CLI Scoring Roadmap](agent-cli-scoring-roadmap.md) | Proposed scoring-model extensions that build on the current deterministic scorecard, shape, command index, and evidence log. |

---

## Relationship To Existing Docs

This packet does not replace the current model and architecture documents. It
builds on them:

- [Evidence And Command Shape Spec](../model/evidence-and-command-shape-spec.md)
  defines the current durable artifacts.
- [Generic Inference Processor](../model/generic-inference-processor.md)
  defines the current runtime evidence to shape pipeline.
- [Computational Scoring Model](../model/computational-scoring-model.md)
  defines the current v0 scorecard and calibration boundary.
- [Shape Quality Evaluation](../model/shape-quality-evaluation.md)
  defines the current fixture truth-set evaluator for `shape.json`.
- [QA, Benchmarking, And Calibration](../operations/qa-benchmarking-and-calibration.md)
  defines the current benchmark runner and the distinction between operational
  benchmark telemetry and future statistical calibration.

---

## Design Principle

The shape file should be useful even before a score is trusted as calibrated.

An agent harness can use high-confidence entries immediately, treat conditional
entries as requiring context or fixtures, and avoid low-confidence or
safety-sensitive entries until verification. A maintainer can use the same
evidence to improve the CLI and watch both readiness and shape confidence move
in the right direction.
