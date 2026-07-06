#!/usr/bin/env bash
# Fast pre-flight (~1–3 min) before make test-all — catches frequent failures early.
# Full coverage still requires: make test-all
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

log() { echo "preflight: $*"; }
fail() { log "FAIL — $*"; exit 1; }

log "root: $ROOT"
log "building cli (incremental)"
cargo build -q -p cli

# shellcheck source=nyra-bin.sh
source "$ROOT/make/lib/nyra-bin.sh"
nyra_export_cli

log "nyra check hello.ny"
"$NYRA_BIN" check examples/syntax/hello.ny >/dev/null

log "parser regression (fuzz OOM guard)"
cargo test -q -p parser fuzz_slow_unit_caps_parse_errors

log "contrib-dev Python tooling"
make -s test-contrib-py

log "abi roundtrip (cdylib + rust host)"
cargo run --quiet -p cli -- build \
  "$ROOT/examples/ffi/export_greet/main.ny" \
  -o libnyra_greet \
  --cdylib
cargo run --quiet --manifest-path "$ROOT/examples/ffi/export_greet/rust_host/Cargo.toml"

if command -v cargo-fuzz >/dev/null 2>&1; then
  artifact="$ROOT/fuzz/artifacts/fuzz_parser/slow-unit-01b56f82106b663c1bd956e31560f239c5928738"
  if [[ -f "$artifact" ]]; then
    log "fuzz_parser artifact (single run)"
    (cd "$ROOT/fuzz" && cargo +nightly fuzz run fuzz_parser --sanitizer address -- \
      -runs=1 "$artifact" 2>/dev/null) || true
  fi
else
  log "cargo-fuzz not installed — skipping fuzz artifact probe"
fi

log "ok — run make test-all for the full suite"
