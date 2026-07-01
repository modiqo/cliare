# Crate Boundaries

> **Scope:** Internal Rust workspace boundaries for the CLIARE implementation.
> **Status:** Current workspace layout after crate extraction and large-file split.

---

## Purpose

CLIARE remains a CLI-first product. The stable external contract is still the
`cliare` binary, command-line behavior, emitted artifacts, schemas, and reports.

The Rust crates introduced during the refactor are internal implementation
boundaries. They are intended to reduce review complexity, make ownership
clearer, and prepare for further extraction without changing user-visible CLI
behavior.

## Current Workspace Slice

The current workspace slice keeps the package named `cliare` at the repository
root as the binary/lib compatibility surface and moves implementation into
workspace crates:

| Crate | Responsibility |
|---|---|
| `cliare-app` | Compatibility facade re-exporting semantic crates under historical module paths. |
| `cliare-cli` | CLI argument structs, traversal/profile knobs, command metadata, and CLI surface tests. |
| `cliare-context` | Runtime context models, context-suite artifacts, and context comparison. |
| `cliare-core` | Shared artifact names, atomic artifact writing, probe intent/status enums, and typed CLIARE errors. |
| `cliare-runtime` | Target fingerprinting, sandbox runtime support, bounded process execution, and output capture. |
| `cliare-evidence` | Shared process-completed evidence records used by evidence writing, claims, scoring, and shape inference. |
| `cliare-guidance` | Operational playbooks and installable agent review skills. |
| `cliare-inference` | Help layout parsing, output classification, diagnostic/precondition classification, belief updates, and bundled score-model loading. |
| `cliare-inspect` | Artifact inspection and describe command behavior. |
| `cliare-issues` | Issue disposition/review model. |
| `cliare-measure` | Measurement traversal, planner, jobs, benchmark, guard, CI artifacts, and evidence writing. |
| `cliare-shape` | Shape observations, claim inference, `shape.json`, and command-index generation. |
| `cliare-policy` | Policy-file evaluation and side-effect path classification. |
| `cliare-score` | Scorecard metrics, findings, score reports, and score artifact writing. |
| `cliare-report` | Full persona reports, issue ledgers, surface queries, artifact guides, Markdown/report formatting, and report evidence models. |
| `cliare` | Root binary entry point and compatibility re-exports for historical `cliare::...` module paths. |

The root crate now has only `src/lib.rs` and `src/main.rs`. `src/lib.rs`
re-exports moved modules from the workspace crates so existing call sites keep
compiling under historical paths such as `cliare::measure`, `cliare::report`,
`cliare::shape`, `cliare::score`, and `cliare::sandbox`. Internal helper modules
now live inside their owning crates instead of root `src/`.

## Dependency Direction

Dependencies should remain one-way:

```text
cliare-core
  -> cliare-runtime / cliare-context / cliare-evidence
  -> cliare-inference
  -> cliare-shape / cliare-policy
  -> cliare-score
  -> cliare-report / cliare-measure / cliare-guidance / cliare-inspect
  -> cliare-cli
  -> cliare-app
  -> cliare
```

Lower-level crates must not depend on Clap argument structs, terminal rendering,
or command dispatch. The CLI layer should translate parsed arguments into typed
domain configs before calling lower-level crates.

## Large-File Rule

The refactor should reduce file size and cognitive load, not only move large
files between crates.

Target limits:

- Prefer implementation files under 500 lines.
- Treat files above 800 lines as refactor debt unless there is a written reason.
- Split by responsibility: model types, loading, execution, rendering, tests,
  and artifact writing should live in separate modules when they grow.

Completed splits include:

- `cliare-runtime`: sandbox snapshot/diff logic and tests moved under
  `sandbox/`, with process execution in `process.rs`.
- `cliare-inference`: layout parsing split into command extraction, document
  handling, flags, sections, output modes, typed models, and tests; diagnostic
  classification split into analysis, recovery parsing, token features, model,
  and tests.
- `cliare-shape`: claim inference and shape artifact generation split into
  focused modules for command claims, flags, output contracts, indexes, gaps,
  Markdown, models, writers, and tests.
- `cliare-score`: scorecard computation split into artifacts, calculator,
  findings, formulas, labels, metrics, model, report rendering, utilities, and
  tests.
- `cliare-report`: persona reports, Markdown reports, issue disposition views,
  and surface queries split into focused model, rendering, packet, matching,
  action, and test modules.
- `cliare-measure`: benchmark, jobs, measure traversal/checkpoint/progress, and
  planner code split into focused modules under their owning domains.
- `cliare-cli`: CLI structs split by command family; command metadata split
  into model, Clap extraction, text rendering, and tests.
- `cliare-context`, `cliare-guidance`, and `cliare-inspect`: context runtime,
  playbook, and describe flows split into smaller modules with sibling tests.

After the split, every Rust source file under `crates/*/src/` is at or below
590 lines. The root `src/` directory contains only the binary entry point and
compatibility re-export facade.

## Remaining Refinement Targets

The current layout is intentionally conservative. Further extraction should
only happen when it removes a real dependency edge or makes review materially
easier. Good candidates are:

1. Move CI artifact generation toward `cliare-score` once report dependencies
   are lowered enough to keep policy and report code independent.
2. Convert command crates to domain config inputs instead of depending directly
   on Clap argument structs.
3. Keep high-churn test modules split by behavior when they approach the same
   500-line review target.
