# CLIARE Agent Instructions

## Quality Bar

CLIARE should be written as a world-class Rust project: clear, boring where possible, explicit where necessary, and easy for experienced Rust engineers to audit. Favor small types, narrow traits, deterministic behavior, bounded resource use, and precise errors over clever abstractions.

Implementation should follow the design packet in `docs/`, especially:

- `docs/model/computational-scoring-model.md`
- `docs/architecture/checkpointing-and-resume.md`
- `docs/architecture/rust-runtime-engineering.md`
- `docs/architecture/operational-contracts.md`

## Rust Standards

- Prefer explicit domain newtypes for IDs, hashes, versions, scores, and artifact references.
- Keep traits narrow and owned by the crate that needs the abstraction.
- Use typed errors with `thiserror` in library code and polished diagnostics at CLI boundaries.
- Treat target CLI failures as evidence unless CLIARE itself failed.
- Do not use unbounded task spawning, unbounded channels, or unbounded output buffers.
- Do not introduce generic graph, actor, distributed execution, database, or ML dependencies without a measured need and a written rationale.
- Keep scoring deterministic for the same evidence and model version.
- Checkpoint and evidence-writing paths must be crash-conscious and replayable.

## Commit Hygiene

- Use Conventional Commits.
- Do not add commit coauthor trailers.
- Do not mention AI assistants in commit messages, code comments, docs, or generated metadata.
- Do not add attribution comments about who authored a line or why a reviewer requested it.
- No emojis in code, docs, commit messages, or user-facing output.

## Verification

Before pushing implementation changes, run:

```sh
cargo fmt --all -- --check
env RUSTC_WRAPPER= cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

If a command cannot run in the current environment, record the reason clearly before committing.
