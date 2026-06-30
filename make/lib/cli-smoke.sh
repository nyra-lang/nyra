#!/usr/bin/env bash
# CLI smoke: fmt, diag, ide, pkg, bind, LSP, DAP.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
export NYRA_TEST_STATS_FILE="${NYRA_TEST_STATS_FILE:-$ROOT/target/.nyra-test-all-stats}"
# shellcheck source=test-stats.sh
source "$ROOT/make/lib/test-stats.sh"
export ROOT

log() { echo "cli-smoke: $*" >&2; }
fail() { log "FAILED: $*"; exit 1; }

# shellcheck source=nyra-bin.sh
source "$ROOT/make/lib/nyra-bin.sh"
nyra_export_cli
NYRA=("$NYRA_BIN")
HELLO="$ROOT/examples/syntax/hello.ny"
PKG_FIXTURE="$ROOT/examples/packages/ny-sqlite"

# --- fmt ---
log "fmt --check hello.ny"
if ! "${NYRA[@]}" fmt --check "$HELLO" >/dev/null; then
  fail "fmt --check hello.ny"
fi

log "fmt --write roundtrip"
FMT_TMP="$(mktemp -d)"
cp "$HELLO" "$FMT_TMP/hello.ny"
"${NYRA[@]}" fmt --write "$FMT_TMP/hello.ny" >/dev/null
if ! nyra_stats_check "$FMT_TMP/hello.ny"; then
  fail "fmt --write broke hello.ny"
fi
rm -rf "$FMT_TMP"

log "fmt print (stdout)"
out="$("${NYRA[@]}" fmt "$HELLO" 2>/dev/null || true)"
if [[ -z "$out" ]]; then
  fail "fmt stdout empty"
fi

# --- diag / ide ---
log "diag hello.ny"
if ! "${NYRA[@]}" diag "$HELLO" >/dev/null; then
  fail "diag hello.ny"
fi

log "diag --json hello.ny"
if ! "${NYRA[@]}" diag --json "$HELLO" >/dev/null; then
  fail "diag --json hello.ny"
fi

log "diag --json includes code field"
diag_json="$("${NYRA[@]}" diag --json "$ROOT/examples/tooling/diag_json.ny" 2>/dev/null || true)"
if [[ -z "$diag_json" ]] || ! echo "$diag_json" | grep -q '"code"'; then
  fail "diag --json missing code field"
fi

log "explain E003"
if ! "${NYRA[@]}" explain E003 >/dev/null; then
  fail "explain E003"
fi

log "explain --list"
if ! "${NYRA[@]}" explain --list >/dev/null; then
  fail "explain --list"
fi

log "ide goto-def (wiring)"
out="$("${NYRA[@]}" ide goto-def "$HELLO" 1 2>&1 || true)"
if [[ -z "$out" ]]; then
  fail "ide goto-def produced no output"
fi

log "ide references (wiring)"
out="$("${NYRA[@]}" ide references "$HELLO" 2 2>&1 || true)"
if [[ -z "$out" ]]; then
  fail "ide references produced no output"
fi

# --- pkg (nyra pkg ↔ nyrapkg aliases) ---
NYRAPKG_BIN="${NYRAPKG:-$ROOT/../nyrapkg/target/release/nyrapkg}"
if [[ -x "$NYRAPKG_BIN" ]]; then
  export NYRAPKG="$NYRAPKG_BIN"
  log "nyra pkg init (delegates to nyrapkg)"
  PKG_INIT_TMP="$(mktemp -d)"
  if ! "${NYRA[@]}" pkg init "$PKG_INIT_TMP" >/dev/null; then
    fail "nyra pkg init"
  fi
  if [[ ! -f "$PKG_INIT_TMP/nyra.mod" ]]; then
    fail "nyra pkg init missing nyra.mod"
  fi
  rm -rf "$PKG_INIT_TMP"
fi

scaffold_pkg_project() {
  local d="$1"
  mkdir -p "$d"
  printf 'module example.local\n\n' >"$d/nyra.mod"
  printf 'fn main() {\n    print("hello world")\n}\n' >"$d/main.ny"
}

log "pkg scaffold (temp project)"
PKG_TMP="$(mktemp -d)"
scaffold_pkg_project "$PKG_TMP"
if [[ ! -f "$PKG_TMP/nyra.mod" || ! -f "$PKG_TMP/main.ny" ]]; then
  fail "pkg scaffold missing nyra.mod or main.ny"
fi
rm -rf "$PKG_TMP"

log "pkg prune --check (prune_unused fixture)"
PRUNE_FIXTURE="$ROOT/tests/fixtures/prune_unused"
if "${NYRA[@]}" pkg prune --check --path "$PRUNE_FIXTURE" >/dev/null 2>&1; then
  fail "pkg prune --check should fail when unused code exists"
fi
nyra_stats_pass

log "pkg build ny-sqlite fixture"
if ! "${NYRA[@]}" pkg build "$PKG_FIXTURE" >/dev/null 2>&1; then
  fail "pkg build ny-sqlite"
fi

SERDE_FIXTURE="$ROOT/examples/packages/ny-serde"
log "pkg build ny-serde fixture"
if ! "${NYRA[@]}" pkg build "$SERDE_FIXTURE" >/dev/null 2>&1; then
  fail "pkg build ny-serde"
fi

log "bind rust serde_json --template (temp project)"
BIND_SERDE_TMP="$(mktemp -d)"
scaffold_pkg_project "$BIND_SERDE_TMP"
if ! "${NYRA[@]}" bind rust serde_json --template --project "$BIND_SERDE_TMP" >/dev/null 2>&1; then
  fail "bind rust serde_json --template"
fi
if [[ ! -f "$BIND_SERDE_TMP/.nyra/cache/rust/serde_json/bindings.ny" ]]; then
  fail "bind serde_json missing bindings.ny"
fi
rm -rf "$BIND_SERDE_TMP"

# --- bind ---
log "bind rust uuid --template (temp project)"
BIND_TMP="$(mktemp -d)"
scaffold_pkg_project "$BIND_TMP"
if ! "${NYRA[@]}" bind rust uuid --template --project "$BIND_TMP" >/dev/null 2>&1; then
  fail "bind rust uuid --template"
fi
if [[ ! -f "$BIND_TMP/.nyra/cache/rust/uuid/bindings.ny" ]]; then
  fail "bind missing bindings.ny"
fi
rm -rf "$BIND_TMP"

# --- LSP / DAP (stdio protocol) ---
log "lsp initialize handshake"
python3 - <<'PY'
import json
import subprocess
import sys

ROOT = __import__("os").environ.get("ROOT", ".")


def read_lsp_msg(stream):
    headers = {}
    while True:
        line = stream.readline()
        if not line:
            return None
        line = line.decode().strip()
        if line == "":
            break
        key, val = line.split(":", 1)
        headers[key.strip()] = val.strip()
    length = int(headers.get("Content-Length", 0))
    if length == 0:
        return None
    return json.loads(stream.read(length).decode())


def read_lsp_response(stream, req_id):
    while True:
        msg = read_lsp_msg(stream)
        if msg is None:
            return None
        if msg.get("id") == req_id and "result" in msg:
            return msg
        # skip notifications (publishDiagnostics, etc.)


def write_lsp_msg(stream, msg):
    body = json.dumps(msg)
    stream.write(f"Content-Length: {len(body)}\r\n\r\n{body}".encode())
    stream.flush()


NYRA_BIN = __import__("os").environ.get("NYRA_BIN", "nyra")

proc = subprocess.Popen(
    [NYRA_BIN, "lsp"],
    cwd=ROOT,
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.DEVNULL,
)
write_lsp_msg(
    proc.stdin,
    {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {"processId": None, "capabilities": {}, "rootUri": None},
    },
)
resp = read_lsp_msg(proc.stdout)
if not resp or "result" not in resp:
    print("LSP bad response:", resp, file=sys.stderr)
    sys.exit(1)
caps = resp["result"].get("capabilities", {})
if not caps.get("definitionProvider"):
    print("LSP missing definitionProvider", file=sys.stderr)
    sys.exit(1)
sync = caps.get("textDocumentSync") or {}
if isinstance(sync, dict):
    change = sync.get("change")
else:
    change = sync
if not caps.get("semanticTokensProvider"):
    print("LSP missing semanticTokensProvider", file=sys.stderr)
    sys.exit(1)
if not caps.get("codeActionProvider"):
    print("LSP missing codeActionProvider", file=sys.stderr)
    sys.exit(1)
write_lsp_msg(
    proc.stdin,
    {
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///tmp/lsp_smoke.ny",
                "languageId": "nyra",
                "version": 1,
                "text": "fn greet() {}\nfn main() { greet() }\n",
            }
        },
    },
)
write_lsp_msg(
    proc.stdin,
    {
        "jsonrpc": "2.0",
        "id": 3,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {"uri": "file:///tmp/lsp_smoke.ny"},
            "position": {"line": 1, "character": 14},
        },
    },
)
comp = read_lsp_response(proc.stdout, 3)
if not comp:
    print("LSP completion bad response:", comp, file=sys.stderr)
    sys.exit(1)
write_lsp_msg(
    proc.stdin,
    {
        "jsonrpc": "2.0",
        "id": 4,
        "method": "textDocument/definition",
        "params": {
            "textDocument": {"uri": "file:///tmp/lsp_smoke.ny"},
            "position": {"line": 1, "character": 14},
        },
    },
)
defn = read_lsp_response(proc.stdout, 4)
if not defn:
    print("LSP goto-definition bad response:", defn, file=sys.stderr)
    sys.exit(1)
write_lsp_msg(proc.stdin, {"jsonrpc": "2.0", "method": "initialized", "params": {}})
write_lsp_msg(proc.stdin, {"jsonrpc": "2.0", "id": 2, "method": "exit", "params": {}})
proc.stdin.close()
proc.wait(timeout=5)
PY

log "dap initialize handshake"
python3 - <<'PY'
import json
import subprocess
import sys

ROOT = __import__("os").environ.get("ROOT", ".")


def read_msg(stream):
    headers = {}
    while True:
        line = stream.readline()
        if not line:
            return None
        line = line.decode().strip()
        if line == "":
            break
        key, val = line.split(":", 1)
        headers[key.strip()] = val.strip()
    length = int(headers.get("Content-Length", 0))
    return json.loads(stream.read(length).decode())


def write_msg(stream, msg):
    body = json.dumps(msg)
    stream.write(f"Content-Length: {len(body)}\r\n\r\n{body}".encode())
    stream.flush()


NYRA_BIN = __import__("os").environ.get("NYRA_BIN", "nyra")

proc = subprocess.Popen(
    [NYRA_BIN, "dap"],
    cwd=ROOT,
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)
write_msg(
    proc.stdin,
    {"seq": 1, "type": "request", "command": "initialize", "arguments": {}},
)
resp = read_msg(proc.stdout)
if not resp or resp.get("type") != "response" or not resp.get("success"):
    print("DAP bad response:", resp, file=sys.stderr)
    sys.exit(1)
body = resp.get("body", {})
if not body.get("supportsConfigurationDoneRequest"):
    print("DAP missing supportsConfigurationDoneRequest", file=sys.stderr)
    sys.exit(1)
write_msg(
    proc.stdin,
    {
        "seq": 2,
        "type": "request",
        "command": "setBreakpoints",
        "arguments": {
            "source": {"path": str(__import__("pathlib").Path(ROOT) / "examples/tooling/debug_demo.ny")},
            "lines": [3],
            "breakpoints": [{"line": 3}],
        },
    },
)
resp2 = read_msg(proc.stdout)
if not resp2 or not resp2.get("success"):
    print("DAP setBreakpoints failed:", resp2, file=sys.stderr)
    sys.exit(1)
bps = resp2.get("body", {}).get("breakpoints", [])
if not bps or bps[0].get("line") != 3:
    print("DAP setBreakpoints bad body:", resp2, file=sys.stderr)
    sys.exit(1)
write_msg(proc.stdin, {"seq": 3, "type": "request", "command": "disconnect", "arguments": {}})
proc.stdin.close()
proc.wait(timeout=5)
PY

# --- test list-json / filter (IDE test explorer) ---
log "test --list-json"
LIST_JSON="$("${NYRA[@]}" test examples/tooling/test_list_json.ny --list-json 2>/dev/null || true)"
if ! echo "$LIST_JSON" | python3 -c "import json,sys; d=json.load(sys.stdin); assert len(d)>=2 and 'name' in d[0]"; then
  fail "test --list-json (expected JSON array with name field)"
fi

log "test --filter"
if ! "${NYRA[@]}" test examples/tooling/test_list_json.ny --filter adds >/dev/null 2>&1; then
  fail "test --filter adds"
fi

log "ok — cli smoke (fmt, diag, ide, pkg, bind, lsp, dap)"
