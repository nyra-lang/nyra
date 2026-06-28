# Nyra TextMate Grammar

Syntax highlighting for `.ny` and `.nyra` source files in editors that support **TextMate grammars** (VS Code, Cursor, Sublime Text, Atom, etc.).

## Grammar file

| | |
|--|--|
| **Repository path** | [`grammar/nyra.tmLanguage.json`](nyra.tmLanguage.json) |
| **Raw URL (main branch)** | `https://raw.githubusercontent.com/nyra-lang/nyra/main/grammar/nyra.tmLanguage.json` |
| **Latest (v0.3.0)** | `https://raw.githubusercontent.com/nyra-lang/nyra/v0.3.0/grammar/nyra.tmLanguage.json` |
| **Pinned (v0.2.0)** | `https://raw.githubusercontent.com/nyra-lang/nyra/v0.2.0/grammar/nyra.tmLanguage.json` |

The canonical spec for language syntax is [`docs/spec-v1.md`](../docs/spec-v1.md). This JSON mirrors the **surface keywords** for highlighting only; it is not a formal grammar.

**Scope:** highlights both **Core** and **Extended** keywords. Learn Core first — see [`docs/status.md`](../docs/status.md).

## VS Code / Cursor

1. Copy or symlink `nyra.tmLanguage.json` into an extension, or use a local extension folder:

```text
nyra-syntax/
  package.json
  syntaxes/
    nyra.tmLanguage.json   ← copy from this directory
```

2. Minimal `package.json`:

```json
{
  "name": "nyra-syntax",
  "displayName": "Nyra",
  "version": "1.0.0",
  "engines": { "vscode": "^1.80.0" },
  "contributes": {
    "languages": [{
      "id": "nyra",
      "aliases": ["Nyra"],
      "extensions": [".ny", ".nyra"]
    }],
    "grammars": [{
      "language": "nyra",
      "scopeName": "source.nyra",
      "path": "./syntaxes/nyra.tmLanguage.json"
    }]
  }
}
```

3. Open the extension folder in VS Code and run **Extensions: Install Extension from Location**.

## Related tooling

- Compiler diagnostics: `nyra diag path.ny --json` — see [`docs/tooling.md`](../docs/tooling.md)
- Language server: `nyra lsp` (stdio) — diagnostics MVP via `lsp/` library
