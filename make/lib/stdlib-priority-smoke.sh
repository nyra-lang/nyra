#!/usr/bin/env bash
# Smoke-test high-priority stdlib modules (strconv, flag, bufio, context, sync, csv, mime).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
# shellcheck source=test-all-collect.sh
source "$ROOT/make/lib/test-all-collect.sh"
ta_set_scope "stdlib-priority-smoke"
# shellcheck source=nyra-bin.sh
source "$ROOT/make/lib/nyra-bin.sh"
if [[ -z "${NYRA_BIN:-}" ]]; then nyra_export_cli; fi
NYRA="${NYRA:-$NYRA_BIN}"

EXAMPLE="$ROOT/examples/stdlib_priority_smoke.ny"
log() { echo "stdlib-priority-smoke: $*" >&2; }
fail() {
  local label="$1"
  local detail="${2:-}"
  log "FAILED: $label"
  ta_fail "$label" "$detail" || exit 1
}

log "check $EXAMPLE"
$NYRA check "$EXAMPLE"

log "run"
combined_file="$(mktemp)"
out="" ec=0
"$NYRA" run "$EXAMPLE" >"$combined_file" 2>&1 || ec=$?
combined="$(cat "$combined_file")"
if ((ec == 0)); then
  out="$combined"
else
  fail "run $EXAMPLE" "$combined"
fi
if ((ec == 0)); then
  if [[ "${NYRA_TEST_ALL:-}" != "1" ]]; then
    printf '%s\n' "$out"
  fi
  echo "$out" | grep -q "42" || fail "missing atoi output" "$out"
  echo "$out" | grep -q "99" || fail "missing itoa output" "$out"
fi
rm -f "$combined_file"

ta_finish "stdlib-priority-smoke"
log "ok"
