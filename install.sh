#!/bin/sh
set -eu

REPO="${CLIARE_REPO:-modiqo/cliare}"
VERSION="${CLIARE_VERSION:-latest}"
INSTALL_DIR="${CLIARE_INSTALL_DIR:-${INSTALL_DIR:-$HOME/.local/bin}}"

say() {
  printf '%s\n' "$*"
}

fail() {
  say "cliare install: $*" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

detect_target() {
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Darwin)
      os_part="apple-darwin"
      ;;
    Linux)
      os_part="unknown-linux-gnu"
      ;;
    *)
      fail "unsupported operating system: $os"
      ;;
  esac

  case "$arch" in
    x86_64 | amd64)
      arch_part="x86_64"
      ;;
    arm64 | aarch64)
      arch_part="aarch64"
      ;;
    *)
      fail "unsupported architecture: $arch"
      ;;
  esac

  printf '%s-%s' "$arch_part" "$os_part"
}

download() {
  url="$1"
  output="$2"
  curl -fsSL --proto '=https' --tlsv1.2 "$url" -o "$output"
}

sha256_file() {
  file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
  else
    fail "required command not found: sha256sum or shasum"
  fi
}

verify_checksum() {
  archive="$1"
  sums="$2"
  archive_name="${archive##*/}"

  expected="$(awk -v name="$archive_name" '$2 == name {print $1}' "$sums")"
  [ -n "$expected" ] || fail "checksum not found for $archive"

  actual="$(sha256_file "$archive")"
  [ "$actual" = "$expected" ] || fail "checksum mismatch for $archive"
}

need curl
need tar
need awk
need uname

target="$(detect_target)"
archive="cliare-${target}.tar.gz"

if [ "$VERSION" = "latest" ]; then
  base_url="https://github.com/${REPO}/releases/latest/download"
else
  base_url="https://github.com/${REPO}/releases/download/${VERSION}"
fi

tmp_dir="$(mktemp -d 2>/dev/null || mktemp -d -t cliare-install)"
trap 'rm -rf "$tmp_dir"' EXIT HUP INT TERM

say "Installing cliare for ${target}"
say "Release: ${VERSION}"
say "Install dir: ${INSTALL_DIR}"

download "${base_url}/${archive}" "${tmp_dir}/${archive}"
download "${base_url}/SHA256SUMS" "${tmp_dir}/SHA256SUMS"
verify_checksum "${tmp_dir}/${archive}" "${tmp_dir}/SHA256SUMS"

mkdir -p "$INSTALL_DIR"
[ -d "$INSTALL_DIR" ] || fail "install directory does not exist: $INSTALL_DIR"
[ -w "$INSTALL_DIR" ] || fail "install directory is not writable: $INSTALL_DIR"

mkdir -p "${tmp_dir}/extract"
tar -xzf "${tmp_dir}/${archive}" -C "${tmp_dir}/extract"
[ -f "${tmp_dir}/extract/cliare" ] || fail "archive did not contain cliare binary"

chmod 0755 "${tmp_dir}/extract/cliare"
cp "${tmp_dir}/extract/cliare" "${INSTALL_DIR}/cliare"
chmod 0755 "${INSTALL_DIR}/cliare"

say "Installed ${INSTALL_DIR}/cliare"
say "Run: ${INSTALL_DIR}/cliare metadata --format text"

case ":$PATH:" in
  *":$INSTALL_DIR:"*)
    ;;
  *)
    say "Note: ${INSTALL_DIR} is not on PATH."
    ;;
esac
