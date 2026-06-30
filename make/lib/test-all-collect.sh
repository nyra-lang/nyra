#!/usr/bin/env bash
# Per-test failure collection for make test-all / test-platform-core.
# When NYRA_TEST_ALL=1: record failures and keep running; strict exit otherwise.
set -euo pipefail

_ta_collect_failures=0
_ta_collect_scope="${NYRA_TEST_COLLECT_SCOPE:-}"

ta_set_scope() {
  _ta_collect_scope="$1"
}

ta_record_failure() {
  local label="$1"
  local detail="${2:-}"
  _ta_collect_failures=$((_ta_collect_failures + 1))
  if [[ -n "${TEST_ALL_FAILURES_FILE:-}" ]]; then
    local scope="${_ta_collect_scope:-script}"
    {
      printf '\n========== FAILED: %s / %s ==========\n' "$scope" "$label"
      if [[ -n "$detail" ]]; then
        printf '%s\n' "$detail"
      fi
      printf '========== end: %s / %s ==========\n' "$scope" "$label"
    } >>"$TEST_ALL_FAILURES_FILE"
  fi
}

# Call at end of multi-step scripts. Exits 1 if any failure was recorded.
ta_finish() {
  local scope="${1:-${_ta_collect_scope:-script}}"
  if ((_ta_collect_failures > 0)); then
    printf '%s: %s test(s) failed — full output in %s\n' \
      "$scope" "$_ta_collect_failures" "${TEST_ALL_FAILURES_FILE:-stderr}" >&2
    exit 1
  fi
}

# Drop-in fail() for scripts: continue under test-all, exit immediately otherwise.
ta_fail() {
  local label="$1"
  local detail="${2:-}"
  if [[ "${NYRA_TEST_ALL:-}" == "1" ]]; then
    ta_record_failure "$label" "$detail"
    return 0
  fi
  return 1
}

ta_quiet() {
  [[ "${NYRA_TEST_ALL:-}" == "1" ]]
}

# Nyra program stdout only (warnings / incremental notes stay on stderr).
ta_nyra_stdout() {
  "$@" 2>/dev/null
}
