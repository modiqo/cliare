#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage: scripts/bump-version.sh <new-version>

Updates the CLIARE workspace version, internal CLIARE dependency versions,
release docs, README install examples, and Cargo.lock.

Example:
  scripts/bump-version.sh 0.1.8
USAGE
}

if [[ $# -ne 1 ]]; then
  usage
  exit 2
fi

new_version="$1"
if [[ ! "$new_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]]; then
  echo "invalid semver version: $new_version" >&2
  exit 2
fi

if [[ ! -f Cargo.toml ]]; then
  echo "run from the repository root" >&2
  exit 2
fi

current_version="$(
  awk '
    /^\[workspace.package\]$/ { in_workspace_package = 1; next }
    /^\[/ { in_workspace_package = 0 }
    in_workspace_package && /^version = "/ {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' Cargo.toml
)"
if [[ -z "$current_version" ]]; then
  echo "could not find current workspace version in Cargo.toml" >&2
  exit 1
fi

if [[ "$current_version" == "$new_version" ]]; then
  echo "workspace is already at $new_version"
  exit 0
fi

echo "Bumping CLIARE from $current_version to $new_version"

while IFS= read -r -d '' manifest; do
  perl -0pi -e "s/version = \"\Q${current_version}\E\"/version = \"${new_version}\"/g" "$manifest"
done < <(find . -path './target' -prune -o -name Cargo.toml -print0)

OLD_VERSION="$current_version" NEW_VERSION="$new_version" perl -0pi -e '
  s/v\Q$ENV{OLD_VERSION}\E/v$ENV{NEW_VERSION}/g;
  s/current crate version is `\Q$ENV{OLD_VERSION}\E`/current crate version is `$ENV{NEW_VERSION}`/g;
' RELEASE.md README.md

cargo metadata --offline --format-version 1 > /dev/null

echo "Updated manifests and Cargo.lock to $new_version"
echo "Next: update CHANGELOG.md, run \`just justdev\`, commit, tag v$new_version, and push the tag."
