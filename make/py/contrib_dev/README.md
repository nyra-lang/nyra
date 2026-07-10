# Contributor hub (`make contribute`)

**Single entry point** for all Nyra contribution automation. Run `make contribute` and pick from the main menu — you do **not** need `make add-builtin`, `make batch-add-builtin`, or `make gen-batchN` separately; the hub runs them internally when you choose the matching option.

Full walkthrough: [`CONTRIBUTING.md` § Contributor hub guide](../../../CONTRIBUTING.md#contributor-hub-guide-make-contribute).

## Monitor legend

Every interactive step shows:

| Label | Meaning |
|-------|---------|
| **WHY** | Why we ask this question |
| **TOOL** | What the hub writes or runs automatically |
| **YOU** | What you implement after the tool finishes |

After questions: **PREVIEW + confirm** → then **MONITOR** (TOOL DID / YOU DO / WHERE / VERIFY / UNDO).

## Main menu (`make contribute`)

```
┌─────────────────────────────────────────────┐
│             make contribute                 │
│  Single hub — TOOL wires, YOU code          │
├─────────────────────────────────────────────┤
│ 1. Add          stdlib, builtin, test, pkg… │
│ 2. Remove       contrib-dev or builtin-dev  │
│ 3. List         all wired scaffolds         │
│ 4. Patch        update wiring / re-scaffold │
│ 5. Batch        gen-batchN + batch-add      │
│ 6. Verify       install-dev, tests, nyra    │
│ 0. Exit                                     │
└─────────────────────────────────────────────┘
```

### Add → recipe submenu

| # | Recipe | What TOOL wires | What YOU implement |
|---|--------|-----------------|-------------------|
| 1 | `stdlib-pure` | `fn` in `stdlib/**/*.ny` + test + example | fn body |
| 2 | `stdlib-extern` | `extern fn` + `stdlib/rt/*.c` + `runtime_map.rs` | C implementation |
| 3 | `builtin` | Runs **builtin-dev add** internally (compiler + C) | C in `stdlib/rt/` |
| 4 | `test-example` | `tests/nyra/*_test.ny` + `examples/<topic>/` | assertions + demo |
| 5 | `pkg` | `examples/packages/<name>/` NyraPkg layout | API + optional C |
| 6 | `cli` | Scaffold under `docs/contrib_scaffold/cli_<name>/` | wire in `cli/` |
| 7 | `conformance` | `tests/conformance/pass/` or `fail/` | contract code |
| 8 | `syntax-scaffold` | Checklist + tests — **no auto lexer/parser** | full compiler pipeline |

### Batch (menu 5)

For many APIs at once (batch3–6 catalogs):

1. Pick batch folder (`batch3`, `batch4`, …)
2. **Generate** JSON from catalog (`gen-batchN`)
3. **Apply** scaffolds (`batch-add-builtin` → builtin-dev + contribute recipes)
4. **Full pipeline** — generate → apply → generate (consolidate)

### Verify (menu 6)

Runs common next steps: `make install-dev`, **Post-scaffold CI gates** (`abi_manifest` + `nyra check`), `make test-contrib-py`, `make test-preflight`, `make test-optional-types`, `nyra test <path>`, `make build-webdocs`.

After **Add** or **Batch**, the hub prompts to run CI safety gates (disable with `NYRA_CONTRIBUTE_SKIP_GATES=1`).

## Non-interactive (CI / scripts)

```bash
make contribute ARGS='add --recipe stdlib-pure --config make/py/contrib_dev/examples/stdlib_pure.json --no-webdocs'
make contribute ARGS='remove --marker test_example:foo --no-webdocs'
make contribute ARGS='list'
make contribute ARGS='patch --marker … --config … --no-webdocs'
```

**Speed tip:** `NYRA_CONTRIBUTE_SKIP_WEBDOCS=1` for batch/CI.

## Legacy make targets

These still work for scripts but print a tip to use the hub:

| Old target | Hub equivalent |
|------------|----------------|
| `make add-builtin` | `make contribute` → 1 → 3 |
| `make remove-builtin` | `make contribute` → 2 → Built-in |
| `make patch-builtin` | `make contribute` → 4 → Built-in |
| `make batch-add-builtin` | `make contribute` → 5 |
| `make gen-batchN` | `make contribute` → 5 → Generate |
| `make contribute-remove` | `make contribute` → 2 |
| `make contribute-list` | `make contribute` → 3 |
| `make contribute-patch` | `make contribute` → 4 |

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

- Built-in methods (recipe 3 internals): [`../builtin_dev/README.md`](../builtin_dev/README.md)
- Full guide: [`../../../CONTRIBUTING.md#contributor-hub-guide-make-contribute`](../../../CONTRIBUTING.md#contributor-hub-guide-make-contribute)
- Contributor map: [`../../../docs/contributor-map.md`](../../../docs/contributor-map.md)
- Makefile catalog: [`../../../docs/make-and-generators.md`](../../../docs/make-and-generators.md)
