#!/usr/bin/env bash
# Runtime smoke for stdlib document JSON + ny-toml (rust::toml bridge).
# ny-serde graduated into stdlib/json — no rust::serde_json bind required.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
export NYRA_TEST_STATS_FILE="${NYRA_TEST_STATS_FILE:-$ROOT/target/.nyra-test-all-stats}"
# shellcheck source=test-stats.sh
source "$ROOT/make/lib/test-stats.sh"

log() { echo "serde-pkg-smoke: $*" >&2; }
fail() { log "FAILED: $*"; exit 1; }

# shellcheck source=nyra-bin.sh
source "$ROOT/make/lib/nyra-bin.sh"
nyra_export_cli
NYRA=("$NYRA_BIN")
SERDE_PKG="$ROOT/examples/packages/ny-serde"
TOML_PKG="$ROOT/examples/packages/ny-toml"
EXAMPLE_PROJ="$ROOT/examples/serde_json_pkg"

log "stdlib json document test"
if ! out="$("${NYRA[@]}" run "$ROOT/tests/suite/run/stdlib/json_document.ny" 2>&1)"; then
  printf '%s\n' "$out" >&2
  fail "run json_document.ny"
fi
if ! printf '%s\n' "$out" | grep -q '^ok$'; then
  printf '%s\n' "$out" >&2
  fail "json_document.ny expected ok"
fi
nyra_stats_pass

log "ny-serde shim package tests (no bind)"
if ! out="$("${NYRA[@]}" test "$SERDE_PKG/serde_test.ny" 2>&1)"; then
  printf '%s\n' "$out" >&2
  fail "ny-serde serde_test.ny"
fi
printf '%s\n' "$out" >&2
if ! printf '%s\n' "$out" | grep -q 'tests passed'; then
  fail "ny-serde (no tests passed line)"
fi
nyra_stats_pass

log "bind rust toml --template ($TOML_PKG)"
if ! "${NYRA[@]}" bind rust toml --template --project "$TOML_PKG" >/dev/null 2>&1; then
  fail "bind rust toml --template"
fi
log "nyra test $TOML_PKG/toml_test.ny"
if ! out="$("${NYRA[@]}" test "$TOML_PKG/toml_test.ny" 2>&1)"; then
  printf '%s\n' "$out" >&2
  fail "nyra test toml_test.ny"
fi
printf '%s\n' "$out" >&2
if ! printf '%s\n' "$out" | grep -q 'tests passed'; then
  fail "toml_test.ny (no tests passed line)"
fi
nyra_stats_pass

log "nyra run examples/serde_json_pkg/main.ny"
out="$("${NYRA[@]}" run "$EXAMPLE_PROJ/main.ny" 2>&1 | grep -E '^\{' | tail -1 || true)"
if [[ "$out" != '{"lang":"nyra","version":1}' ]]; then
  printf '%s\n' "$out" >&2
  fail "serde_json_pkg example (expected compact JSON, got: $(printf %q "$out"))"
fi
nyra_stats_pass

log "ok — serde package smoke"
