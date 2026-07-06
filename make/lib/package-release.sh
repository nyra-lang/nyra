#!/usr/bin/env sh
# Package a release archive for GitHub Releases.
set -eu

VERSION="${1:?version required (e.g. 0.1.0)}"
TRIPLE="${2:?target triple required (e.g. x86_64-unknown-linux-gnu)}"

# التعديل الصحيح: السكريبت موجود في make/lib/package-release.sh
# للرجوع إلى الـ Root (مجلد nyra)، نحتاج نطلع خطوتين لورا: من lib لـ make ومن make لـ nyra
ROOT="$(cd -- "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

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

sync_workspace_version() {
  if ! command -v python3 >/dev/null 2>&1; then
    echo "error: python3 required to sync Cargo.toml version for releases" >&2
    exit 1
  fi
  python3 - "$VERSION" <<'PY'
import re
import sys
from pathlib import Path

version = sys.argv[1]
path = Path("Cargo.toml")
text = path.read_text()
pattern = r'(\[workspace\.package\]\s*\n(?:[^\[]*\n)*?)version = "[^"]+"'
new, n = re.subn(pattern, rf'\1version = "{version}"', text, count=1)
if n != 1:
    sys.exit("failed to update [workspace.package] version in Cargo.toml")
path.write_text(new)
PY
}

echo "Syncing workspace version to $VERSION ..."
sync_workspace_version

echo "Building cli for $TRIPLE ..."

HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"

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

echo "Copying stdlib (full tree) ..."
cp -R stdlib/. "$STAGE/share/stdlib/"
rm -rf "$STAGE/share/stdlib/target" 2>/dev/null || true

printf '%s\n' "$VERSION" > "$STAGE/version"

if [ "$IS_WINDOWS" -eq 0 ]; then
  reported="$("$STAGE/bin/nyra" --version 2>/dev/null | sed 's/^nyra //')"
  if [ "$reported" != "$VERSION" ]; then
    echo "error: built nyra reports ${reported}, expected ${VERSION}" >&2
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