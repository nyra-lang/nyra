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
info "==> Building release cli..."
cargo build --release -p cli

info "==> Installing to PATH (cargo install --force)..."
cargo install --path cli --force

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

if command -v bash >/dev/null 2>&1 && [ -f "$ROOT/make/lib/build-prebuilt-rt.sh" ]; then
  info "==> Building dev runtime archive (fast debug links)..."
  bash "$ROOT/make/lib/build-prebuilt-rt.sh" "$(command -v nyra)"
fi

if command -v nyra >/dev/null 2>&1; then
  info ""
  info "Done. Active binary:"
  info "  $(command -v nyra)"
  nyra -V 2>/dev/null || nyra --version 2>/dev/null || true
else
  die "nyra not on PATH after install. Ensure \$HOME/.cargo/bin is in PATH."
fi
