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
  --keep-dev          Keep cargo-installed nyra (~/.cargo/bin/nyra) instead of removing it
  --help              Show this help

Requires: curl, tar (clang optional if --with-toolchain succeeds)
EOF
}

WITH_TOOLCHAIN=0
KEEP_DEV=0
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
    --keep-dev)
      KEEP_DEV=1
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

warn() {
  echo "warning: $*" >&2
}

json_field() {
  field="$1"
  printf '%s' "$2" | sed -n "s/.*\"${field}\": \"\\([^\"]*\\)\".*/\\1/p" | head -n 1
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
    die "unsupported OS: $OS

Use the Windows installer in PowerShell:
  irm https://raw.githubusercontent.com/nyra-lang/nyra/main/scripts/install.ps1 | iex"
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

TAG="$(json_field tag_name "$RELEASE_JSON")"
if [ -z "$TAG" ]; then
  die "could not read release tag from GitHub API"
fi

ASSET_URL="$(printf '%s\n' "$RELEASE_JSON" | grep 'browser_download_url' | grep "${ASSET}" \
  | sed -E 's/.*"browser_download_url": *"(.*)"/\1/' | head -n 1)"

if [ -z "$ASSET_URL" ]; then
  die "release asset not found: ${ASSET} (release ${TAG})

Push a tag (e.g. v0.1.0) and wait for the Release workflow, or pass --version matching an existing release."
fi

TMP="$(mktemp -d "${TMPDIR:-/tmp}/nyra-install.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT INT TERM

info "Installing ${TAG} → ${INSTALL_DIR}"
info "Downloading ${ASSET} ..."
curl -fsSL -o "$TMP/$ASSET" "$ASSET_URL"

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

# Replace previous release payload (keep lib/llvm toolchain if present).
if [ -d "$INSTALL_DIR" ]; then
  info "Removing previous Nyra release files under ${INSTALL_DIR} ..."
fi
mkdir -p "$INSTALL_DIR"
rm -rf \
  "$INSTALL_DIR/bin" \
  "$INSTALL_DIR/share" \
  "$INSTALL_DIR/version" \
  "$INSTALL_DIR/env" \
  "$INSTALL_DIR/env.ps1"

tar -xzf "$TMP/$ASSET" -C "$INSTALL_DIR"

if [ ! -x "$INSTALL_DIR/bin/nyra" ]; then
  die "install failed: $INSTALL_DIR/bin/nyra missing"
fi

export NYRA_HOME="$INSTALL_DIR"
if [ "$WITH_TOOLCHAIN" -eq 1 ]; then
  info "Installing native LLVM toolchain under $INSTALL_DIR/lib/llvm ..."
  "$INSTALL_DIR/bin/nyra" toolchain install --wasi || info "note: toolchain install skipped (install llvm manually)"
fi

remove_cargo_nyra() {
  cargo_nyra="${HOME}/.cargo/bin/nyra"
  [ -x "$cargo_nyra" ] || return 0
  if [ "$KEEP_DEV" -eq 1 ]; then
    warn "keeping dev build at $cargo_nyra (--keep-dev); ensure ${INSTALL_DIR}/bin comes first on PATH"
    return 0
  fi
  if command -v cargo >/dev/null 2>&1; then
    info "Removing cargo-installed nyra (dev build) so the release binary is used ..."
    cargo uninstall nyra 2>/dev/null || rm -f "$cargo_nyra"
  else
    info "Removing $cargo_nyra ..."
    rm -f "$cargo_nyra"
  fi
}

remove_cargo_nyra

# Shell profile: replace any prior Nyra install.sh blocks (avoid duplicate PATH entries).
clean_profile_nyra_blocks() {
  profile="$1"
  [ -f "$profile" ] || return 0
  tmp="${profile}.nyra-install.$$"
  awk '
    /^# Nyra \(install\.sh\) BEGIN/ { skip=1; next }
    /^# Nyra \(install\.sh\) END/ { skip=0; next }
    /^# Nyra \(install\.sh\)/ { skip=1; next }
    skip && /^[[:space:]]*$/ { skip=0; next }
    skip { next }
    { print }
  ' "$profile" > "$tmp"
  mv "$tmp" "$profile"
}

write_profile_nyra_block() {
  profile="$1"
  [ -f "$profile" ] || touch "$profile"
  clean_profile_nyra_blocks "$profile"
  {
    echo ''
    echo '# Nyra (install.sh) BEGIN'
    echo "export NYRA_HOME=\"${INSTALL_DIR}\""
    echo "export PATH=\"\${NYRA_HOME}/bin:\${PATH}\""
    if [ -f "${INSTALL_DIR}/env" ]; then
      echo '[ -f "${NYRA_HOME}/env" ] && . "${NYRA_HOME}/env"'
    elif [ -d "${INSTALL_DIR}/lib/llvm/bin" ]; then
      echo 'export NYRA_LLVM_BIN="${NYRA_HOME}/lib/llvm/bin"'
      echo 'export PATH="${NYRA_LLVM_BIN}:${PATH}"'
    fi
    echo '# Nyra (install.sh) END'
  } >> "$profile"
  UPDATED_PROFILE="$profile"
}

UPDATED_PROFILE=""
case "${SHELL:-}" in
  */zsh) write_profile_nyra_block "$HOME/.zshrc" ;;
  */bash) write_profile_nyra_block "$HOME/.bashrc" ;;
  *) write_profile_nyra_block "$HOME/.zshrc" ;;
esac

print_next_steps() {
  profile="${UPDATED_PROFILE:-$HOME/.zshrc}"
  shell_reload="source ${profile}"

  info ""
  info "================================================================"
  info "  Nyra is installed — follow these steps for your system"
  info "================================================================"
  info ""

  case "$OS" in
    Darwin)
      info "macOS"
      info "----"
      info "1. Reload your shell (or open a new Terminal window):"
      info "     ${shell_reload}"
      info ""
      info "2. Verify Nyra is on PATH:"
      info "     which nyra"
      info "     nyra --version"
      info ""
      info "3. Install a C linker if you have not already (required to run programs):"
      info "     xcode-select --install"
      info "   Or with Homebrew:"
      info "     brew install llvm"
      info ""
      info "4. Optional — install LLVM/WASI under ${INSTALL_DIR}:"
      info "     nyra toolchain install --wasi"
      info ""
      info "5. Create your first project:"
      info "     mkdir myapp && cd myapp"
      info "     nyra pkg init"
      info "     nyra run ."
      ;;
    Linux)
      info "Linux"
      info "-----"
      info "1. Reload your shell (or open a new terminal):"
      info "     ${shell_reload}"
      info ""
      info "2. Verify Nyra is on PATH:"
      info "     which nyra"
      info "     nyra --version"
      info ""
      info "3. Install clang if you have not already (required to run programs):"
      info "     Debian/Ubuntu:  sudo apt update && sudo apt install -y clang"
      info "     Fedora/RHEL:    sudo dnf install -y clang"
      info "     Arch:           sudo pacman -S clang"
      info ""
      info "4. Optional — install LLVM/WASI under ${INSTALL_DIR}:"
      info "     nyra toolchain install --wasi"
      info ""
      info "5. Create your first project:"
      info "     mkdir myapp && cd myapp"
      info "     nyra pkg init"
      info "     nyra run ."
      ;;
    *)
      info "1. Reload your shell, then run: nyra --version"
      info "2. Install clang/LLVM for your OS (required to link programs)."
      info "3. mkdir myapp && cd myapp && nyra pkg init"
      ;;
  esac

  info ""
  info "Install location: ${INSTALL_DIR}"
  info "Docs: https://github.com/nyra-lang/nyra"
  info ""
  info "Windows? Use PowerShell instead:"
  info "  irm https://raw.githubusercontent.com/nyra-lang/nyra/main/scripts/install.ps1 | iex"
}

info ""
info "Nyra installed to $INSTALL_DIR"
INSTALLED_VER="$("$INSTALL_DIR/bin/nyra" --version 2>/dev/null | sed 's/^nyra //')"
FILE_VER="$(cat "$INSTALL_DIR/version" 2>/dev/null || true)"
TAG_VER="${TAG#v}"
info "nyra $INSTALLED_VER (release ${TAG})"
if [ -n "$INSTALLED_VER" ] && [ "$INSTALLED_VER" != "$TAG_VER" ]; then
  warn "binary version ($INSTALLED_VER) does not match release tag ($TAG)"
  warn "This release asset may be stale — try another tag or build from source."
fi
if [ -n "$FILE_VER" ] && [ "$FILE_VER" != "$TAG_VER" ]; then
  warn "version file ($FILE_VER) does not match release tag ($TAG)"
fi

if command -v nyra >/dev/null 2>&1; then
  active="$(command -v nyra)"
  if [ "$active" != "$INSTALL_DIR/bin/nyra" ]; then
    warn "'nyra' on PATH is still $active (expected ${INSTALL_DIR}/bin/nyra)"
    if [ -n "$UPDATED_PROFILE" ]; then
      warn "Run: source ${UPDATED_PROFILE}   (or open a new terminal)"
    fi
  fi
fi

print_next_steps
