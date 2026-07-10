#!/usr/bin/env sh
# CI gate: package a release archive exactly like the Release workflow (no git tag).
set -eu

ROOT="$(cd -- "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

TRIPLE="${1:-$(rustc -vV | sed -n 's/^host: //p')}"
if [ -z "$TRIPLE" ]; then
  echo "error: could not determine target triple" >&2
  exit 1
fi

read_workspace_version() {
  sed -n '/^\[workspace\.package\]/,/^\[/p' Cargo.toml | sed -n 's/^version = "\(.*\)"/\1/p' | head -1
}

VERSION="$(read_workspace_version)"
if [ -z "$VERSION" ]; then
  echo "error: could not read [workspace.package] version from Cargo.toml" >&2
  exit 1
fi

ARCH="${TRIPLE%%-*}"
OS="${TRIPLE#*-}"
OS="${OS%%-*}"

case "$OS" in
  unknown) OS="linux" ;;
  apple) OS="darwin" ;;
  pc-windows) OS="windows" ;;
esac

if [ "$OS" = "windows" ]; then
  ASSET="nyra-${ARCH}-windows.zip"
else
  ASSET="nyra-${ARCH}-${OS}.tar.gz"
fi

echo "ci-package-release: VERSION=${VERSION} TRIPLE=${TRIPLE} ASSET=${ASSET}"

# shellcheck disable=SC1091
. "$(dirname "$0")/install-release-llvm.sh" "$TRIPLE"

rustup target add "$TRIPLE" 2>/dev/null || true

rm -rf dist
"$(dirname "$0")/package-release.sh" "$VERSION" "$TRIPLE"

ASSET_PATH="dist/${ASSET}"
if [ ! -f "$ASSET_PATH" ]; then
  echo "error: expected release asset missing: ${ASSET_PATH}" >&2
  exit 1
fi

if [ "$OS" = "windows" ]; then
  unzip -l "$ASSET_PATH" | grep -E 'bin/nyra\.exe|share/stdlib/|version$|env\.ps1$' >/dev/null
else
  tar -tzf "$ASSET_PATH" | grep -E '^bin/nyra$|^share/stdlib/|^version$|^env$' >/dev/null
fi

if [ "$OS" = "windows" ]; then
  file_ver="$(unzip -p "$ASSET_PATH" version 2>/dev/null || true)"
else
  file_ver="$(tar -xOf "$ASSET_PATH" version 2>/dev/null || true)"
fi
if [ "$file_ver" != "$VERSION" ]; then
  echo "error: archive version file '${file_ver:-<empty>}' != Cargo.toml (${VERSION})" >&2
  exit 1
fi

echo "OK: release package smoke passed (${ASSET})"
