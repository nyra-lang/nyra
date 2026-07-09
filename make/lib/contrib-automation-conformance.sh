#!/usr/bin/env bash
# Contributor automation conformance (CONF-CONTRIB-PY): Python hub + batch tooling.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

log() { echo "contrib-automation-conformance: $*" >&2; }

log "running make/py/test_contrib_conformance.py"
python3 "$ROOT/make/py/test_contrib_conformance.py"
log "done"
