#!/usr/bin/env bash
# Progress reporting for make test-all (ASCII-only, no emojis).
set -euo pipefail

case "${1:-}" in
  now)
    date '+%Y-%m-%d %H:%M:%S'
    exit 0
    ;;
  total)
    # gate count only — no progress file yet
    ;;
  *)
    : "${NYRA_TEST_ALL_PROGRESS_FILE:?NYRA_TEST_ALL_PROGRESS_FILE required}"
    ;;
esac

PROGRESS_WIDTH="${NYRA_TEST_ALL_PROGRESS_WIDTH:-28}"
TEST_ALL_LOG="${TEST_ALL_LOG:-}"

nyra_progress_log() {
  printf '%s\n' "$1"
  if [[ -n "$TEST_ALL_LOG" ]]; then
    printf '%s\n' "$1" >>"$TEST_ALL_LOG"
  fi
}

nyra_progress_bar() {
  local cur="$1" total="$2"
  local width="$PROGRESS_WIDTH"
  local filled=0 empty=0 pct=0
  local i

  if (( total > 0 )); then
    filled=$(( cur * width / total ))
    pct=$(( cur * 100 / total ))
  fi
  empty=$(( width - filled ))

  printf '['
  for ((i = 0; i < filled; i++)); do printf '#'; done
  for ((i = 0; i < empty; i++)); do printf '-'; done
  printf '] %3d%%  (%s/%s)' "$pct" "$cur" "$total"
}

nyra_progress_gate_list() {
  if [[ "${NYRA_PROGRESS_PROFILE:-}" == "platform" ]]; then
    cat <<'GATES'
build-workspace
test-cargo-workspace
test-conformance
test-nyra-lang
test-optional-types
smoke-stdlib
smoke-stdlib-runtime
smoke-stdlib-priority
test-runtime-smoke
platform-native-smoke
GATES
    return
  fi
  cat <<'GATES'
build-workspace
build-cli
test-count
test-webdocs-tabs
test-webdocs-snippets
smoke-vscode-extension
test-optional-types
test-comparison-parity
test-cargo-workspace
test-nyra-lang
test-runtime-smoke
smoke-cli
smoke-apps
smoke-sqlite
smoke-database
smoke-serde-pkg
test-conformance
smoke-corpus
smoke-examples
smoke-stdlib
smoke-stdlib-priority
smoke-stdlib-medium
smoke-stdlib-runtime
test-compiletest
test-fuzz-smoke
gen-abi-header
abi-roundtrip-cdylib
abi-roundtrip-rust-host
smoke-cross-wasm
smoke-cross-linux
smoke-cross-windows
test-fuzz-stress
GATES
  if [[ "${TEST_PERF:-}" == "1" ]]; then
    printf '%s\n' test-perf
  fi
  if [[ "${TEST_SAN:-}" == "1" ]]; then
    printf '%s\n' test-sanitizer
    printf '%s\n' test-race-tsan
    printf '%s\n' test-race-native
  fi
  if [[ "${TEST_FUZZ:-}" == "1" ]]; then
    printf '%s\n' test-fuzz-nightly
  fi
}

nyra_progress_total() {
  nyra_progress_gate_list | wc -l | tr -d ' '
}

nyra_progress_init() {
  local root="${NYRA_TEST_ALL_PROGRESS_FILE%/*}"
  local total started
  if [[ -n "$root" && "$root" != "$NYRA_TEST_ALL_PROGRESS_FILE" ]]; then
    mkdir -p "$root"
  fi
  total="$(nyra_progress_total)"
  started="$(date '+%Y-%m-%d %H:%M:%S')"
  printf '%s\n' "0 $total 0 $started" >"$NYRA_TEST_ALL_PROGRESS_FILE"
}

nyra_progress_ensure() {
  if [[ ! -f "$NYRA_TEST_ALL_PROGRESS_FILE" ]]; then
    nyra_progress_init
  fi
}

nyra_progress_read() {
  nyra_progress_ensure
  # shellcheck disable=SC2034
  read -r NYRA_PROGRESS_CURRENT NYRA_PROGRESS_TOTAL NYRA_PROGRESS_FAILED NYRA_PROGRESS_STARTED \
    <"$NYRA_TEST_ALL_PROGRESS_FILE"
}

nyra_progress_write() {
  printf '%s %s %s %s\n' \
    "${NYRA_PROGRESS_CURRENT:-0}" \
    "${NYRA_PROGRESS_TOTAL:-0}" \
    "${NYRA_PROGRESS_FAILED:-0}" \
    "${NYRA_PROGRESS_STARTED:-}" \
    >"$NYRA_TEST_ALL_PROGRESS_FILE"
}

nyra_progress_header() {
  local root="${1:-}"
  local total
  total="$(nyra_progress_total)"
  if [[ "${NYRA_PROGRESS_PROFILE:-}" == "platform" ]]; then
    nyra_progress_log '+----------------------------------------------------------+'
    nyra_progress_log '|  NYRA PLATFORM TEST SUITE (macOS / Windows CI)           |'
    nyra_progress_log '+----------------------------------------------------------+'
  else
    nyra_progress_log '+----------------------------------------------------------+'
    nyra_progress_log '|  NYRA TEST SUITE                                         |'
    nyra_progress_log '+----------------------------------------------------------+'
  fi
  if [[ -n "$root" ]]; then
    nyra_progress_log "  root:  $root"
  fi
  nyra_progress_log "  gates: $total"
}

nyra_progress_phase() {
  local name="$1"
  nyra_progress_read
  nyra_progress_log ""
  nyra_progress_log "$(nyra_progress_bar "$NYRA_PROGRESS_CURRENT" "$NYRA_PROGRESS_TOTAL")"
  nyra_progress_log "  \\-- phase: $name"
}

nyra_progress_begin() {
  local label="$1"
  local bar line
  nyra_progress_read
  NYRA_PROGRESS_CURRENT=$((NYRA_PROGRESS_CURRENT + 1))
  nyra_progress_write
  bar="$(nyra_progress_bar "$NYRA_PROGRESS_CURRENT" "$NYRA_PROGRESS_TOTAL")"
  line="$bar  >> $label"
  nyra_progress_log ""
  nyra_progress_log "$line"
  nyra_progress_log "      started $(date '+%H:%M:%S')"
}

nyra_progress_end() {
  local label="$1"
  local status="$2"
  local bar mark line
  nyra_progress_read
  bar="$(nyra_progress_bar "$NYRA_PROGRESS_CURRENT" "$NYRA_PROGRESS_TOTAL")"
  case "$status" in
    ok|OK|pass)
      mark="OK "
      ;;
    *)
      mark="FAIL"
      NYRA_PROGRESS_FAILED=$((NYRA_PROGRESS_FAILED + 1))
      nyra_progress_write
      ;;
  esac
  line="$bar  [$mark] $label"
  nyra_progress_log "$line"
  local remaining=$((NYRA_PROGRESS_TOTAL - NYRA_PROGRESS_CURRENT))
  if (( remaining > 0 )); then
    nyra_progress_log "      remaining gates: $remaining"
  fi
}

nyra_progress_sub() {
  local parent="$1"
  local cur="$2"
  local total="$3"
  local detail="${4:-}"
  local width=16
  local filled=0
  local empty=0
  local bar

  if (( total > 0 )); then
    filled=$(( cur * width / total ))
  fi
  empty=$(( width - filled ))

  bar='['
  for ((i = 0; i < filled; i++)); do bar+='#'; done
  for ((i = filled; i < width; i++)); do bar+='-'; done
  bar+=']'

  if [[ -n "$detail" ]]; then
    nyra_progress_log "      |-- $bar $cur/$total  $detail"
  else
    nyra_progress_log "      |-- $bar $cur/$total  $parent"
  fi
}

nyra_progress_summary_line() {
  nyra_progress_read
  nyra_progress_log ""
  nyra_progress_log "$(nyra_progress_bar "$NYRA_PROGRESS_CURRENT" "$NYRA_PROGRESS_TOTAL")  finished"
  nyra_progress_log "  gates run: $NYRA_PROGRESS_CURRENT / $NYRA_PROGRESS_TOTAL"
  nyra_progress_log "  gate failures: $NYRA_PROGRESS_FAILED"
  if [[ -n "$NYRA_PROGRESS_STARTED" ]]; then
    nyra_progress_log "  started: $NYRA_PROGRESS_STARTED"
  fi
  nyra_progress_log "  ended:   $(date '+%Y-%m-%d %H:%M:%S')"
}

case "${1:-}" in
  init)
    nyra_progress_init
    nyra_progress_header "${ROOT:-}"
    ;;
  phase) shift; nyra_progress_phase "$*" ;;
  begin) shift; nyra_progress_begin "$*" ;;
  end) shift; nyra_progress_end "$1" "${2:-ok}" ;;
  sub) shift; nyra_progress_sub "$@" ;;
  summary) nyra_progress_summary_line ;;
  total) printf '%s\n' "$(nyra_progress_total)" ;;
  *)
    printf 'usage: %s init|phase|begin|end|sub|summary|now|total\n' "$0" >&2
    exit 2
    ;;
esac
