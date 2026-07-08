#!/usr/bin/env bash
# Build nyra CLI + TLS staticlibs with the correct link triple on each host.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
NYRA_LINK_TRIPLE="$HOST_TRIPLE"
TLS_CARGO_TRIPLE="$HOST_TRIPLE"

is_windows_host() {
  case "$(uname -s 2>/dev/null || echo unknown)" in
    MINGW* | MSYS* | CYGWIN*) return 0 ;;
  esac
  [ "${OS:-}" = "Windows_NT" ]
}

if is_windows_host; then
  NYRA_LINK_TRIPLE="x86_64-pc-windows-gnu"
  if [ "$HOST_TRIPLE" != "$NYRA_LINK_TRIPLE" ]; then
    TLS_CARGO_TRIPLE="$NYRA_LINK_TRIPLE"
  fi
fi

cargo build -q -p cli

if [ "$TLS_CARGO_TRIPLE" = "$HOST_TRIPLE" ]; then
  cargo build -q -p nyra-rt-tls -p nyra-rt-tls-native
else
  rustup target add "$TLS_CARGO_TRIPLE" >/dev/null 2>&1 || true
  cargo build -q -p nyra-rt-tls -p nyra-rt-tls-native --target "$TLS_CARGO_TRIPLE"
fi

install_tls_prebuilt() {
  local lib="$1"
  local dest_name="$2"
  local search_dir
  if [ "$TLS_CARGO_TRIPLE" = "$HOST_TRIPLE" ]; then
    search_dir="target/debug"
  else
    search_dir="target/$TLS_CARGO_TRIPLE/debug"
  fi
  local src=""
  if [ "$NYRA_LINK_TRIPLE" != "${NYRA_LINK_TRIPLE%-windows-gnu}" ]; then
    if [ -f "$search_dir/lib${lib}.a" ]; then
      src="$search_dir/lib${lib}.a"
    fi
  else
    for name in "lib${lib}.a" "${lib}.lib"; do
      if [ -f "$search_dir/$name" ]; then
        src="$search_dir/$name"
        break
      fi
    done
  fi
  if [ -z "$src" ]; then
    return 1
  fi
  mkdir -p "$ROOT/stdlib/prebuilt/$NYRA_LINK_TRIPLE"
  cp -f "$src" "$ROOT/stdlib/prebuilt/$NYRA_LINK_TRIPLE/$dest_name"
  # Drop stale MSVC archives when Nyra links with MinGW.
  if [ "$NYRA_LINK_TRIPLE" != "${NYRA_LINK_TRIPLE%-windows-gnu}" ]; then
    rm -f "$ROOT/stdlib/prebuilt/$NYRA_LINK_TRIPLE/${lib}.lib"
  fi
}

if [ "$NYRA_LINK_TRIPLE" != "${NYRA_LINK_TRIPLE%-windows-gnu}" ]; then
  TLS_DEST_NAME="libnyra_rt_tls.a"
  TLS_NATIVE_DEST_NAME="libnyra_rt_tls_native.a"
elif is_windows_host; then
  TLS_DEST_NAME="nyra_rt_tls.lib"
  TLS_NATIVE_DEST_NAME="nyra_rt_tls_native.lib"
else
  TLS_DEST_NAME="libnyra_rt_tls.a"
  TLS_NATIVE_DEST_NAME="libnyra_rt_tls_native.a"
fi

install_tls_prebuilt nyra_rt_tls "$TLS_DEST_NAME" || true
install_tls_prebuilt nyra_rt_tls_native "$TLS_NATIVE_DEST_NAME" || true
