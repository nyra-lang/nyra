#!/usr/bin/env bash
# Cross-compilation smoke tests for Nyra CLI (linux / windows / wasm).
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
. "$ROOT/make/lib/wasm-toolchain.sh"
. "$ROOT/make/lib/cross-target-helpers.sh"

NYRA="${NYRA:-cargo run --quiet --}"
HELLO="${ROOT}/examples/syntax/hello.ny"
HELLO_DIR="${ROOT}/examples/syntax"

echo "== cross-smoke: wasm32-wasip1 =="
if wasm_toolchain_ready; then
  $NYRA build "$HELLO" --for wasm -o hello.wasm
  WASM_BIN="$(cross_find_artifact "$HELLO_DIR" debug hello.wasm)" || {
    echo "missing wasm artifact under $HELLO_DIR/target/" >&2
    exit 1
  }
  echo "wasm artifact: $WASM_BIN"
  if command -v wasmtime >/dev/null 2>&1; then
    wasmtime "$WASM_BIN"
  else
    echo "note: wasmtime not installed; skipping wasm run"
  fi
else
  wasm_toolchain_hint
fi

echo "== cross-smoke: linux =="
if cross_linux_linker_ready; then
  $NYRA build "$HELLO" --release --for linux
  LINUX_BIN="$(cross_find_artifact "$HELLO_DIR" release hello)" || {
    echo "missing linux artifact under $HELLO_DIR/target/" >&2
    exit 1
  }
  echo "linux artifact: $LINUX_BIN"
  if [ -x "$LINUX_BIN" ]; then
    "$LINUX_BIN"
  fi
else
  cross_log_skip "linux cross linker unavailable"
fi

echo "== cross-smoke: windows =="
if cross_windows_linker_ready; then
  $NYRA build "${ROOT}/examples/syntax/spawn_channel.ny" --for windows -o spawn_win.exe
  WIN_BIN="$(cross_find_artifact "$HELLO_DIR" debug spawn_win.exe)" || {
    echo "missing windows cross artifact under $HELLO_DIR/target/" >&2
    exit 1
  }
  echo "windows cross artifact: $WIN_BIN"
else
  cross_log_skip "mingw-w64 not installed"
fi

echo "cross-smoke: ok"
