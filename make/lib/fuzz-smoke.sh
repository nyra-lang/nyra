#!/usr/bin/env bash
# Short fuzz smoke (requires `cargo install cargo-fuzz`).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PROGRESS="$ROOT/make/lib/test-all-progress.sh"

fuzz_progress() {
  local cur="$1"
  local total="$2"
  local detail="$3"
  if [[ -n "${NYRA_TEST_ALL_PROGRESS_FILE:-}" ]]; then
    NYRA_TEST_ALL_PROGRESS_FILE="$NYRA_TEST_ALL_PROGRESS_FILE" \
      TEST_ALL_LOG="${TEST_ALL_LOG:-}" \
      "$PROGRESS" sub "fuzz smoke" "$cur" "$total" "$detail"
  else
    printf 'fuzz-smoke: [%s/%s] %s\n' "$cur" "$total" "$detail"
  fi
}

if ! command -v cargo-fuzz >/dev/null 2>&1; then
  echo "fuzz-smoke: cargo-fuzz not installed — skipping (install with: cargo install cargo-fuzz)"
  exit 0
fi

bash "$ROOT/make/lib/sync-fuzz-corpus.sh"

fuzz_sanitizer() {
  local mode="${NYRA_FUZZ_SANITIZER:-auto}"
  if [[ "$mode" != "auto" ]]; then
    echo "$mode"
    return
  fi
  if rustup run nightly rustc -V >/dev/null 2>&1; then
    echo "address"
  else
    echo "none"
  fi
}

fuzz_extra_args() {
  local target="$1"
  local -a args=()
  local dict="$ROOT/fuzz/dictionaries/nyra.dict"
  if [[ -f "$dict" ]]; then
    args+=(-dict="$dict")
  fi
  args+=(-max_len=16384)
  if [[ "$target" == "fuzz_codegen" ]]; then
    args+=(-max_total_time=45 -rss_limit_mb=4096)
  else
    args+=(-max_total_time=60 -rss_limit_mb=2048)
  fi
  printf '%s\n' "${args[@]}"
}

SAN="$(fuzz_sanitizer)"
if [[ "$SAN" == "none" ]]; then
  echo "fuzz-smoke: nightly unavailable — using --sanitizer none (install: rustup install nightly)"
fi

fuzz_cargo() {
  if [[ "$SAN" == "none" ]]; then
    cargo fuzz "$@"
  else
    cargo +nightly fuzz "$@"
  fi
}

cd "$ROOT/fuzz"
targets=(fuzz_lexer fuzz_parser fuzz_compile fuzz_gen fuzz_codegen)
total="${#targets[@]}"
idx=0
for target in "${targets[@]}"; do
  idx=$((idx + 1))
  fuzz_progress "$idx" "$total" "$target (sanitizer=$SAN)"
  # shellcheck disable=SC2046
  fuzz_cargo run "$target" --sanitizer "$SAN" -- $(fuzz_extra_args "$target")
done
echo "fuzz-smoke: ok"
