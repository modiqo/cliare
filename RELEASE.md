# Release Process

This repository is prepared for binary and crates.io releases. The current crate version is `0.1.7`.

## Channels

- crates.io: publish the Rust crate so users can run `cargo install cliare`.
- GitHub Releases: publish prebuilt binaries, `install.sh`, and `SHA256SUMS` so users can install with `curl`.

## Binary Release Automation

The tag workflow `.github/workflows/release-binaries.yml` builds release archives for:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

It creates or updates the GitHub release for the tag and uploads:

- `cliare-<target>.tar.gz`
- `cliare-x86_64-pc-windows-msvc.zip`
- `install.sh`
- `SHA256SUMS`

User install command:

```sh
curl -fsSL https://github.com/modiqo/cliare/releases/latest/download/install.sh | sh
```

The installer supports:

```sh
CLIARE_INSTALL_DIR=/usr/local/bin
CLIARE_VERSION=v0.1.7
CLIARE_REPO=modiqo/cliare
```

The shell installer supports macOS and Linux. Windows users should download the
`cliare-x86_64-pc-windows-msvc.zip` release asset, verify it against
`SHA256SUMS`, and place `cliare.exe` on `PATH`.

## crates.io Automation

The tag workflow `.github/workflows/release-crates.yml` publishes all workspace crates to crates.io in dependency order when a `vX.Y.Z` tag is pushed and the tag version matches `Cargo.toml`.

Before tagging:

1. Revoke any token that was ever pasted into chat or logs.
2. Create a fresh crates.io API token.
3. Store it in GitHub repository secrets as `CARGO_REGISTRY_TOKEN`.
4. Run the preflight commands below locally.

## Preflight

Run from a clean working tree:

```sh
just justdev
```

`just justdev` runs formatting, type checking, clippy with warnings denied, the
workspace test suite, package file-set checks for every crate, and a quick
CLIARE-on-CLIARE measurement.

Verify the machine-readable command contract when touching CLI shape:

```sh
cargo run -- metadata --format json
```

Run a deeper CLIARE-on-CLIARE pass before tagging:

```sh
just cliare-on-cliare cliare standard
```

## Version And Tag

1. Run `scripts/bump-version.sh <new-version>`.
2. Update `CHANGELOG.md` with the final release date and notable changes.
3. Run `just justdev`.
4. Commit with a Conventional Commit message.
5. Tag the release:

```sh
git tag -a v0.1.7 -m "v0.1.7"
git push origin main
git push origin v0.1.7
```

## crates.io

Publishing is normally handled by `.github/workflows/release-crates.yml` after a version tag is pushed. To dry-run one package locally after its dependencies already exist on crates.io:

```sh
just publish-dry-run cliare
```

Post-publish install check:

```sh
cargo install cliare
cliare metadata --format text
```

## Homebrew

Homebrew distribution is deferred until a tap repository exists. The formula template remains at `packaging/homebrew/cliare.rb` for future use, but no Homebrew workflow is active.

## GitHub Release

The binary release workflow creates or updates the GitHub release for the pushed version tag. Confirm it includes:

- Release notes copied from `CHANGELOG.md`.
- The curl install command.
- The crates.io install command, when published.
- Checksums for attached archives and installer.
- A short example of `cliare measure <target> --out .cliare/<target> --profile standard --refresh`.

## Do Not Publish Yet If

- `cargo package` includes local artifacts, credentials, generated measurement outputs, or unrelated workspace files.
- `cliare metadata --format json` is not parseable.
- The CLIARE-on-CLIARE run has unreviewed action-required findings.
