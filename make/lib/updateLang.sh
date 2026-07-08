#!/usr/bin/env bash
# Build Nyra from this repo and install `nyra` onto the system PATH.
#
# Usage (from anywhere):
#   /path/to/Nyra/scripts/dev-install.sh
#   ./scripts/dev-install.sh          # from repo root
#
# Requires: rust/cargo, clang (for nyra run link step)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

info() { printf '%s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

if ! command -v cargo >/dev/null 2>&1; then
  die "cargo not found. Install Rust: https://rustup.rs"
fi

if ! command -v clang >/dev/null 2>&1; then
  die "clang not found (needed when you run nyra programs).
  macOS: xcode-select --install"
fi

info "==> Nyra dev install (repo: $ROOT)"
if command -v python3 >/dev/null 2>&1; then
  info "==> Regenerating stdlib/nyra_rt.h from ABI manifest..."
  python3 "$ROOT/make/py/gen-abi-header.py"
  info "==> Regenerating docs/bindings.md..."
  python3 "$ROOT/make/py/gen-bindings-doc.py"
fi
info "==> Building release cli + TLS runtimes (rustls + native)..."
cargo build --release -p cli -p nyra-rt-tls -p nyra-rt-tls-native

info "==> Installing to PATH (cargo install --force)..."
cargo install --path cli --force

# Always copy the freshly built binary from cargo's install dir, not
# `command -v nyra` (PATH may prefer an older ~/.nyra/bin/nyra and cp would
# no-op with "identical (not copied)" while leaving the stale binary active).
CARGO_NYRA="${CARGO_HOME:-$HOME/.cargo}/bin/nyra"
if [ ! -x "$CARGO_NYRA" ]; then
  die "expected cargo-installed nyra at $CARGO_NYRA"
fi

NYRA_HOME="${NYRA_HOME:-$HOME/.nyra}"
STD_DEST="$NYRA_HOME/share/stdlib"
info "==> Syncing stdlib to $STD_DEST ..."
mkdir -p "$STD_DEST/rt"
cp stdlib/nyra_rt.c stdlib/nyra_rt_wasi.c stdlib/nyra_rt.h "$STD_DEST/" 2>/dev/null || cp stdlib/nyra_rt.c stdlib/nyra_rt_wasi.c "$STD_DEST/"
[ -f stdlib/nyra_rt.h ] && cp stdlib/nyra_rt.h "$STD_DEST/"
cp stdlib/rt/*.c stdlib/rt/*.h "$STD_DEST/rt/" 2>/dev/null || cp stdlib/rt/*.c "$STD_DEST/rt/"
mkdir -p "$STD_DEST/rt_wasi"
cp stdlib/rt_wasi/*.c stdlib/rt_wasi/*.h "$STD_DEST/rt_wasi/" 2>/dev/null || cp stdlib/rt_wasi/*.c "$STD_DEST/rt_wasi/"
for f in stdlib/*.ny; do
  [ -f "$f" ] && cp "$f" "$STD_DEST/"
done
for sub in os net core http; do
  if [ -d "stdlib/$sub" ]; then
    mkdir -p "$STD_DEST/$sub"
    cp -R "stdlib/$sub/." "$STD_DEST/$sub/"
  fi
done

# Bundle rustls + native TLS clients for HTTPS without requiring Rust on the user machine.
TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
TLS_SRC="target/release/libnyra_rt_tls.a"
TLS_NATIVE_SRC="target/release/libnyra_rt_tls_native.a"
if [ -f "$TLS_SRC" ]; then
  mkdir -p "$STD_DEST/prebuilt/$TRIPLE"
  cp -f "$TLS_SRC" "$STD_DEST/prebuilt/$TRIPLE/libnyra_rt_tls.a"
  mkdir -p "$ROOT/stdlib/prebuilt/$TRIPLE"
  cp -f "$TLS_SRC" "$ROOT/stdlib/prebuilt/$TRIPLE/libnyra_rt_tls.a"
  info "==> Installed libnyra_rt_tls.a for $TRIPLE"
else
  die "missing $TLS_SRC after cargo build -p nyra-rt-tls"
fi
if [ -f "$TLS_NATIVE_SRC" ]; then
  mkdir -p "$STD_DEST/prebuilt/$TRIPLE"
  cp -f "$TLS_NATIVE_SRC" "$STD_DEST/prebuilt/$TRIPLE/libnyra_rt_tls_native.a"
  mkdir -p "$ROOT/stdlib/prebuilt/$TRIPLE"
  cp -f "$TLS_NATIVE_SRC" "$ROOT/stdlib/prebuilt/$TRIPLE/libnyra_rt_tls_native.a"
  info "==> Installed libnyra_rt_tls_native.a for $TRIPLE"
else
  die "missing $TLS_NATIVE_SRC after cargo build -p nyra-rt-tls-native"
fi

if command -v bash >/dev/null 2>&1 && [ -f "$ROOT/make/lib/build-prebuilt-rt.sh" ]; then
  info "==> Building dev runtime archive (fast debug links)..."
  bash "$ROOT/make/lib/build-prebuilt-rt.sh" "$CARGO_NYRA"
fi

NYRA_BIN_DIR="$NYRA_HOME/bin"
mkdir -p "$NYRA_BIN_DIR"
cp -f "$CARGO_NYRA" "$NYRA_BIN_DIR/nyra"
installed_ver="$("$NYRA_BIN_DIR/nyra" --version 2>/dev/null | sed 's/^nyra //')"
if [ -n "$installed_ver" ]; then
  printf '%s\n' "$installed_ver" > "$NYRA_HOME/version"
fi
info "==> Linked dev binary into $NYRA_BIN_DIR/nyra"
info ""
info "Done. Active binary:"
info "  $NYRA_BIN_DIR/nyra"
"$NYRA_BIN_DIR/nyra" -V 2>/dev/null || "$NYRA_BIN_DIR/nyra" --version 2>/dev/null || true
