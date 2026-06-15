# Release Process

This repository is prepared for an initial crates.io release. The current crate version is `0.1.1`.

## Channels

- crates.io: publish the Rust crate so users can run `cargo install cliare`.
- GitHub Releases: attach source archives, checksums, and optional prebuilt binaries after the tag is pushed.

## crates.io Automation

The tag workflow `.github/workflows/release-crates.yml` publishes to crates.io when a `vX.Y.Z` tag is pushed and the tag version matches `Cargo.toml`.

Before tagging:

1. Revoke any token that was ever pasted into chat or logs.
2. Create a fresh crates.io API token.
3. Store it in GitHub repository secrets as `CARGO_REGISTRY_TOKEN`.
4. Run the preflight commands below locally.

## Preflight

Run from a clean working tree:

```sh
cargo fmt --all -- --check
env RUSTC_WRAPPER= cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo package --list
cargo package
cargo publish --dry-run
```

Verify the machine-readable command contract:

```sh
cargo run -- metadata --format json
```

Measure CLIARE itself before tagging:

```sh
cargo run -- measure cliare --out .cliare/cliare --profile standard --refresh
cargo run -- issues list --out .cliare/cliare --format human
```

## Version And Tag

1. Update `Cargo.toml` and `Cargo.lock` if the version changes.
2. Update `CHANGELOG.md` with the final release date and notable changes.
3. Commit with a Conventional Commit message.
4. Tag the release:

```sh
git tag -a v0.1.1 -m "v0.1.1"
git push origin main
git push origin v0.1.1
```

## crates.io

Publish only after `cargo publish --dry-run` succeeds:

```sh
cargo publish
```

Post-publish install check:

```sh
cargo install cliare
cliare metadata --format text
```

## Homebrew

Homebrew distribution is deferred until a tap repository exists. The formula template remains at `packaging/homebrew/cliare.rb` for future use, but no Homebrew workflow is active.

## GitHub Release

Create a release for `v0.1.1` that includes:

- Release notes copied from `CHANGELOG.md`.
- The crates.io install command.
- Checksums for any attached archives or binaries.
- A short example of `cliare measure <target> --out .cliare/<target> --profile standard --refresh`.

## Do Not Publish Yet If

- `cargo package` includes local artifacts, credentials, generated measurement outputs, or unrelated workspace files.
- `cliare metadata --format json` is not parseable.
- The CLIARE-on-CLIARE run has unreviewed action-required findings.
