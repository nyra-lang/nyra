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
make contribute-remove ARGS='-i'   # remove by marker (skips webDocs by default)
make contribute-patch ARGS='--marker test_example:foo --config make/py/contrib_dev/examples/test_example.json'

# Non-interactive add with JSON spec:
make contribute ARGS='add --recipe stdlib-pure --config make/py/contrib_dev/examples/stdlib_pure.json --no-webdocs'

# Multi-fn / struct module (put full Nyra source in pure_source):
make contribute ARGS='add --recipe stdlib-pure --config make/py/contrib_dev/examples/stdlib_module.json --force --no-webdocs'
```

**Speed tip:** `contribute-remove` / `contribute-patch` default to `--no-webdocs`. Use `NYRA_CONTRIBUTE_SKIP_WEBDOCS=1` for CI/scripts.

## Pain points fixed (for future contributors)

| Problem | Fix |
|---------|-----|
| One-fn-only pure scaffold | `pure_source` embeds full structs/fns into the module file |
| Remove/list hung on huge trees | `discover.py` skips `target`, `webDocs`, `vendor`, `Apps`, caches |
| Remove always rebuilt webDocs | make targets pass `--no-webdocs` by default |
| Patch did not refresh docs optionally | `--no-webdocs` on patch; opt-in regenerates |

## Menu

```
┌─────────────────────────────────────────────┐
│             make contribute                 │
│  Step-by-step monitor — TOOL wires, YOU code│
├─────────────────────────────────────────────┤
│ 1. Stdlib Pure Function (Pattern A)         │
│    Nyra fn in stdlib — no new C             │
│    (also: multi-fn modules via pure_source) │
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
| 1 | `stdlib-pure` | `fn`/`pure_source` in `stdlib/**/*.ny` + test + example | fn body / module body |
| 2 | `stdlib-extern` | `extern fn` + `stdlib/rt/*.c` + `runtime_map.rs` (+ optional ABI) | C implementation |
| 3 | `builtin` | Delegates to `make add-builtin` | C + compiler wiring |
| 4 | `test-example` | `tests/nyra/*_test.ny` + `examples/<topic>/` pair | assertions + demo |
| 5 | `pkg` | `examples/packages/<name>/` NyraPkg layout | API + optional C |
| 6 | `cli` | Scaffold under `docs/contrib_scaffold/cli_<name>/` | manual wire in `cli/` |
| 7 | `conformance` | `tests/conformance/pass/` or `fail/` contract | contract code |
| 8 | `syntax-scaffold` | Checklist + tests/examples — **no auto lexer/parser** | full compiler pipeline |

Wizard copy (WHY/TOOL/YOU per question) lives in `wizard_guide.py` — keep in sync with CONTRIBUTING.md.

## JSON examples (`examples/`)

| File | Recipe |
|------|--------|
| `stdlib_pure.json` | 1 (single fn) |
| `stdlib_module.json` | 1 (`pure_source` multi-fn) |
| `stdlib_extern.json` | 2 |
| `test_example.json` | 4 |
| `pkg.json` | 5 |
| `cli.json` | 6 |
| `conformance.json` | 7 |
| `syntax_scaffold.json` | 8 |

## Related

- Built-in methods: [`../builtin_dev/README.md`](../builtin_dev/README.md)
- Full guide: [`../../../CONTRIBUTING.md#contributor-hub-guide-make-contribute`](../../../CONTRIBUTING.md#contributor-hub-guide-make-contribute)
- Contributor map: [`../../../docs/contributor-map.md`](../../../docs/contributor-map.md)
- Makefile catalog: [`../../../docs/make-and-generators.md`](../../../docs/make-and-generators.md)
