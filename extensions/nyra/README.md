# Nyra for VS Code

Official Nyra language extension: syntax highlighting, LSP, and DAP debugging.

## Features

- **Syntax** for `.ny` / `.nyra`
- **Language Server** (`nyra lsp`): diagnostics, completion, hover, go-to-definition, find references, rename, format
- **Debugger** (`nyra dap`): DAP adapter over LLDB/GDB (not raw lldb in launch.json)
- **Tasks**: build, run, check

## Setup

1. Install the [Nyra toolchain](https://github.com/nyra-lang/nyra#quick-start).
2. Install this extension from the Marketplace or a `.vsix` build.
3. Open a folder with `main.ny`.

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `nyra.languageServerPath` | `nyra` | CLI for LSP |
| `nyra.languageServerArgs` | `["lsp"]` | LSP args |
| `nyra.debugAdapterPath` | `nyra` | CLI for DAP |

## Debug

Use **Run and Debug → Nyra: Launch**. Build with debug symbols first:

```bash
nyra build . --debug-symbols
```

## License

MIT — see repository root.
