#!/usr/bin/env bash
# Build smoke for Apps/Basics and Apps/Graphics (optional raylib).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
# shellcheck source=test-all-collect.sh
source "$ROOT/make/lib/test-all-collect.sh"
ta_set_scope "apps-smoke"
# shellcheck source=nyra-bin.sh
source "$ROOT/make/lib/nyra-bin.sh"
if [[ -z "${NYRA_BIN:-}" ]]; then nyra_export_cli; fi
NYRA="${NYRA:-$NYRA_BIN}"

log() { echo "apps-smoke: $*" >&2; }
fail() {
  log "FAILED: $*"
  ta_fail "$*" "" || exit 1
}

build_app() {
  local label="$1"
  local dir="$2"
  local out=""
  if [[ "${NYRA_TEST_ALL:-}" != "1" ]]; then
    log "build $label"
  fi
  if ! out="$(cd "$dir" && $NYRA build . 2>&1)"; then
    fail "$label build" "$out"
    return 0
  fi
  if [[ -n "$out" && "${NYRA_TEST_ALL:-}" != "1" ]]; then
    printf '%s\n' "$out"
  fi
  return 0
}

BASICS=(
  MergeSort
  AllSorts
  AVLTree
  BTree
  Base64
  SHA256
  AES
  RSA
  UuidGenerator
  AStar
  RedBlackTree
  Binary_Search
  Calculator
  TodoCLI
  Timer
  PasswordGenerator
  UnitConverter
  CsvReader
  UrlParser
  Dijkstra
  Graph
  Huffman
  LZW
)

for app in "${BASICS[@]}"; do
  dir="$ROOT/Apps/Basics/$app"
  if [[ ! -f "$dir/main.ny" && ! -f "$dir/nyra.mod" ]]; then
    if [[ "${NYRA_TEST_ALL:-}" != "1" ]]; then
      log "skip Basics/$app (no project)"
    fi
    continue
  fi
  build_app "Basics/$app" "$dir"
done

raylib_ok=0
for libdir in /opt/homebrew/opt/raylib/lib /usr/local/opt/raylib/lib; do
  if [[ -d "$libdir" ]]; then
    raylib_ok=1
    break
  fi
done

GRAPHICS=(
  ImageViewer
  Paint
  PhotoEditor
  RayTracer
  Renderer2D
  SpriteEngine
  ParticleEngine
  FontRenderer
  PDFViewer
)

if [[ "$raylib_ok" -eq 1 ]]; then
  for app in "${GRAPHICS[@]}"; do
    dir="$ROOT/Apps/Graphics/$app"
    build_app "Graphics/$app" "$dir"
  done
else
  log "skip Graphics/* (raylib not found — brew install raylib)"
fi

ta_finish "apps-smoke"
log "ok — Apps smoke"
