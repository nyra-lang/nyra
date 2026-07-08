#!/usr/bin/env bash
# Fast failure triage (~5–15 min): run the gates that most often break CI, keep going on
# failure, and print one combined report at the end.
#
# Usage:
#   make test-triage
#   # or directly:
#   bash make/lib/test-triage.sh
#
# Full log:  target/.nyra-test-triage.txt
# Failures:  target/.nyra-test-all-failures  (same file as make test-all)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

TRIAGE_LOG="${NYRA_TEST_TRIAGE_LOG:-$ROOT/target/.nyra-test-triage.txt}"
FAILURES_FILE="${TEST_ALL_FAILURES_FILE:-$ROOT/target/.nyra-test-all-failures}"
GATE_LIB="$ROOT/make/lib/test-all-gate.sh"

mkdir -p "$(dirname "$TRIAGE_LOG")"
: >"$FAILURES_FILE"
printf 'test-triage: started %s\nroot: %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$ROOT" >"$TRIAGE_LOG"
printf 'test-triage: failures log: %s\n' "$FAILURES_FILE" | tee -a "$TRIAGE_LOG" >&2

run_gate() {
  local target="$1"
  local label="${2:-$target}"
  ROOT="$ROOT" \
    TEST_ALL_FAILURES_FILE="$FAILURES_FILE" \
    TEST_ALL_LOG="$TRIAGE_LOG" \
    NYRA_TEST_ALL=1 \
    "$GATE_LIB" make "$target" "$label"
}

printf '\n=== test-triage: build ===\n' | tee -a "$TRIAGE_LOG" >&2
run_gate build-workspace "cargo build --workspace"
run_gate build-cli "nyra cli"

printf '\n=== test-triage: compiler / corpus ===\n' | tee -a "$TRIAGE_LOG" >&2
run_gate test-examples-corpus "examples corpus (corpus_e2e_stdout)"

printf '\n=== test-triage: language + smokes ===\n' | tee -a "$TRIAGE_LOG" >&2
run_gate test-nyra-lang "nyra language tests"
run_gate test-runtime-smoke "runtime smoke"
run_gate smoke-cli "cli smoke"
run_gate smoke-stdlib-priority "stdlib priority smoke"

failures=0
if [[ -s "$FAILURES_FILE" ]]; then
  failures="$(grep -c '^========== FAILED:' "$FAILURES_FILE" || true)"
fi

printf '\n' | tee -a "$TRIAGE_LOG" >&2
if (( failures > 0 )); then
  printf 'test-triage: %s gate(s) failed\n\n' "$failures" | tee -a "$TRIAGE_LOG" >&2
  cat "$FAILURES_FILE" | tee -a "$TRIAGE_LOG" >&2
  printf '\ntest-triage: full log: %s\n' "$TRIAGE_LOG" >&2
  exit 1
fi

printf 'test-triage: all gates passed\n' | tee -a "$TRIAGE_LOG" >&2
printf 'test-triage: log: %s\n' "$TRIAGE_LOG" >&2
