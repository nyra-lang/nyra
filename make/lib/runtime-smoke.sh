#!/usr/bin/env bash
# Nyra run/test smoke steps (extracted from smoke.mk for collect-on-fail under test-all).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
# shellcheck source=test-all-collect.sh
source "$ROOT/make/lib/test-all-collect.sh"
ta_set_scope "runtime-smoke"
export NYRA_TEST_STATS_FILE="${NYRA_TEST_STATS_FILE:-$ROOT/target/.nyra-test-all-stats}"
# shellcheck source=test-stats.sh
source "$ROOT/make/lib/test-stats.sh"
# shellcheck source=nyra-bin.sh
source "$ROOT/make/lib/nyra-bin.sh"
nyra_export_cli
NYRA="$NYRA_BIN"

log() { echo "runtime-smoke: $*" >&2; }
fail() {
  local label="$1"
  local detail="${2:-}"
  log "FAILED: $label"
  ta_fail "$label" "$detail" || exit 1
}

run_expect() {
  local label="$1"
  shift
  local out=""
  if ! out="$("$@" 2>&1)"; then
    fail "$label" "$out"
    return 0
  fi
  if [[ -n "$out" && "${NYRA_TEST_ALL:-}" != "1" ]]; then
    printf '%s\n' "$out"
  fi
  nyra_stats_pass
}

run_expect_eq() {
  local label="$1"
  local expected="$2"
  shift 2
  local out=""
  if ! out="$("$@" 2>&1)"; then
    fail "$label" "$out"
    return 0
  fi
  if [[ "$out" != "$expected" ]]; then
    fail "$label (expected $(printf %q "$expected"), got $(printf %q "$out"))" "$out"
    return 0
  fi
  if [[ "${NYRA_TEST_ALL:-}" != "1" ]]; then
    printf '%s\n' "$out"
  fi
  nyra_stats_pass
}

run_expect "$NYRA run examples/syntax/hello.ny"
run_expect "$NYRA run examples/syntax/for_in.ny"
run_expect "$NYRA run examples/syntax/string_methods.ny"
run_expect "$NYRA run examples/syntax/date_basics.ny"
run_expect "$NYRA run examples/syntax/array_sort.ny"
run_expect_eq "examples/syntax/math.ny" "30" "$NYRA" run examples/syntax/math.ny
run_expect "$NYRA run examples/syntax/hashmap_chain.ny"
run_expect "$NYRA run tests/nyra/net/gaps_fix_test.ny"
run_expect "$NYRA run tests/nyra/net/map_drop_test.ny"
run_expect "$NYRA run tests/nyra/net/net_prod_test.ny"
run_expect "$NYRA run tests/nyra/net/net_prod_test.typed.ny"
run_expect "$NYRA run tests/nyra/language_gaps.ny"
run_expect "$NYRA run tests/nyra/language_gaps.typed.ny"
run_expect "$NYRA test tests/nyra/match_or_test.ny"
run_expect "$NYRA run tests/nyra/match_or_test.typed.ny"
run_expect "$NYRA test tests/nyra/match_nested_test.ny"
run_expect "$NYRA run tests/nyra/match_nested_test.typed.ny"
run_expect "$NYRA test tests/nyra/match_struct_tuple_test.ny"
run_expect "$NYRA run tests/nyra/match_struct_tuple_test.typed.ny"
run_expect "$NYRA run tests/nyra/modules_test.ny"
run_expect "$NYRA run tests/nyra/modules_test.typed.ny"
run_expect "$NYRA run tests/nyra/stdlib_gaps.ny"
run_expect "$NYRA run tests/nyra/stdlib_gaps.typed.ny"
run_expect "$NYRA run tests/nyra/games_stdlib.ny"
run_expect "$NYRA run tests/nyra/games_stdlib.typed.ny"
run_expect "$NYRA run tests/nyra/games_gaps.ny"
run_expect "$NYRA run examples/dev/compiler_inprocess.ny"
run_expect "$NYRA test tests/nyra/parser_gaps_test.ny"
run_expect "$NYRA test tests/nyra/parser_gaps.typed.ny"
run_expect "$NYRA build examples/projects/calculator"
run_expect "$NYRA test examples/smoke_test_test.ny"

ta_finish "runtime-smoke"
log "ok — runtime smoke"
