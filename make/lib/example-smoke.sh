#!/usr/bin/env bash
# Compile-check examples not fully covered elsewhere (root smokes, rust-bridge, projects).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
export NYRA_TEST_STATS_FILE="${NYRA_TEST_STATS_FILE:-$ROOT/target/.nyra-test-all-stats}"
# shellcheck source=test-stats.sh
source "$ROOT/make/lib/test-stats.sh"

log() { echo "example-smoke: $*" >&2; }
fail() { log "FAILED: $*"; exit 1; }

check() {
  local path="$1"
  log "check $path"
  if ! nyra_stats_check "$path"; then
    fail "check $path"
  fi
}

# Root-level example files (including smoke tests missing from corpus manifest).
while IFS= read -r -d '' path; do
  rel="${path#$ROOT/}"
  check "$rel"
done < <(find "$ROOT/examples" -maxdepth 1 -name '*.ny' -print0 | sort -z)

# Rust bridge + language bridge examples.
check examples/rust-bridge/uuid/main.ny
check examples/rust-bridge/regex/main.ny
check examples/bridge/main.ny

# Project examples (compile-only; runtime needs services / network).
check examples/projects/tcp_echo/server.ny
check examples/projects/tcp_echo/client.ny
check examples/projects/https_smoke/main.ny
check examples/projects/read_file/main.ny
check examples/projects/http_hello/server_main.ny
check examples/ffi/call_libc/main.ny
check examples/ffi/hello_from_rust/main.ny
check examples/unsafe/raw_memory/main.ny
check examples/os/asm/main.ny
check examples/os/battery/main.ny
check examples/os/platform/main.ny
check examples/os/minimal/main.ny
check examples/os/minimal/getenv.ny
check examples/os/minimal/getenv2.ny
check examples/os/minimal/import_os.ny
check examples/os/minimal/name.ny
check examples/packages/ny-sqlite
check examples/packages/ny-serde
check examples/packages/ny-toml
check examples/serde_json_pkg
check examples/stdlib/demo
check examples/stdlib/extended
check examples/stdlib/vec_smoke
check examples/language_features/demo.ny

log "ok — example smoke"
