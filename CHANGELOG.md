# Changelog

All notable changes to this project are documented here.

## [0.1.1] - 2026-06-15

Release-readiness update.

- Makes `cliare-score-v0` the single source of truth for claim priors, evidence weights, score weights, thresholds, and display precision.
- Adds `cliare issues list --format human` for a concise maintainer review queue.
- Documents model governance so scorecard hashes cover claim-confidence inference parameters.
- Adds crates.io release automation for version tags.

## [0.1.0] - 2026-06-15

Initial public release candidate.

- Measures released CLI binaries as black boxes and emits evidence-backed command indexes.
- Generates maintainer, harness, and security reports.
- Detects persistent filesystem side effects from safe discovery probes.
- Provides issue ledgers and dispositions for reviewed findings.
- Supports quick, standard, and deep measurement profiles.
- Exposes a parseable CLIARE command spec through `cliare metadata --format json`.
- Includes CI artifacts for scorecards, SARIF, JUnit, and GitHub Action summaries.
