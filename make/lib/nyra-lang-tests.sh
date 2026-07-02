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

log "nyra test tests/nyra"
nyra_test_log="$ROOT/target/.nyra-lang-tests.out"
mkdir -p "$ROOT/target"
: >"$nyra_test_log"
if ! "${NYRA[@]}" test tests/nyra 2>&1 | tee "$nyra_test_log" >&2; then
  fail "nyra test tests/nyra"
fi
if ! grep -q 'tests passed' "$nyra_test_log"; then
  fail "nyra test tests/nyra (no tests passed line)"
fi
rm -f "$nyra_test_log"
nyra_stats_pass

log "import_consts fixture (multi-file import + const)"
out="$("${NYRA[@]}" run "$ROOT/tests/fixtures/import_consts" 2>/dev/null)" || {
  fail "nyra run import_consts fixture"
}
if [[ "$out" != $'Hello\n42' ]]; then
  printf '%s\n' "$out" >&2
  fail "import_consts fixture expected Hello+42, got: $(printf %q "$out")"
fi
nyra_stats_pass

log "ok — nyra native language tests"
