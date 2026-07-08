#!/usr/bin/env bash
# Compile/run Nyra examples in easy (plain) and typed (.typed.ny) forms.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
export NYRA_TEST_STATS_FILE="${NYRA_TEST_STATS_FILE:-$ROOT/target/.nyra-test-all-stats}"
# shellcheck source=test-stats.sh
source "$ROOT/make/lib/test-stats.sh"
# shellcheck source=nyra-bin.sh
source "$ROOT/make/lib/nyra-bin.sh"
nyra_export_cli
NYRA=("$NYRA_BIN")

log() { echo "optional-types: $*" >&2; }
fail() { log "FAILED: $*"; exit 1; }

check_file() {
  local label="$1"
  local path="$2"
  if nyra_stats_check "$path"; then
    log "ok — check $label"
  else
    fail "check $label ($path)"
  fi
}

normalize_text_out() {
  # Windows runners emit CRLF from print(); normalize before comparing expected output.
  printf '%s' "$1" | tr -d '\r'
}

run_file() {
  local label="$1"
  local path="$2"
  local expect="${3:-}"
  local out="" err_file="" ec=0
  err_file="$(mktemp)"
  out="$(normalize_text_out "$("${NYRA[@]}" run "$path" 2>"$err_file")")" || ec=$?
  if ((ec != 0)); then
    fail "run $label (exit $ec): $(tr -d '\r' <"$err_file")"
  fi
  if [[ -n "$expect" ]]; then
    expect="$(normalize_text_out "$expect")"
    if [[ "$out" != "$expect" ]]; then
      fail "run $label: expected $(printf %q "$expect") got $(printf %q "$out")"
    fi
  fi
  log "ok — run $label"
  nyra_stats_pass
}

log "sync typed example siblings"
python3 "$ROOT/make/py/gen-typed-examples.py"

log "checking builtins (easy + typed)"
while IFS= read -r -d '' plain; do
  rel="${plain#$ROOT/}"
  check_file "$rel" "$plain"
  typed="${plain%.ny}.typed.ny"
  if [[ -f "$typed" ]]; then
    check_file "${rel%.ny}.typed.ny" "$typed"
  fi
done < <(find "$ROOT/examples/builtins" -type f -name '*.ny' ! -name '*.typed.ny' -print0)

log "checking syntax examples (easy + typed)"
while IFS= read -r -d '' plain; do
  rel="${plain#$ROOT/}"
  check_file "$rel" "$plain"
  typed="${plain%.ny}.typed.ny"
  if [[ -f "$typed" ]]; then
    check_file "${rel%.ny}.typed.ny" "$typed"
  fi
done < <(find "$ROOT/examples/syntax" -type f -name '*.ny' ! -name '*.typed.ny' -print0)

# Spot-run a few deterministic builtins in both styles
run_file "strings/split.ny" "$ROOT/examples/builtins/strings/split.ny" $'3\na\nb\nc'
run_file "strings/split.typed.ny" "$ROOT/examples/builtins/strings/split.typed.ny" $'3\na\nb\nc'
run_file "arrays/sort.ny" "$ROOT/examples/builtins/arrays/sort.ny" $'1\n2\n5\n8\n10\n10'
run_file "arrays/sort.typed.ny" "$ROOT/examples/builtins/arrays/sort.typed.ny" $'1\n2\n5\n8\n10\n10'

log "done"
