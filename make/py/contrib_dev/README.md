# Contributor hub (`make contribute`)

Unified interactive menu for Nyra contribution scaffolds. Each recipe generates marked files (`[contrib-dev:…]`) and prints a **monitor report** (DONE / YOUR TASKS / NEXT STEPS).

## Quick start

```bash
make contribute                    # interactive add (default)
make contribute-list               # list wired scaffolds
make contribute-remove ARGS='-i'   # remove by marker
make contribute-patch ARGS='--marker test_example:foo --config make/py/contrib_dev/examples/test_example.json'

# Non-interactive add with JSON spec:
make contribute ARGS='add --recipe stdlib-extern --config make/py/contrib_dev/examples/stdlib_extern.json'
```

## Menu

```
┌─────────────────────────────────────────────┐
│             make contribute                 │
├─────────────────────────────────────────────┤
│ 1. Stdlib Pure Function (Pattern A)         │
│ 2. Stdlib Extern + C (Pattern B)            │
│ 3. Built-in Method (.method)                │
│ 4. Test + Example Pair                      │
│ 5. NyraPkg Package                          │
│ 6. CLI Command / Flag                       │
│ 7. Conformance Test                         │
│ 8. Syntax / Keyword Scaffold                │
└─────────────────────────────────────────────┘
```

| # | Recipe | What it wires |
|---|--------|---------------|
| 1 | `stdlib-pure` | `fn` in `stdlib/**/*.ny` + test + example |
| 2 | `stdlib-extern` | `extern fn` + `stdlib/rt/*.c` + `runtime_map.rs` (+ optional ABI) |
| 3 | `builtin` | Delegates to `make add-builtin` |
| 4 | `test-example` | `tests/nyra/*_test.ny` + `examples/<topic>/` pair |
| 5 | `pkg` | `examples/packages/<name>/` NyraPkg layout |
| 6 | `cli` | Scaffold under `docs/contrib_scaffold/cli_<name>/` (manual wire) |
| 7 | `conformance` | `tests/conformance/pass/` or `fail/` contract |
| 8 | `syntax-scaffold` | Checklist + tests/examples — **no auto lexer/parser edits** |

## Subcommands

| Command | Make target | Purpose |
|---------|-------------|---------|
| `add` | `make contribute` | Create scaffold (default) |
| `list` | `make contribute-list` | Show all `[contrib-dev:…]` markers |
| `remove` | `make contribute-remove` | Remove scaffold by marker |
| `patch` | `make contribute-patch` | Remove + re-add with updated spec |

## JSON examples (`examples/`)

| File | Recipe |
|------|--------|
| `stdlib_pure.json` | 1 |
| `stdlib_extern.json` | 2 |
| `test_example.json` | 4 |
| `pkg.json` | 5 |
| `cli.json` | 6 |
| `conformance.json` | 7 |
| `syntax_scaffold.json` | 8 |

## File map

| Path | Purpose |
|------|---------|
| `../contribute.py` | Hub CLI |
| `discover.py` | Scan repo for wired markers |
| `remove.py` | Remove scaffolds |
| `patch_recipe.py` | Remove + re-add |
| `spec.py` | Recipe data models |
| `wizard.py` | Interactive prompts |
| `templates.py` | Nyra / C / Rust templates |
| `patch.py` | `[contrib-dev:…]` file patching |
| `monitor.py` | Terminal monitor output |
| `recipes/*.py` | One module per menu item |

## CI

```bash
make test-contrib-py   # py_compile + JSON spec smoke (in test-preflight)
```

## Related

- Built-in methods: [`../builtin_dev/README.md`](../builtin_dev/README.md)
- Full guide: [`../../../docs/contributor-map.md`](../../../docs/contributor-map.md)
- Makefile catalog: [`../../../docs/make-and-generators.md`](../../../docs/make-and-generators.md)
