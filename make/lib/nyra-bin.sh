#!/usr/bin/env bash
# Resolve target/debug/nyra for test scripts (avoids hundreds of `cargo run -p cli` spawns).
set -euo pipefail

_nyra_bin_repo_root() {
  local here
  here="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  printf '%s' "$here"
}

# Export NYRA_BIN (absolute path to the nyra executable). Builds cli once if missing.
nyra_export_cli() {
  local root="${NYRA_ROOT:-$(_nyra_bin_repo_root)}"
  local bin="${NYRA_BIN:-$root/target/debug/nyra}"
  # Windows: cargo emits nyra.exe; MSYS -x may treat nyra as executable when only .exe exists.
  if [[ ! -f "$bin" && -f "$root/target/debug/nyra.exe" ]]; then
    bin="$root/target/debug/nyra.exe"
  fi
  if [[ ! -f "$bin" ]]; then
    (cd "$root" && cargo build -q -p cli -p compiler-ffi)
  fi
  if [[ ! -f "$bin" && -f "$root/target/debug/nyra.exe" ]]; then
    bin="$root/target/debug/nyra.exe"
  fi
  if [[ ! -f "$bin" ]]; then
    echo "nyra-bin: missing executable: $bin" >&2
    return 1
  fi
  export NYRA_ROOT="$root"
  export NYRA_BIN="$bin"
  export NYRA="$bin"
}
