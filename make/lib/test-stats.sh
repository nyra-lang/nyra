#!/usr/bin/env bash
# Shared pass / error / warning counters for scripts/test-all.sh and smoke scripts.
# Counters persist in NYRA_TEST_STATS_FILE so child scripts can update them.

: "${NYRA_TEST_STATS_FILE:=}"

nyra_stats_init() {
  local root="${NYRA_TEST_STATS_FILE%/*}"
  if [[ -n "$root" && "$root" != "$NYRA_TEST_STATS_FILE" ]]; then
    mkdir -p "$root"
  fi
  printf '0 0 0\n' >"$NYRA_TEST_STATS_FILE"
}

nyra_stats_read() {
  if [[ ! -f "$NYRA_TEST_STATS_FILE" ]]; then
    nyra_stats_init
  fi
  # shellcheck disable=SC2034
  read -r NYRA_TEST_STATS_PASSED NYRA_TEST_STATS_ERRORS NYRA_TEST_STATS_WARNINGS \
    <"$NYRA_TEST_STATS_FILE"
}

nyra_stats_write() {
  printf '%s %s %s\n' \
    "${NYRA_TEST_STATS_PASSED:-0}" \
    "${NYRA_TEST_STATS_ERRORS:-0}" \
    "${NYRA_TEST_STATS_WARNINGS:-0}" \
    >"$NYRA_TEST_STATS_FILE"
}

nyra_stats_pass() {
  nyra_stats_read
  NYRA_TEST_STATS_PASSED=$((NYRA_TEST_STATS_PASSED + 1))
  nyra_stats_write
}

nyra_stats_add_passes() {
  local n="${1:-0}"
  nyra_stats_read
  NYRA_TEST_STATS_PASSED=$((NYRA_TEST_STATS_PASSED + n))
  nyra_stats_write
}

nyra_stats_add_errors() {
  local n="${1:-0}"
  nyra_stats_read
  NYRA_TEST_STATS_ERRORS=$((NYRA_TEST_STATS_ERRORS + n))
  nyra_stats_write
}

nyra_stats_add_warnings() {
  local n="${1:-0}"
  nyra_stats_read
  NYRA_TEST_STATS_WARNINGS=$((NYRA_TEST_STATS_WARNINGS + n))
  nyra_stats_write
}

nyra_stats_count_diagnostics() {
  local text="$1"
  local w=0 e=0
  if [[ -n "$text" ]]; then
    w=$(printf '%s\n' "$text" | grep -c 'warning\[W' || true)
    e=$(printf '%s\n' "$text" | grep -cE '^error(\[E|:)' || true)
  fi
  nyra_stats_add_warnings "$w"
  nyra_stats_add_errors "$e"
}

nyra_stats_check() {
  local path="$1"
  local out=""
  local ec=0
  if [[ -z "${NYRA_BIN:-}" || ! -x "${NYRA_BIN:-}" ]]; then
    # shellcheck source=nyra-bin.sh
    source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/nyra-bin.sh"
    nyra_export_cli
  fi
  out="$("$NYRA_BIN" check "$path" 2>&1)" || ec=$?
  if [[ -n "$out" && "${NYRA_TEST_ALL:-}" != "1" ]]; then
    printf '%s\n' "$out" >&2
  fi
  nyra_stats_count_diagnostics "$out"
  if (( ec == 0 )); then
    nyra_stats_pass
    return 0
  fi
  if [[ "${NYRA_TEST_ALL:-}" == "1" ]] && declare -f ta_record_failure >/dev/null 2>&1; then
    ta_record_failure "check $path" "$out"
  fi
  return "$ec"
}

nyra_stats_add_cargo_test_results() {
  local text="$1"
  local tp tf
  tp=$(printf '%s\n' "$text" | grep -E 'test result:.*passed' \
    | sed -E 's/.* ([0-9]+) passed.*/\1/' | awk '{s+=$1} END{print s+0}')
  tf=$(printf '%s\n' "$text" | grep -E 'test result:.*failed' \
    | sed -E 's/.*; ([0-9]+) failed.*/\1/' | awk '{s+=$1} END{print s+0}')
  nyra_stats_add_passes "$tp"
  nyra_stats_add_errors "$tf"
}

if [[ -n "$NYRA_TEST_STATS_FILE" ]]; then
  nyra_stats_read
fi
