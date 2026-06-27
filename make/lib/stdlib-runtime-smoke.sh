#!/usr/bin/env bash
# Runtime smoke for core stdlib modules (beyond compile-only nyra check).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
export NYRA_TEST_STATS_FILE="${NYRA_TEST_STATS_FILE:-$ROOT/target/.nyra-test-all-stats}"
# shellcheck source=test-stats.sh
source "$ROOT/make/lib/test-stats.sh"
# shellcheck source=nyra-bin.sh
source "$ROOT/make/lib/nyra-bin.sh"
nyra_export_cli
NYRA=("$NYRA_BIN")

EXAMPLE="$ROOT/examples/stdlib_runtime_smoke.ny"
TYPED="$ROOT/examples/stdlib_runtime_smoke.typed.ny"

log() { echo "stdlib-runtime-smoke: $*" >&2; }
fail() { log "FAILED: $*"; exit 1; }

run_expect() {
  local label="$1"
  local path="$2"
  local needle="$3"
  local out
  out="$("${NYRA[@]}" run "$path" 2>&1)" || {
    printf '%s\n' "$out" >&2
    fail "run $label"
  }
  if ! printf '%s\n' "$out" | grep -q "$needle"; then
    printf '%s\n' "$out" >&2
    fail "run $label (missing output containing: $needle)"
  fi
  log "ok — run $label"
  nyra_stats_pass
}

log "check zero-types runtime smoke"
if ! nyra_stats_check "$EXAMPLE"; then
  fail "check $EXAMPLE"
fi

log "check typed runtime smoke"
if ! nyra_stats_check "$TYPED"; then
  fail "check $TYPED"
fi

run_expect "stdlib_runtime_smoke.ny" "$EXAMPLE" "stdlib-runtime ok"
run_expect "stdlib_runtime_smoke.typed.ny" "$TYPED" "stdlib-runtime ok"

log "done"
