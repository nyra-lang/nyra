#!/usr/bin/env sh
# Nyra installer — curl -fsSL .../install.sh | sh
# Optional: | sh -s -- --version 0.1.0 --install-dir ~/.nyra
set -eu

REPO="${NYRA_INSTALL_REPO:-nyra-lang/nyra}"
INSTALL_DIR="${NYRA_INSTALL_DIR:-$HOME/.nyra}"
VERSION="latest"

usage() {
  cat <<'EOF'
Nyra installer

Usage:
  curl -fsSL https://raw.githubusercontent.com/nyra-lang/nyra/main/scripts/install.sh | sh
  curl -fsSL .../install.sh | sh -s -- --version 0.1.0
  curl -fsSL .../install.sh | sh -s -- --install-dir DIR

Options:
  --version VER       Release tag (0.1.0) or "latest" (default)
  --install-dir DIR   Install root (default: ~/.nyra)
  --with-toolchain    Run nyra toolchain install (LLVM under lib/llvm)
  --help              Show this help

Requires: curl, tar (clang optional if --with-toolchain succeeds)
EOF
}

WITH_TOOLCHAIN=0
while [ $# -gt 0 ]; do
  case "$1" in
    --version)
      VERSION="${2:?--version requires a value}"
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="${2:?--install-dir requires a value}"
      shift 2
      ;;
    --with-toolchain)
      WITH_TOOLCHAIN=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

die() {
  echo "error: $*" >&2
  exit 1
}

info() {
  echo "$*"
}

if ! command -v clang >/dev/null 2>&1; then
  if [ "$WITH_TOOLCHAIN" -eq 0 ] && [ ! -x "${INSTALL_DIR}/lib/llvm/bin/clang" ]; then
    die "clang not found.

Install a C toolchain:
  macOS:  xcode-select --install  OR  brew install llvm && nyra toolchain install
  Debian/Ubuntu:  sudo apt install clang
  Fedora:  sudo dnf install clang

Or re-run with --with-toolchain after installing LLVM (brew install llvm)."
  fi
fi

if ! command -v curl >/dev/null 2>&1; then
  die "curl is required"
fi

if ! command -v tar >/dev/null 2>&1; then
  die "tar is required"
fi

OS="$(uname -s 2>/dev/null || true)"
ARCH="$(uname -m 2>/dev/null || true)"

case "$OS" in
  Darwin) PLATFORM="darwin" ;;
  Linux) PLATFORM="linux" ;;
  *)
    die "unsupported OS: $OS (Linux and macOS only)"
    ;;
esac

case "$ARCH" in
  x86_64|amd64) ARCH="x86_64" ;;
  arm64|aarch64) ARCH="aarch64" ;;
  *)
    die "unsupported CPU: $ARCH (need x86_64 or aarch64)"
    ;;
esac

ASSET="nyra-${ARCH}-${PLATFORM}.tar.gz"
API="https://api.github.com/repos/${REPO}/releases"

if [ "$VERSION" = "latest" ]; then
  RELEASE_JSON="$(curl -sSL "${API}/latest")"
  if printf '%s' "$RELEASE_JSON" | grep -q '"message": "Not Found"'; then
    RELEASE_LIST="$(curl -fsSL "${API}?per_page=1")"
    RELEASE_JSON="$(printf '%s' "$RELEASE_LIST" | sed 's/^\[//;s/\]$//')"
    if [ -z "$RELEASE_JSON" ] || [ "$RELEASE_JSON" = "null" ]; then
      die "no GitHub releases found for ${REPO}

Create a release and attach ${ASSET}, or pass --version matching an existing tag."
    fi
    info "note: no published 'latest' release — using newest tag (pre-release is OK)"
  fi
else
  TAG="v${VERSION#v}"
  RELEASE_JSON="$(curl -fsSL "${API}/tags/${TAG}")"
fi

# تم تحسين الـ Regex هنا لالتقاط رابط الـ Asset بدقة بدون مشاكل الـ Parsing
ASSET_URL="$(printf '%s\n' "$RELEASE_JSON" | grep 'browser_download_url' | grep "${ASSET}" \
  | sed -E 's/.*"browser_download_url": *"(.*)"/\1/' | head -n 1)"

if [ -z "$ASSET_URL" ]; then
  die "release asset not found: ${ASSET}

Push a tag (e.g. v0.1.0) and wait for the Release workflow, or pass --version matching an existing release."
fi

TMP="$(mktemp -d "${TMPDIR:-/tmp}/nyra-install.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT INT TERM

info "Downloading ${ASSET} ..."
curl -fsSL -o "$TMP/$ASSET" "$ASSET_URL"

# تم تحسين الـ Regex هنا أيضاً لالتقاط الـ SHA256SUMS
SUMS_URL="$(printf '%s\n' "$RELEASE_JSON" | grep 'browser_download_url' | grep 'SHA256SUMS' \
  | sed -E 's/.*"browser_download_url": *"(.*)"/\1/' | head -n 1)"

if [ -n "$SUMS_URL" ]; then
  curl -fsSL -o "$TMP/SHA256SUMS" "$SUMS_URL"
  (cd "$TMP" && {
    if command -v sha256sum >/dev/null 2>&1; then
      grep -F " $ASSET" SHA256SUMS | sha256sum -c -
    elif command -v shasum >/dev/null 2>&1; then
      grep -F " $ASSET" SHA256SUMS | shasum -a 256 -c -
    else
      info "note: no sha256sum/shasum; skipping checksum verify"
    fi
  })
fi

mkdir -p "$INSTALL_DIR"
tar -xzf "$TMP/$ASSET" -C "$INSTALL_DIR"

if [ ! -x "$INSTALL_DIR/bin/nyra" ]; then
  die "install failed: $INSTALL_DIR/bin/nyra missing"
fi

export NYRA_HOME="$INSTALL_DIR"
if [ "$WITH_TOOLCHAIN" -eq 1 ]; then
  info "Installing native LLVM toolchain under $INSTALL_DIR/lib/llvm ..."
  "$INSTALL_DIR/bin/nyra" toolchain install --wasi || info "note: toolchain install skipped (install llvm manually)"
fi

# Shell profile: PATH + NYRA_HOME (idempotent)
append_profile() {
  profile="$1"
  [ -f "$profile" ] || return 0
  if grep -q 'NYRA_HOME=' "$profile" 2>/dev/null && grep -q '.nyra/bin' "$profile" 2>/dev/null; then
    return 0
  fi
  {
    echo ''
    echo '# Nyra (install.sh)'
    echo "export NYRA_HOME=\"${INSTALL_DIR}\""
    echo "export PATH=\"\${NYRA_HOME}/bin:\${PATH}\""
    if [ -f "${INSTALL_DIR}/env" ]; then
      echo '[ -f "${NYRA_HOME}/env" ] && . "${NYRA_HOME}/env"'
    elif [ -d "${INSTALL_DIR}/lib/llvm/bin" ]; then
      echo 'export NYRA_LLVM_BIN="${NYRA_HOME}/lib/llvm/bin"'
      echo 'export PATH="${NYRA_LLVM_BIN}:${PATH}"'
    fi
  } >> "$profile"
  info "Updated $profile — run: source $profile"
}

case "${SHELL:-}" in
  */zsh) append_profile "$HOME/.zshrc" ;;
  */bash) append_profile "$HOME/.bashrc" ;;
esac
append_profile "$HOME/.zshrc"
append_profile "$HOME/.bashrc"

info ""
info "Nyra installed to $INSTALL_DIR"
"$INSTALL_DIR/bin/nyra" --version
info ""
info "Next: source your shell profile (or open a new terminal), then:"
info "  nyra toolchain install --wasi   # optional: bundled LLVM layout under ~/.nyra/lib/llvm"
info "  mkdir myapp && cd myapp && nyra pkg init"