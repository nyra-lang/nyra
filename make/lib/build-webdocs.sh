#!/usr/bin/env bash
# Build webDocs search index and AI skill file.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
WEBDOCS="$ROOT/webDocs"

echo "==> Syncing skills/skill.md from webDocs/nyra-skill.md"
node "$WEBDOCS/scripts/build-nyra-skill.mjs"

echo "==> Generating .typed.ny example siblings"
python3 "$ROOT/make/py/gen-typed-examples.py"

echo "==> Embedding Without types / With types tabs on all doc code snippets"
node "$WEBDOCS/scripts/embed-all-code-tabs.mjs"

echo "==> Capturing built-in example output (methods.html)"
if [ -x "$ROOT/target/release/nyra" ] || [ -x "$ROOT/target/debug/nyra" ]; then
  node "$WEBDOCS/scripts/capture-builtin-outputs.mjs" || \
    echo "   (capture-builtin-outputs failed — gallery keeps previous outputs)"
else
  echo "   (skipped: no nyra binary built — run 'cargo build -p cli' to capture output)"
fi

echo "==> Embedding built-in runnable gallery in stdlib.html"
node "$WEBDOCS/scripts/build-builtin-snippets.mjs"

echo "==> Building search-index.json"
node "$WEBDOCS/scripts/build-search-index.mjs"

echo "==> webDocs build complete"
