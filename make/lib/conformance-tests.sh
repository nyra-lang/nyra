#!/usr/bin/env bash
# Language conformance tests (CONF-LANG): pass (nyra test) + fail (nyra check) + fixtures.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
export NYRA_TEST_STATS_FILE="${NYRA_TEST_STATS_FILE:-$ROOT/target/.nyra-test-all-stats}"
# shellcheck source=test-stats.sh
source "$ROOT/make/lib/test-stats.sh"

log() { echo "conformance-tests: $*" >&2; }
fail() { log "FAILED: $*"; exit 1; }

normalize_text_out() {
  # Windows runners emit CRLF from print(); normalize before comparing expected output.
  printf '%s' "$1" | tr -d '\r'
}

# shellcheck source=nyra-bin.sh
source "$ROOT/make/lib/nyra-bin.sh"
nyra_export_cli
NYRA="$NYRA_BIN"

PASS_ROOT="$ROOT/tests/conformance/pass"
FAIL_ROOT="$ROOT/tests/conformance/fail"

# --- Pass: runtime feature tests ---
log "CONF-LANG pass: nyra test $PASS_ROOT"
ec=0
out="$(normalize_text_out "$("$NYRA" test "$PASS_ROOT" 2>&1)")" || ec=$?
if ((ec != 0)); then
  printf '%s\n' "$out" >&2
  fail "nyra test $PASS_ROOT (exit $ec)"
fi
printf '%s\n' "$out" >&2
if ! printf '%s\n' "$out" | grep -q 'tests passed'; then
  fail "nyra test $PASS_ROOT (no tests passed line)"
fi
nyra_stats_pass

# --- Fail: must not compile ---
log "CONF-LANG fail: nyra check $FAIL_ROOT (expect compile errors)"
fail_count=0
fail_ran=0
while IFS= read -r -d '' f; do
  fail_ran=$((fail_ran + 1))
  if "$NYRA" check "$f" >/dev/null 2>&1; then
    log "expected compile failure but succeeded: $f"
    fail_count=$((fail_count + 1))
  else
    log "ok (rejected): ${f#"$ROOT"/}"
  fi
done < <(find "$FAIL_ROOT" -name '*.ny' -type f -print0 | sort -z)

if [[ "$fail_ran" -eq 0 ]]; then
  fail "no fail tests under $FAIL_ROOT"
fi
if [[ "$fail_count" -ne 0 ]]; then
  fail "$fail_count/$fail_ran fail tests compiled when they should not"
fi
log "CONF-LANG fail: $fail_ran/$fail_ran rejected as expected"
nyra_stats_pass

# --- Fixture: multi-file import + run ---
IMPORT_FIX="$ROOT/tests/conformance/fixtures/import_smoke"
log "CONF-LANG fixture: nyra run $IMPORT_FIX"
if ! run_out="$("$NYRA" run "$IMPORT_FIX" 2>/dev/null)"; then
  fail "nyra run $IMPORT_FIX"
fi
run_out="$(normalize_text_out "$run_out")"
if [[ "$run_out" != $'hello-import\n42' ]]; then
  printf '%s\n' "$run_out" >&2
  fail "import_smoke expected hello-import + 42, got: $(printf %q "$run_out")"
fi
nyra_stats_pass

log "ok — language conformance (pass + fail + fixtures)"
