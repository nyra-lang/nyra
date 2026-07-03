#!/usr/bin/env bash
# Build O0 dev runtime archive for fast debug links.
# Prefer the nyra CLI (writes a matching stamp); fall back to plain clang+ar.
#
# Usage: build-prebuilt-rt.sh [NYRA_BIN]

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
NYRA_BIN="${1:-${NYRA_BIN:-$(command -v nyra 2>/dev/null || true)}}"

if [[ -n "$NYRA_BIN" && -x "$NYRA_BIN" ]]; then
  "$NYRA_BIN" internal build-prebuilt-rt
  exit 0
fi

DEST="${NYRA_HOME:-$HOME/.nyra}/share/stdlib"
TRIPLE="$(clang -dumpmachine 2>/dev/null || uname -m)"
OUT="$DEST/prebuilt/$TRIPLE"
RT="$DEST/rt"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/nyra-prebuilt-rt.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

[[ -d "$RT" ]] || RT="$ROOT/stdlib/rt"
mkdir -p "$OUT"
OBJS=()
for src in "$RT"/*.c; do
  [[ -f "$src" ]] || continue
  [[ "$src" == *.inc.c ]] && continue
  base="$(basename "$src" .c)"
  clang -c -O0 -Wno-override-module "$src" -o "$WORK/${base}.o"
  OBJS+=("$WORK/${base}.o")
done
AR="$(command -v llvm-ar || command -v ar)"
"$AR" rcs "$OUT/libnyra_rt_dev.a" "${OBJS[@]}"
echo "prebuilt runtime (fallback): $OUT/libnyra_rt_dev.a"
