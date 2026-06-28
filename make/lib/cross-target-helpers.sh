#!/usr/bin/env bash
# Shared helpers for cross-target smoke tests (linux / windows artifact paths).
set -euo pipefail

# Find a built binary under target/{profile}/ or target/{triple}/{profile}/.
# Usage: cross_find_artifact <project_dir> <profile> <basename> [extra_triple ...]
cross_find_artifact() {
  local project_dir="$1"
  local profile="$2"
  local basename="$3"
  shift 3
  local -a triples=(
    ""
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
    "x86_64-pc-windows-gnu"
    "aarch64-pc-windows-gnu"
    "wasm32-wasip1"
  )
  triples+=("$@")
  local triple path
  for triple in "${triples[@]}"; do
    if [[ -z "$triple" ]]; then
      path="$project_dir/target/$profile/$basename"
    else
      path="$project_dir/target/$triple/$profile/$basename"
    fi
    if [[ -f "$path" ]]; then
      printf '%s\n' "$path"
      return 0
    fi
  done
  return 1
}

cross_windows_linker_ready() {
  command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1 \
    || command -v x86_64-w64-mingw32-clang >/dev/null 2>&1 \
    || command -v x86_64-w64-mingw32-g++ >/dev/null 2>&1
}

cross_linux_linker_ready() {
  # Native Linux hosts can build --for linux without a separate cross linker.
  case "$(uname -s 2>/dev/null || echo unknown)" in
    Linux) return 0 ;;
  esac
  # macOS/Windows: require an explicit GNU cross toolchain (host clang lacks linux sysroot).
  command -v x86_64-linux-gnu-gcc >/dev/null 2>&1 \
    || command -v aarch64-linux-gnu-gcc >/dev/null 2>&1 \
    || command -v x86_64-linux-gnu-clang >/dev/null 2>&1 \
    || command -v aarch64-linux-gnu-clang >/dev/null 2>&1
}

cross_log_skip() {
  printf 'cross-target: note: %s — skipping\n' "$*" >&2
}
