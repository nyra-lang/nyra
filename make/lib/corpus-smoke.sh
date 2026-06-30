#!/usr/bin/env bash
# nyra check on every examples/corpus manifest entry with expect_compile=true.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
# shellcheck source=test-all-collect.sh
source "$ROOT/make/lib/test-all-collect.sh"
ta_set_scope "corpus-smoke"
export NYRA_TEST_STATS_FILE="${NYRA_TEST_STATS_FILE:-$ROOT/target/.nyra-test-all-stats}"
# shellcheck source=test-stats.sh
source "$ROOT/make/lib/test-stats.sh"

log() { echo "corpus-smoke: $*" >&2; }
fail() {
  log "FAILED: $*"
  ta_fail "$*" "" || exit 1
}

paths="$(python3 - <<'PY'
import tomllib
from pathlib import Path

manifest = Path("tests/corpus/manifest.toml")
data = tomllib.loads(manifest.read_text())
for case in data.get("case", []):
    if case.get("expect_compile", True):
        print(case["path"])
PY
)"

count=0
failed=0
while IFS= read -r path; do
  [[ -z "$path" ]] && continue
  if [[ "${NYRA_TEST_ALL:-}" != "1" ]]; then
    log "check corpus: $path"
  fi
  if ! nyra_stats_check "$path"; then
    failed=$((failed + 1))
    if [[ "${NYRA_TEST_ALL:-}" != "1" ]]; then
      fail "corpus check $path"
    fi
  fi
  count=$((count + 1))
done <<<"$paths"

if (( failed > 0 )); then
  ta_finish "corpus-smoke"
fi
log "ok — $count corpus entries"
