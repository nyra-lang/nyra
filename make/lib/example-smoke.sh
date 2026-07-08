#!/usr/bin/env bash
# Compile-check examples not fully covered elsewhere (root smokes, rust-bridge, projects).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
# shellcheck source=test-all-collect.sh
source "$ROOT/make/lib/test-all-collect.sh"
ta_set_scope "example-smoke"
export NYRA_TEST_STATS_FILE="${NYRA_TEST_STATS_FILE:-$ROOT/target/.nyra-test-all-stats}"
# shellcheck source=test-stats.sh
source "$ROOT/make/lib/test-stats.sh"

log() { echo "example-smoke: $*" >&2; }
fail() {
  log "FAILED: $*"
  ta_fail "$*" "" || exit 1
}

check() {
  local path="$1"
  if [[ "${NYRA_TEST_ALL:-}" != "1" ]]; then
    log "check $path"
  fi
  if ! nyra_stats_check "$path"; then
    if [[ "${NYRA_TEST_ALL:-}" != "1" ]]; then
      fail "check $path"
    fi
    return 1
  fi
  return 0
}

failed=0
while IFS= read -r -d '' path; do
  rel="${path#$ROOT/}"
  check "$rel" || failed=$((failed + 1))
done < <(find "$ROOT/examples" -maxdepth 1 -name '*.ny' -print0 | sort -z)

check examples/rust-bridge/uuid/main.ny || failed=$((failed + 1))
check examples/rust-bridge/regex/main.ny || failed=$((failed + 1))
check examples/bridge/main.ny || failed=$((failed + 1))
check examples/projects/tcp_echo/server.ny || failed=$((failed + 1))
check examples/projects/tcp_echo/client.ny || failed=$((failed + 1))
check examples/projects/https_smoke/main.ny || failed=$((failed + 1))
check examples/projects/read_file/main.ny || failed=$((failed + 1))
check examples/projects/http_hello/server_main.ny || failed=$((failed + 1))
check examples/ffi/call_libc/main.ny || failed=$((failed + 1))
check examples/ffi/hello_from_rust/main.ny || failed=$((failed + 1))
check examples/unsafe/raw_memory/main.ny || failed=$((failed + 1))
check examples/os/asm/main.ny || failed=$((failed + 1))
check examples/os/battery/main.ny || failed=$((failed + 1))
check examples/os/platform/main.ny || failed=$((failed + 1))
check examples/os/minimal/main.ny || failed=$((failed + 1))
check examples/os/minimal/getenv.ny || failed=$((failed + 1))
check examples/os/minimal/getenv2.ny || failed=$((failed + 1))
check examples/os/minimal/import_os.ny || failed=$((failed + 1))
check examples/os/minimal/name.ny || failed=$((failed + 1))
check examples/packages/ny-sqlite || failed=$((failed + 1))
check examples/packages/ny-serde || failed=$((failed + 1))
check examples/packages/ny-toml || failed=$((failed + 1))
check examples/serde_json_pkg || failed=$((failed + 1))
check examples/stdlib/demo || failed=$((failed + 1))
check examples/stdlib/extended || failed=$((failed + 1))
check examples/stdlib/vec_smoke || failed=$((failed + 1))
check examples/language_features/demo.ny || failed=$((failed + 1))

if (( failed > 0 )); then
  ta_finish "example-smoke"
fi
log "ok — example smoke"
