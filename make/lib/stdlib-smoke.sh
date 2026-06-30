#!/usr/bin/env bash
# Compile-check every Nyra stdlib module. Full suite: make test-all
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
# shellcheck source=test-all-collect.sh
source "$ROOT/make/lib/test-all-collect.sh"
ta_set_scope "stdlib-smoke"
export NYRA_TEST_STATS_FILE="${NYRA_TEST_STATS_FILE:-$ROOT/target/.nyra-test-all-stats}"
# shellcheck source=test-stats.sh
source "$ROOT/make/lib/test-stats.sh"

log() { echo "stdlib-smoke: $*" >&2; }
fail() {
  log "FAILED: $*"
  ta_fail "$*" "" || exit 1
}

count=0
failed=0
while IFS= read -r -d '' path; do
  rel="${path#$ROOT/}"
  if [[ "${NYRA_TEST_ALL:-}" != "1" ]]; then
    log "check $rel"
  fi
  if ! nyra_stats_check "$path"; then
    failed=$((failed + 1))
    if [[ "${NYRA_TEST_ALL:-}" != "1" ]]; then
      fail "check $rel"
    fi
  fi
  count=$((count + 1))
done < <(find "$ROOT/stdlib" -name '*.ny' -print0 | sort -z)

if (( failed > 0 )); then
  ta_finish "stdlib-smoke"
fi
log "ok — $count stdlib modules"
