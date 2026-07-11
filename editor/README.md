# Editor integration (VS Code / Cursor)

## Language Server

Nyra ships a full LSP via `nyra lsp` (stdio):

| Capability | Status |
|------------|--------|
| Diagnostics | ✅ errors + Extended warnings |
| Completion | ✅ symbols + keywords |
| Hover | ✅ type/signature markdown |
| Go to definition | ✅ cross-file (import graph) |
| Find references | ✅ workspace-wide |
| Rename | ✅ span-accurate cross-file |
| Document symbols | ✅ outline (fn/struct/enum/const) |
| Format | ✅ AST-based + comment preservation |
| Semantic tokens | ✅ keywords, types, functions, literals |
| Inlay hints | ✅ inferred types on `let` |
| Code actions | ✅ quick fixes from `help:` |
| Signature help | ✅ inside call parentheses |
| Workspace symbols | ✅ `#` search |
| Document highlight | ✅ read occurrences |

**LSP reliability:** incremental sync, debounced diagnostics (250ms), `didClose` cleanup, workspace file watcher refresh.

**LSP depth:** semantic tokens, inlay hints (types + param names), CodeLens ▶ Run Test, `source.fixAll` (inferred types), code actions, signature help, workspace symbols, document highlight, span-accurate rename.

**Extension polish:** Test Explorer (`nyra test --list-json`), format-on-save, status bar version, `$nyra` problem matcher, snippets, bundled-toolchain option, `scripts/package-vscode-extension.sh`.

**DAP Phase 4:** real LLDB/GDB sessions, breakpoints, stack/locals, stepping, source requests. Build with `nyra build . --debug-symbols` before debugging.

### Cursor / VS Code

**Recommended:** install the official extension from [`extensions/nyra/`](../extensions/nyra/) (Marketplace: *Nyra* by `nyra-lang`).

It wires:

- `nyra lsp` — language server (stdio)
- `nyra dap` — Debug Adapter Protocol (not raw lldb in `launch.json`)

Manual setup (without the extension):

1. Install the TextMate grammar from [`grammar/nyra.tmLanguage.json`](../grammar/nyra.tmLanguage.json) (see [`grammar/README.md`](../grammar/README.md)).
2. Copy snippets from [`editor/vscode/`](../editor/vscode/) into your project `.vscode/` or use as reference.
3. Configure the language server in **Settings → Languages & Frameworks** or `settings.json`:

```json
{
  "nyra.languageServerPath": "nyra",
  "nyra.languageServerArgs": ["lsp"]
}
```

If your editor supports generic LSP stdio, point it at:

```bash
nyra lsp
```

Fallback diagnostics without LSP:

```bash
nyra diag path/to/file.ny --json
```

## Formatter

```bash
nyra fmt              # stdout
nyra fmt --write .    # rewrite in place
nyra fmt --check .    # CI gate (exit 1 if not formatted)
```

Uses **AST-based** formatting when the file parses; falls back to line-based formatting otherwise.

## Debugger

```bash
nyra debug .                    # build -g + launch lldb/gdb (CLI wrapper)
nyra dap                        # DAP adapter (stdio) — used by VS Code extension
nyra debug . -- --arg1 value    # pass args to program
nyra debug . --init-vscode      # write .vscode/launch.json + tasks.json
```

The **VS Code extension** uses `nyra dap` (Debug Adapter Protocol) instead of invoking lldb directly.

Requires `lldb` (macOS) or `gdb` (Linux). Build with `--debug-symbols` for source-level stepping in `.ny` files (via DWARF in the native binary).

## Per-crate incremental (Cargo-style manifest)

`nyra build` tracks per-source-file hashes in `target/<profile>/.nyra-cache/crates/manifest.json` and reports changed crates. Full codegen still runs today (imports merge into one program); the manifest enables future split compilation.

## Incremental builds

`nyra build` fingerprints sources separately from link flags. Unchanged sources skip codegen and only relink when libraries/flags change.

```bash
# first build  → full compile
# second build → incremental: cache hit
# change --link-lib only → incremental: codegen skipped, relinking
```

## Watch mode

```bash
nyra watch .                  # re-check on save (default)
nyra watch . --on build       # rebuild
nyra watch . --on run         # rebuild + run
```

## CI gates

| Script | Purpose |
|--------|---------|
| `scripts/test-all.sh` | Full suite |
| `scripts/sanitizer-check.sh` | ASan/UBSan smoke |
| `scripts/fuzz-smoke.sh` | 60s libFuzzer per target |
| `scripts/fuzz-nightly.sh` | 5min libFuzzer (weekly CI) |

GitHub Actions: [`.github/workflows/ci.yml`](../.github/workflows/ci.yml)
