# CLIARE

CLIARE is an open-source standard and Rust reference implementation for measuring how ready a command-line interface is for agents and automation.

It treats a CLI as a black-box runtime system, exercises it in a controlled sandbox, records evidence, infers command shape, and produces a reproducible agent-readiness scorecard for CI, badges, and long-term improvement tracking.

CLIARE stands for **CLI for Agent Readiness**.

## Status

This repository is private while the project is being shaped. The initial commit contains the Rust project scaffold and the full design packet under [`docs/`](docs/00-index.md).

## Goals

- Infer command trees, flags, arguments, output contracts, and safety properties from runtime evidence.
- Score CLI readiness across discovery, grammar, execution, output, safety, and recovery.
- Run locally in CI without uploading binaries to a hosted service.
- Emit portable artifacts: evidence logs, command-shape catalogs, scorecards, and reports.
- Provide a public standard that CLI maintainers can use to improve agent operability.

## CLI

```sh
cliare measure ./mycli
cliare guard ./mycli --baseline .cliare/baseline.scorecard.json
cliare certify ./mycli
cliare rescore .cliare/evidence.jsonl
```

The implemented `measure` command fingerprints a target binary, runs bounded safe probes, records `evidence.jsonl`, emits a generic `shape.json`, and writes experimental `scorecard.json` and `report.md` artifacts over currently measured dimensions. The implemented `guard` command measures a target and fails on total-score regression against a baseline scorecard. Default probing now allows command paths up to depth 5 with a 256-probe budget; tune with `--max-depth` and `--max-probes` for larger or tighter CI runs. Scorecards also report coverage pressure, including observed depth, frontier remaining, candidates skipped by depth, and probes skipped by budget. Other commands remain planned.

## Design Packet

Start here:

- [Design index](docs/00-index.md)
- [Mathematical model](docs/06-mathematical-model.md)
- [Rust runtime engineering](docs/13-rust-runtime-engineering.md)
- [Operational contracts](docs/14-operational-contracts.md)

## License

Apache-2.0. See [LICENSE](LICENSE).
