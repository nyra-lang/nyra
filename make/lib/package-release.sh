#!/usr/bin/env sh
# Package a release archive for GitHub Releases.
set -eu

VERSION="${1:?version required (e.g. 0.1.0)}"
TRIPLE="${2:?target triple required (e.g. x86_64-unknown-linux-gnu)}"

# التعديل الصحيح: السكريبت موجود في make/lib/package-release.sh
# للرجوع إلى الـ Root (مجلد nyra)، نحتاج نطلع خطوتين لورا: من lib لـ make ومن make لـ nyra
ROOT="$(cd -- "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

read_workspace_version() {
  sed -n '/^\[workspace\.package\]/,/^\[/p' Cargo.toml | sed -n 's/^version = "\(.*\)"/\1/p' | head -1
}

ARCH="${TRIPLE%%-*}"
OS="${TRIPLE#*-}"
OS="${OS%%-*}"

case "$OS" in
  unknown) OS="linux" ;;
  apple) OS="darwin" ;;
  pc-windows) OS="windows" ;;
esac

IS_WINDOWS=0
if [ "$OS" = "windows" ]; then
  IS_WINDOWS=1
  ASSET="nyra-${ARCH}-windows.zip"
else
  ASSET="nyra-${ARCH}-${OS}.tar.gz"
fi

STAGE="$(mktemp -d "${TMPDIR:-/tmp}/nyra-pkg.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT INT TERM

mkdir -p "$STAGE/bin" "$STAGE/share/stdlib"

WORKSPACE_VERSION="$(read_workspace_version)"
if [ -z "$WORKSPACE_VERSION" ]; then
  echo "error: could not read [workspace.package] version from Cargo.toml" >&2
  exit 1
fi
if [ "$VERSION" != "$WORKSPACE_VERSION" ]; then
  echo "error: release VERSION=$VERSION does not match Cargo.toml ($WORKSPACE_VERSION)" >&2
  echo "hint: tag the release as v${WORKSPACE_VERSION} (or update Cargo.toml first)" >&2
  exit 1
fi

echo "Packaging Nyra $VERSION for $TRIPLE ..."

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"

# Nyra links user programs with MinGW; TLS staticlibs must match that triple.
NYRA_LINK_TRIPLE="$TRIPLE"
TLS_CARGO_TRIPLE="$TRIPLE"
if [ "$IS_WINDOWS" -eq 1 ]; then
  NYRA_LINK_TRIPLE="x86_64-pc-windows-gnu"
  if [ "$HOST_TRIPLE" != "$NYRA_LINK_TRIPLE" ]; then
    TLS_CARGO_TRIPLE="$NYRA_LINK_TRIPLE"
  fi
fi

find_staticlib() {
  lib="$1"
  triple="${2:-$TLS_CARGO_TRIPLE}"
  if [ "$triple" = "$HOST_TRIPLE" ]; then
    search_dir="target/release"
  else
    search_dir="target/$triple/release"
  fi
  for name in "lib${lib}.a" "${lib}.lib"; do
    if [ -f "$search_dir/$name" ]; then
      printf '%s' "$search_dir/$name"
      return 0
    fi
  done
  return 1
}

prebuilt_tls_name() {
  if [ "$NYRA_LINK_TRIPLE" != "${NYRA_LINK_TRIPLE%-windows-gnu}" ]; then
    printf '%s' "libnyra_rt_tls.a"
  elif [ "$IS_WINDOWS" -eq 1 ]; then
    printf '%s' "nyra_rt_tls.lib"
  else
    printf '%s' "libnyra_rt_tls.a"
  fi
}

prebuilt_tls_native_name() {
  if [ "$NYRA_LINK_TRIPLE" != "${NYRA_LINK_TRIPLE%-windows-gnu}" ]; then
    printf '%s' "libnyra_rt_tls_native.a"
  elif [ "$IS_WINDOWS" -eq 1 ]; then
    printf '%s' "nyra_rt_tls_native.lib"
  else
    printf '%s' "libnyra_rt_tls_native.a"
  fi
}

if [ "$TRIPLE" = "$HOST_TRIPLE" ]; then
  cargo build --release -p cli
  if [ "$IS_WINDOWS" -eq 1 ]; then
    cp "target/release/nyra.exe" "$STAGE/bin/nyra.exe"
  else
    cp "target/release/nyra" "$STAGE/bin/nyra"
  fi
else
  rustup target add "$TRIPLE" 2>/dev/null || true
  cargo build --release -p cli --target "$TRIPLE"
  if [ "$IS_WINDOWS" -eq 1 ]; then
    cp "target/$TRIPLE/release/nyra.exe" "$STAGE/bin/nyra.exe"
  else
    cp "target/$TRIPLE/release/nyra" "$STAGE/bin/nyra"
  fi
fi

if [ "$TLS_CARGO_TRIPLE" = "$HOST_TRIPLE" ]; then
  cargo build --release -p nyra-rt-tls -p nyra-rt-tls-native
else
  rustup target add "$TLS_CARGO_TRIPLE" 2>/dev/null || true
  cargo build --release -p nyra-rt-tls -p nyra-rt-tls-native --target "$TLS_CARGO_TRIPLE"
fi

TLS_LIB="$(find_staticlib nyra_rt_tls)" || true
TLS_NATIVE_LIB="$(find_staticlib nyra_rt_tls_native)" || true

echo "Copying stdlib (full tree) ..."
cp -R stdlib/. "$STAGE/share/stdlib/"
rm -rf "$STAGE/share/stdlib/target" 2>/dev/null || true

# Ship prebuilt rustls + native TLS clients.
if [ -n "$TLS_LIB" ]; then
  mkdir -p "$STAGE/share/stdlib/prebuilt/$NYRA_LINK_TRIPLE"
  cp "$TLS_LIB" "$STAGE/share/stdlib/prebuilt/$NYRA_LINK_TRIPLE/$(prebuilt_tls_name)"
  echo "Bundled $(prebuilt_tls_name) for $NYRA_LINK_TRIPLE"
else
  echo "error: missing nyra_rt_tls staticlib — HTTPS client would not work offline" >&2
  exit 1
fi
if [ -n "$TLS_NATIVE_LIB" ]; then
  mkdir -p "$STAGE/share/stdlib/prebuilt/$NYRA_LINK_TRIPLE"
  cp "$TLS_NATIVE_LIB" "$STAGE/share/stdlib/prebuilt/$NYRA_LINK_TRIPLE/$(prebuilt_tls_native_name)"
  echo "Bundled $(prebuilt_tls_native_name) for $NYRA_LINK_TRIPLE"
else
  echo "error: missing nyra_rt_tls_native staticlib — tls native would not work offline" >&2
  exit 1
fi

printf '%s\n' "$VERSION" > "$STAGE/version"

nyra_version_report() {
  bin="$1"
  if [ ! -x "$bin" ]; then
    return 1
  fi
  if [ "$OS" = "darwin" ]; then
    host_arch="$(uname -m)"
    case "$host_arch:$ARCH" in
      arm64:x86_64)
        arch -x86_64 "$bin" --version 2>&1 | sed -n 's/^nyra //p'
        return
        ;;
      x86_64:aarch64)
        arch -arm64 "$bin" --version 2>&1 | sed -n 's/^nyra //p'
        return
        ;;
    esac
  fi
  "$bin" --version 2>&1 | sed -n 's/^nyra //p'
}

if [ "$IS_WINDOWS" -eq 0 ]; then
  NYRA_BIN="$STAGE/bin/nyra"
  reported="$(nyra_version_report "$NYRA_BIN" | head -1)"
  if [ -z "$reported" ]; then
    reported="$(strings "$NYRA_BIN" 2>/dev/null | sed -n 's/^nyra //p' | head -1)"
  fi
  if [ -z "$reported" ]; then
    reported="$(strings "$NYRA_BIN" 2>/dev/null | grep -E "^${VERSION}\$" | head -1)"
  fi
  if [ "$reported" != "$VERSION" ]; then
    echo "error: built nyra reports ${reported:-<empty>}, expected ${VERSION}" >&2
    if [ "$OS" = "darwin" ] && [ -z "$reported" ]; then
      echo "hint: on macOS, install LLVM (libclang) so the staged nyra binary can run — see release.yml" >&2
    fi
    exit 1
  fi
fi

# Shell env helper (Unix)
cat > "$STAGE/env" <<EOF
# Nyra release $VERSION — source this file or copy into your profile
export NYRA_HOME="\$(CDPATH= cd -- "\$(dirname "\${BASH_SOURCE[0]:-\$0}")" && pwd)"
export PATH="\${NYRA_HOME}/bin:\${PATH}"
EOF

# PowerShell env helper (Windows)
cat > "$STAGE/env.ps1" <<'EOF'
# Nyra release — dot-source: . "$env:USERPROFILE\.nyra\env.ps1"
$NyraHome = Split-Path -Parent $MyInvocation.MyCommand.Path
$env:NYRA_HOME = $NyraHome
$env:PATH = "$NyraHome\bin;$env:PATH"
EOF

mkdir -p dist
if [ "$IS_WINDOWS" -eq 1 ]; then
  if command -v zip >/dev/null 2>&1; then
    (cd "$STAGE" && zip -r "$ROOT/dist/$ASSET" bin share version env.ps1)
  else
    powershell.exe -NoProfile -Command "Compress-Archive -Path '${STAGE}\\*' -DestinationPath '${ROOT}\\dist\\${ASSET}' -Force"
  fi
else
  # التأكد من استخدام $ROOT المطلق لعدم حدوث تداخل أثناء الضغط من مجلد الـ STAGE
  tar -czf "$ROOT/dist/$ASSET" -C "$STAGE" bin share version env
fi
echo "Wrote dist/$ASSET"