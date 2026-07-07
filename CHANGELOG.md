# Changelog

All notable changes to this project are documented here.

## [Unreleased]

### Added
- Added optional Nix flake support for running and installing cliare with Nix, plus a Devbox environment for reproducible development.

## [0.1.9] - 2026-07-02

Crates.io packaging reissue.

- Fixes the bundled score-model package layout so `cliare-inference` compiles from its crates.io tarball.
- Fixes the version-bump helper for workspace manifests and offline lockfile refresh.
- Reissues the release as `0.1.9` after the `0.1.8` crates.io workflow partially published internal workspace crates.

## [0.1.8] - 2026-07-02

Repository release rigor.

- Adds evidence-backed summary reports with assessment, meaning, command associations, evidence excerpts, and suggested remedies.
- Adds a condition dictionary artifact so report labels can be decoded in plain English.
- Adds `just justdev` as the local preflight gate for formatting, checking, linting, tests, package file-set checks, and quick CLIARE-on-CLIARE dogfooding.
- Adds CI that runs the local preflight gate and native workspace builds on Linux, macOS, and Windows.
- Extends binary release assets to include Windows x86_64.
- Hardens crates.io release automation for the multi-crate workspace with dependency-ordered publishing.
- Adds a version-bump script for coordinated workspace manifest, documentation, and lockfile updates.

## [0.1.6] - 2026-06-17

Report readability and workflow guidance.

- Adds plain-English report guidance for persona reports.
- Replaces internal issue-table columns with reader-facing meaning and action columns.
- Fixes invalid-probe recovery findings so precondition recovery does not trigger a contradictory invalid-probe warning.
- Adds an ordered `just` workflow cheatsheet and renames run-folder parameters away from ambiguous `id` wording.
- Updates generated CLIARE skill guidance to match the new persona report table shape.

## [0.1.5] - 2026-06-16

Operational hardening for measurement artifacts.

- Adds probe-level measurement checkpoints and compatible resume support.
- Adds cache manifest run IDs and per-artifact digests.
- Cleans abandoned in-progress evidence logs before fresh measurements.
- Makes snapshot scanner caps configurable through measurement and guard CLI options.
- Documents hostile-binary containment as external operational policy and records that current isolated measurements are not a hostile-binary containment boundary.

## [0.1.4] - 2026-06-16

Documentation and release install clarity.

- Reorganizes the design packet into themed documentation folders with numberless file names.
- Moves the technical paper source and generated PDF under `docs/papers/`.
- Updates README install instructions for crates.io and GitHub Releases curl installation.
- Updates documentation links after the docs reorganization.

## [0.1.3] - 2026-06-15

Progress logging clarity.

- Adds the probe-budget percentage formula and a concrete `529 / 5000` example to measurement progress logs.

## [0.1.2] - 2026-06-15

Release packaging fix.

- Excludes generated storybook image assets from crates.io packages to keep uploads below registry limits.
- Moves the Intel macOS binary release build off the stalled `macos-13` runner label.

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
