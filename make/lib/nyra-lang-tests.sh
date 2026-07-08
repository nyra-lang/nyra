#!/usr/bin/env bash
# Native Nyra test files (syntax, ownership, imports) via `nyra test`.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
export NYRA_TEST_STATS_FILE="${NYRA_TEST_STATS_FILE:-$ROOT/target/.nyra-test-all-stats}"
# shellcheck source=test-stats.sh
source "$ROOT/make/lib/test-stats.sh"

log() { echo "nyra-lang-tests: $*" >&2; }
fail() { log "FAILED: $*"; exit 1; }

# shellcheck source=nyra-bin.sh
source "$ROOT/make/lib/nyra-bin.sh"
nyra_export_cli
NYRA=("$NYRA_BIN")
export NYRA_TEST_TIMEOUT_SECS="${NYRA_TEST_TIMEOUT_SECS:-60}"

log "NYRA_BIN=$NYRA_BIN"
log "NYRA_TEST_TIMEOUT_SECS=$NYRA_TEST_TIMEOUT_SECS"
log "nyra test tests/nyra (streaming output)"
nyra_test_log="$ROOT/target/.nyra-lang-tests.out"
mkdir -p "$ROOT/target"
: >"$nyra_test_log"
log "stream log: $nyra_test_log"
if ! "${NYRA[@]}" test tests/nyra 2>&1 | tee "$nyra_test_log" >&2; then
  fail "nyra test tests/nyra"
fi
if ! grep -q 'tests passed' "$nyra_test_log"; then
  fail "nyra test tests/nyra (no tests passed line)"
fi
rm -f "$nyra_test_log"
nyra_stats_pass

log "import_consts fixture (multi-file import + const)"
import_out_log="$ROOT/target/.nyra-import-consts.stdout"
import_err_log="$ROOT/target/.nyra-import-consts.stderr"
: >"$import_out_log"
: >"$import_err_log"
log "nyra run $ROOT/tests/fixtures/import_consts (streaming output)"
if ! "${NYRA[@]}" run "$ROOT/tests/fixtures/import_consts" \
  > >(tee "$import_out_log" >&2) \
  2> >(tee "$import_err_log" >&2); then
  fail "nyra run import_consts fixture"
fi
out="$(tr -d '\r' <"$import_out_log")"
if [[ "$out" != $'Hello\n42' ]]; then
  printf '%s\n' "$out" >&2
  fail "import_consts fixture expected Hello+42, got: $(printf %q "$out")"
fi
rm -f "$import_out_log" "$import_err_log"
nyra_stats_pass

log "ok — nyra native language tests"
