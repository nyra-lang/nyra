#!/usr/bin/env bash
# Run one test-all gate; record failures; never abort the suite.
set -euo pipefail

: "${ROOT:?ROOT required}"
: "${TEST_ALL_FAILURES_FILE:?TEST_ALL_FAILURES_FILE required}"

record_failure() {
  local label="$1"
  local detail="$2"
  {
    printf '\n========== FAILED: %s ==========\n' "$label"
    printf '%s\n' "$detail"
    printf '========== end: %s ==========\n' "$label"
  } >>"$TEST_ALL_FAILURES_FILE"
}

gate_log_step() {
  printf 'make: ⏳ %s ...\n' "$1"
}

gate_log_ok() {
  printf 'make: ✅ ok — %s\n' "$1"
}

gate_log_fail() {
  printf 'make: ❌ failed — %s\n' "$1"
}

gate_make() {
  local target="$1"
  local label="${2:-$target}"
  local log ec

  gate_log_step "$label"
  log="$(mktemp "${TMPDIR:-/tmp}/nyra-gate.XXXXXX")"
  if make -C "$ROOT" "$target" >"$log" 2>&1; then
    gate_log_ok "$label"
    rm -f "$log"
    return 0
  fi
  ec=$?
  gate_log_fail "$label"
  record_failure "$label (make $target)" "$(cat "$log")"
  rm -f "$log"
  return 0
}

gate_cmd() {
  local label="$1"
  shift
  local log ec

  gate_log_step "$label"
  log="$(mktemp "${TMPDIR:-/tmp}/nyra-gate.XXXXXX")"
  if "$@" >"$log" 2>&1; then
    gate_log_ok "$label"
    rm -f "$log"
    return 0
  fi
  ec=$?
  gate_log_fail "$label"
  record_failure "$label ($*)" "$(cat "$log")"
  rm -f "$log"
  return 0
}

gate_init() {
  local root="${TEST_ALL_FAILURES_FILE%/*}"
  if [[ -n "$root" && "$root" != "$TEST_ALL_FAILURES_FILE" ]]; then
    mkdir -p "$root"
  fi
  : >"$TEST_ALL_FAILURES_FILE"
}

gate_failure_count() {
  if [[ ! -s "$TEST_ALL_FAILURES_FILE" ]]; then
    printf '0'
    return
  fi
  grep -c '^========== FAILED:' "$TEST_ALL_FAILURES_FILE" || true
}

gate_summary() {
  local n
  n="$(gate_failure_count)"
  if [[ "$n" -gt 0 ]]; then
    printf '\n'
    printf 'make: ❌ test-all finished with %s failed gate(s) at %s\n' \
      "$n" "$(date '+%Y-%m-%d %H:%M:%S')"
    printf 'make: failure log: %s\n\n' "$TEST_ALL_FAILURES_FILE"
    cat "$TEST_ALL_FAILURES_FILE"
    return 1
  fi
  printf 'make: ✅ test-all completed successfully at %s\n' \
    "$(date '+%Y-%m-%d %H:%M:%S')"
  return 0
}

case "${1:-}" in
  init) gate_init ;;
  make) shift; gate_make "$@" ;;
  cmd) shift; gate_cmd "$@" ;;
  count) printf '%s\n' "$(gate_failure_count)" ;;
  summary) gate_summary ;;
  *)
    printf 'usage: %s init|make <target> [label]|cmd <label> <cmd...>|summary\n' "$0" >&2
    exit 2
    ;;
esac
