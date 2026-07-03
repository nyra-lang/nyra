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
export NYRA_TEST_TIMEOUT_SECS="${NYRA_TEST_TIMEOUT_SECS:-60}"

PASS_ROOT="$ROOT/tests/conformance/pass"
FAIL_ROOT="$ROOT/tests/conformance/fail"

# --- Pass: runtime feature tests ---
log "NYRA_BIN=$NYRA"
log "NYRA_TEST_TIMEOUT_SECS=$NYRA_TEST_TIMEOUT_SECS"
log "CONF-LANG pass: nyra test $PASS_ROOT (streaming output)"
pass_log="$ROOT/target/.nyra-conformance-pass.out"
mkdir -p "$ROOT/target"
: >"$pass_log"
if ! "$NYRA" test "$PASS_ROOT" 2>&1 | tr -d '\r' | tee "$pass_log" >&2; then
  fail "nyra test $PASS_ROOT"
fi
if ! grep -q 'tests passed' "$pass_log"; then
  fail "nyra test $PASS_ROOT (no tests passed line)"
fi
rm -f "$pass_log"
nyra_stats_pass

# --- Fail: must not compile ---
log "CONF-LANG fail: nyra check $FAIL_ROOT (expect compile errors)"
fail_count=0
fail_ran=0
while IFS= read -r -d '' f; do
  fail_ran=$((fail_ran + 1))
  log "check expected-fail: ${f#"$ROOT"/}"
  check_out="$("$NYRA" check "$f" 2>&1)" && check_ec=0 || check_ec=$?
  if ((check_ec == 0)); then
    printf '%s\n' "$check_out" >&2
    log "expected compile failure but succeeded: $f"
    fail_count=$((fail_count + 1))
  else
    if [[ "${NYRA_TEST_VERBOSE_FAILS:-0}" == "1" ]]; then
      printf '%s\n' "$check_out" >&2
    fi
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
log "CONF-LANG fixture: nyra run $IMPORT_FIX (streaming output)"
fixture_out_log="$ROOT/target/.nyra-conformance-import-smoke.stdout"
fixture_err_log="$ROOT/target/.nyra-conformance-import-smoke.stderr"
: >"$fixture_out_log"
: >"$fixture_err_log"
if ! "$NYRA" run "$IMPORT_FIX" \
  > >(tee "$fixture_out_log" >&2) \
  2> >(tee "$fixture_err_log" >&2); then
  fail "nyra run $IMPORT_FIX"
fi
run_out="$(normalize_text_out "$(cat "$fixture_out_log")")"
if [[ "$run_out" != $'hello-import\n42' ]]; then
  printf '%s\n' "$run_out" >&2
  fail "import_smoke expected hello-import + 42, got: $(printf %q "$run_out")"
fi
rm -f "$fixture_out_log" "$fixture_err_log"
nyra_stats_pass

log "ok — language conformance (pass + fail + fixtures)"
