#!/usr/bin/env sh
# Public installer entry — works as:
#   curl -fsSL https://raw.githubusercontent.com/nyra-lang/nyra/main/scripts/install.sh | sh
#   ./scripts/install.sh          (from a git clone)
# Implementation: make/lib/install.sh
set -eu

REPO="${NYRA_INSTALL_REPO:-nyra-lang/nyra}"
BRANCH="${NYRA_INSTALL_BRANCH:-main}"

# Running from clone: ./scripts/install.sh or /path/to/nyra/scripts/install.sh
case "$0" in
  */scripts/install.sh)
    ROOT="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
    if [ -f "$ROOT/make/lib/install.sh" ]; then
      exec sh "$ROOT/make/lib/install.sh" "$@"
    fi
    ;;
esac

# curl | sh — $0 is "sh" / "-"; fetch the self-contained installer
URL="https://raw.githubusercontent.com/${REPO}/${BRANCH}/make/lib/install.sh"
if ! command -v curl >/dev/null 2>&1; then
  echo "error: curl is required" >&2
  exit 1
fi
curl -fsSL "$URL" | sh -s -- "$@"
