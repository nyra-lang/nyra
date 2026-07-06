# Contributor hub (`make contribute`)

Unified **step-by-step monitor** for Nyra contribution scaffolds. Each recipe generates marked files (`[contrib-dev:…]`) and explains **WHAT / WHY / TOOL vs YOU** at every question.

Full walkthrough (menu, all questions, example answers, simulations): [`CONTRIBUTING.md` § Contributor hub guide](../../../CONTRIBUTING.md#contributor-hub-guide-make-contribute).

## Monitor legend

Every interactive step shows:

| Label | Meaning |
|-------|---------|
| **WHY** | Why we ask this question |
| **TOOL** | What `make contribute` writes automatically |
| **YOU** | What you implement after the tool finishes |

After questions: **PREVIEW + confirm** → then **MONITOR** (TOOL DID / YOU DO / WHERE / VERIFY / UNDO).

## Quick start

```bash
make contribute                    # interactive add — tiger logo + menu
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
│  Step-by-step monitor — TOOL wires, YOU code│
├─────────────────────────────────────────────┤
│ 1. Stdlib Pure Function (Pattern A)         │
│    Nyra fn in stdlib — no new C             │
│ 2. Stdlib Extern + C (Pattern B)            │
│    extern fn + rt/*.c + runtime_map         │
│ 3. Built-in Method (.method)                │
│    → make add-builtin wizard                │
│ 4. Test + Example Pair                      │
│    tests/nyra/* + examples/* (typed pair)   │
│ 5. NyraPkg Package                          │
│    examples/packages/<name>/                │
│ 6. CLI Command / Flag                       │
│    scaffold → manual wire in cli/           │
│ 7. Conformance Test                         │
│    pass/ or fail/ language contract         │
│ 8. Syntax / Keyword Scaffold                │
│    checklist — no auto lexer/parser         │
└─────────────────────────────────────────────┘
```

| # | Recipe | What TOOL wires | What YOU implement |
|---|--------|-----------------|-------------------|
| 1 | `stdlib-pure` | `fn` in `stdlib/**/*.ny` + test + example | fn body |
| 2 | `stdlib-extern` | `extern fn` + `stdlib/rt/*.c` + `runtime_map.rs` (+ optional ABI) | C implementation |
| 3 | `builtin` | Delegates to `make add-builtin` | C + compiler wiring |
| 4 | `test-example` | `tests/nyra/*_test.ny` + `examples/<topic>/` pair | assertions + demo |
| 5 | `pkg` | `examples/packages/<name>/` NyraPkg layout | API + optional C |
| 6 | `cli` | Scaffold under `docs/contrib_scaffold/cli_<name>/` | manual wire in `cli/` |
| 7 | `conformance` | `tests/conformance/pass/` or `fail/` contract | contract code |
| 8 | `syntax-scaffold` | Checklist + tests/examples — **no auto lexer/parser** | full compiler pipeline |

Wizard copy (WHY/TOOL/YOU per question) lives in `wizard_guide.py` — keep in sync with CONTRIBUTING.md.

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
| `wizard_guide.py` | WHY/TOOL/YOU copy per recipe + step |
| `discover.py` | Scan repo for wired markers |
| `remove.py` | Remove scaffolds |
| `patch_recipe.py` | Remove + re-add |
| `spec.py` | Recipe data models |
| `wizard.py` | Interactive prompts + preview/confirm |
| `templates.py` | Nyra / C / Rust templates |
| `patch.py` | `[contrib-dev:…]` file patching |
| `monitor.py` | Terminal monitor output |
| `tiger_banner.py` | Static ASCII tiger logo |
| `recipes/*.py` | One module per menu item |

## CI

```bash
make test-contrib-py   # py_compile + JSON spec smoke (in test-preflight)
```

## Related

- Built-in methods: [`../builtin_dev/README.md`](../builtin_dev/README.md)
- Full guide: [`../../../CONTRIBUTING.md#contributor-hub-guide-make-contribute`](../../../CONTRIBUTING.md#contributor-hub-guide-make-contribute)
- Contributor map: [`../../../docs/contributor-map.md`](../../../docs/contributor-map.md)
- Makefile catalog: [`../../../docs/make-and-generators.md`](../../../docs/make-and-generators.md)
