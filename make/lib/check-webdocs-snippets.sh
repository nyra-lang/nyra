#!/usr/bin/env bash
# Run webDocs Nyra snippets end-to-end (nyra run). Catches doc examples that only pass `check`.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

# shellcheck source=nyra-bin.sh
source "$ROOT/make/lib/nyra-bin.sh"
nyra_export_cli
export NYRA_BIN

MANIFEST="$ROOT/tests/webdocs/pass-manifest.txt"
if [[ "${NYRA_WEBDOCS_FULL:-}" == "1" ]]; then
  python3 "$ROOT/make/py/check-webdocs-snippets.py" "$@"
else
  python3 "$ROOT/make/py/check-webdocs-snippets.py" \
    --manifest "$MANIFEST" \
    "$@"
fi
