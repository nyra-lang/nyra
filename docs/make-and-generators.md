# Makefile & Python generators — contributor guide

Nyra keeps **build orchestration in Make** and **code/doc generators in Python** under `make/py/`. Contributors should almost always invoke generators through `make <target>` — not by running Python scripts directly (unless debugging the script itself).

**Quick reference:** `make help` · [`make/generators.mk`](../make/generators.mk) · [`make/py/builtin_dev/README.md`](../make/py/builtin_dev/README.md)

---

## Directory layout

```
make/
├── common.mk              # Shared variables (ROOT, NYRA_BIN, paths)
├── nyra.mk                # Build `target/debug/nyra` CLI binary
├── build.mk               # bench, webDocs rebuild, packaging
├── install.mk             # install-dev, install-release, install-llvm
├── test.mk                # Individual test gates (compiletest, nyra-lang, …)
├── smoke.mk               # CLI/stdlib/examples smoke targets
├── generators.mk          # Python generator entry points ← start here for py/
├── test-all.mk            # Full CI-equivalent suite
├── test-platform*.mk      # OS-specific CI subsets
├── lib/                   # Shell helpers (test-all-gate.sh, bench.sh, …)
└── py/                    # Python generators + builtin-dev tooling
    ├── builtin-dev.py     # CLI: add / remove / patch stdlib builtins
    ├── builtin_dev/       # Library for wiring compiler + stdlib methods
    └── *.py               # One-off generators (ABI, docs, suite, …)
```

Root [`Makefile`](../Makefile) includes all `make/*.mk` files and defines `make help`.

---

## Makefile modules — what each file does

| File | Responsibility |
|------|----------------|
| [`common.mk`](../make/common.mk) | `ROOT`, `TARGET_DIR`, `NYRA_BIN`, `MAKE_PY`, log helpers |
| [`nyra.mk`](../make/nyra.mk) | `build-cli`, `ensure-nyra` — builds `target/debug/nyra` |
| [`build.mk`](../make/build.mk) | `bench`, `build-webdocs`, `package-vscode`, `package-release` |
| [`install.mk`](../make/install.mk) | `install-dev` (cargo install + stdlib sync), `install-release`, `install-llvm` |
| [`test.mk`](../make/test.mk) | Individual gates: `test-nyra-lang`, `test-compiletest`, `test-conformance`, fuzz, sanitizer, perf |
| [`smoke.mk`](../make/smoke.mk) | Fast smokes: `smoke-cli`, `smoke-stdlib`, `smoke-examples`, cross-compile smokes |
| [`generators.mk`](../make/generators.mk) | All `make/py/*.py` wrappers (ABI, docs, builtins, …) |
| [`test-all.mk`](../make/test-all.mk) | `make test-all` — ordered fast → slow gates; logs to `target/test-all.txt` |
| [`test-platform*.mk`](../make/test-platform.mk) | Platform CI cores (`test-all-linux`, `test-all-macos`, …) |
| [`release.mk`](../make/release.mk) | `make dist`, `verify-dist` — release tarballs |

### Common `make` targets (contributors)

| Target | When to use |
|--------|-------------|
| `make help` | List common targets |
| `make test-preflight` | Fast smoke (~1–3 min) before a PR |
| `make test-triage` | Run common CI gates; failures in `target/.nyra-test-all-failures` |
| `make test-all` | Full suite (same as CI core) |
| `make test-nyra-lang` | Native Nyra tests in `tests/nyra/` |
| `make build-workspace` | `cargo build --workspace` |
| `make build-cli` | Build `target/debug/nyra` only |
| `make install-dev` | **Install CLI to PATH** — `cargo install --path cli --force` + stdlib sync |
| `make gen-abi-header` | Regenerate `stdlib/nyra_rt.h` from `docs/abi-manifest.toml` |
| `make gen-bindings-doc` | Regenerate `docs/bindings.md` + `webDocs/bindings.html` |
| `make add-builtin` | Wire a new string/array builtin (interactive wizard by default) |
| `make remove-builtin` | Remove a wired builtin |
| `make patch-builtin` | Update wiring of an existing builtin |

**Important:** `cargo build -p cli` updates `./target/debug/nyra` only. Commands like `nyra test` use **`~/.cargo/bin/nyra`** unless you reinstall:

```bash
make install-dev
# or: cargo install --path cli --force
```

---

## Python scripts (`make/py/`) — catalog

Always prefer **`make <target>`** from [`generators.mk`](../make/generators.mk).

### ABI & bindings

| Script | Make target | Output / purpose |
|--------|-------------|------------------|
| [`gen-abi-header.py`](../make/py/gen-abi-header.py) | `make gen-abi-header` | Reads `docs/abi-manifest.toml` → writes `stdlib/nyra_rt.h` (stable C ABI header) |
| [`gen-bindings-doc.py`](../make/py/gen-bindings-doc.py) | `make gen-bindings-doc` | Manifest + stdlib scan → `docs/bindings.md`, `webDocs/bindings.html` |

**When:** after adding/changing entries in `docs/abi-manifest.toml` (new `extern fn` / stable symbol).

**TOML note:** ABI blocks added by `add-builtin --stable-abi` use `# [builtin-dev:…]` comments (TOML syntax), not `//`.

### Builtin developer tooling (`make/py/builtin_dev/`)

Automations to **add**, **remove**, or **patch** stdlib methods (e.g. `.strip_suffix()`) without hand-editing every compiler file. Each run prints a **monitor report** (what changed, your tasks, usage examples).

| Script / module | Role |
|-----------------|------|
| [`builtin-dev.py`](../make/py/builtin-dev.py) | Main CLI: `add`, `remove`, `patch` subcommands |
| [`add-builtin.py`](../make/py/add-builtin.py) | Makefile shortcut → `builtin-dev.py add` |
| [`remove-builtin.py`](../make/py/remove-builtin.py) | Makefile shortcut → `remove` |
| [`patch-builtin.py`](../make/py/patch-builtin.py) | Makefile shortcut → `patch` |
| [`builtin_dev/add.py`](../make/py/builtin_dev/add.py) | **Wire ADD** — patches rt C, stdlib, typecheck, codegen, ownership, examples, tests |
| [`builtin_dev/remove.py`](../make/py/builtin_dev/remove.py) | **Wire REMOVE** — reverses add (incl. legacy hand-wired code) |
| [`builtin_dev/wire_patch.py`](../make/py/builtin_dev/wire_patch.py) | **Wire PATCH** — re-wire + preserve C body when method name unchanged |
| [`builtin_dev/wizard_prompts.py`](../make/py/builtin_dev/wizard_prompts.py) | Interactive wizard — explains what each answer controls |
| [`builtin_dev/method_catalog.py`](../make/py/builtin_dev/method_catalog.py) | Known methods: behavior, examples, default args/returns |
| [`builtin_dev/monitor_report.py`](../make/py/builtin_dev/monitor_report.py) | Terminal monitor output after add/remove/patch |
| [`builtin_dev/discover.py`](../make/py/builtin_dev/discover.py) | Scan repo for `[builtin-dev:method:receiver]` markers |
| [`builtin_dev/spec.py`](../make/py/builtin_dev/spec.py) | `BuiltinSpec` — method, args, C symbol, receiver kind |
| [`builtin_dev/templates.py`](../make/py/builtin_dev/templates.py) | Code templates (C stubs, Rust match arms, Nyra wrappers) |
| [`builtin_dev/patch.py`](../make/py/builtin_dev/patch.py) | Safe file patching (marked blocks, match arms) |
| [`builtin_dev/paths.py`](../make/py/builtin_dev/paths.py) | Repo path map (compiler/stdlib file locations) |
| [`builtin_dev/examples/*.json`](../make/py/builtin_dev/examples/) | Ready-made specs (e.g. `strip_suffix.json`) |

### Contributor hub (`make/py/contrib_dev/`)

Unified **`make contribute`** menu for common contribution scaffolds (stdlib pure/extern, tests, NyraPkg, CLI, conformance). Built-in methods (option 3) delegate to `make add-builtin`.

| Script / module | Role |
|-----------------|------|
| [`contribute.py`](../make/py/contribute.py) | Hub CLI — interactive menu or `--recipe` + `--config` |
| [`contrib_dev/recipes/`](../make/py/contrib_dev/recipes/) | One recipe per menu item |
| [`contrib_dev/examples/*.json`](../make/py/contrib_dev/examples/) | JSON specs for non-interactive runs |

```bash
make contribute
make contribute ARGS='add --recipe test-example --config make/py/contrib_dev/examples/test_example.json'
make contribute-list
make contribute-remove ARGS='-i'
make test-contrib-py
```

Full details: [`make/py/contrib_dev/README.md`](../make/py/contrib_dev/README.md).

**Typical workflow — new string method:**

```bash
make add-builtin                              # wizard (default)
make add-builtin ARGS='--config make/py/builtin_dev/examples/strip_suffix.json'
# 1. Implement C in stdlib/rt/rt_strings.c ([builtin-dev:…] block)
# 2. Fix test expectations in tests/nyra/
make install-dev
nyra test tests/nyra/string_strip_suffix_test.ny
nyra run examples/builtins/strings/strip_suffix.ny
make gen-abi-header && make gen-bindings-doc   # if stable ABI was enabled
```

Full details: [`make/py/builtin_dev/README.md`](../make/py/builtin_dev/README.md).

**When to use builtins tooling vs manual:** use for **string methods** that need typecheck + LLVM codegen + C runtime + `.method()` wrapper. Pure Nyra stdlib helpers (pattern A in CONTRIBUTING) do not need this.

### Compiletest & examples generation

| Script | Make target | Purpose |
|--------|-------------|---------|
| [`gen-suite-tests.py`](../make/py/gen-suite-tests.py) | `make gen-suite-tests` | Generate combinatorial tests under `tests/suite/*/generated/` |
| [`gen-typed-examples.py`](../make/py/gen-typed-examples.py) | `make gen-typed-examples` | Generate `.typed.ny` companions for examples |
| [`gen-comparison-extended.py`](../make/py/gen-comparison-extended.py) | `make gen-comparison-extended` | Extended cross-language comparison sources |
| [`sync-comparison-typed.py`](../make/py/sync-comparison-typed.py) | `make sync-comparison-typed` | Sync typed variants for comparison benchmarks |
| [`bump-comparison-hardness.py`](../make/py/bump-comparison-hardness.py) | `make bump-comparison-hardness` | Adjust comparison benchmark difficulty |

**When:** changing compiletest grid size/profile; CI may auto-regenerate on count mismatch (`test-compiletest`).

### webDocs & snippets

| Script | Make target | Purpose |
|--------|-------------|---------|
| [`sync-webdocs-code-tabs.py`](../make/py/sync-webdocs-code-tabs.py) | `make sync-webdocs-code-tabs` | Sync zero-types / typed code-tab pairs in `webDocs/*.html` |
| [`snippet-types.py`](../make/py/snippet-types.py) | `make snippet-types` | Add explicit types to Nyra snippets (used by sync script) |
| [`strip-apps-types.py`](../make/py/strip-apps-types.py) | `make strip-apps-types` | Strip types from Apps sources (maintenance) |
| [`strip-nyra-symbol-prefix.py`](../make/py/strip-nyra-symbol-prefix.py) | `make strip-nyra-symbol-prefix` | Strip symbol prefixes from generated/debug output |

**When:** editing HTML docs with paired easy/typed code blocks; run `make test-webdocs-tabs` to verify.

### Benchmarks & README

| Script | Make target | Purpose |
|--------|-------------|---------|
| [`bench_comparison_html.py`](../make/py/bench_comparison_html.py) | `make bench-comparison-html` | HTML report → `examples/comparison/results/latest.html` |
| [`bench_measure.py`](../make/py/bench_measure.py) | (via `make/lib/bench.sh`) | Measure benchmark timings |
| [`bench_report.py`](../make/py/bench_report.py) | (via bench pipeline) | Aggregate benchmark results |
| [`update-readme-bench.py`](../make/py/update-readme-bench.py) | `make update-readme-bench` | Update README benchmark section from results |

**When:** publishing comparison benchmark results (`make bench`).

### Misc generators

| Script | Make target | Purpose |
|--------|-------------|---------|
| [`gen-ar-file-index.py`](../make/py/gen-ar-file-index.py) | `make gen-ar-file-index` | Generate archive/file index docs |
| [`gen-abi-header.py`](../make/py/gen-abi-header.py) | (see ABI above) | |
| [`gen-bindings-doc.py`](../make/py/gen-bindings-doc.py) | (see ABI above) | |

---

## Who should edit what?

| You want to… | Edit | Run |
|--------------|------|-----|
| Add a **string method** (`.foo()`) | Prefer `make add-builtin`; then C impl in `stdlib/rt/` | `make install-dev` · `nyra test` |
| Add **stable C symbol** to public ABI | `docs/abi-manifest.toml` | `make gen-abi-header` · `make gen-bindings-doc` |
| Regenerate **compiletest grid** | — | `make gen-suite-tests GEN_SUITE_ARGS="--profile ci"` |
| Fix **doc code tabs** (easy + typed) | `webDocs/*.html` | `make sync-webdocs-code-tabs` |
| Improve **builtin wizard / templates** | `make/py/builtin_dev/*.py` | `python3 -m py_compile make/py/builtin_dev/*.py` |
| Add a **new generator** | New `make/py/foo.py` + target in `generators.mk` | Document here + `make help` if user-facing |
| Wire generator into **CI** | `make/test-all.mk` or `make/test.mk` | `make test-all` |

---

## Adding a new Python generator (checklist)

1. Add `make/py/your-script.py` with a module docstring (purpose, inputs, outputs).
2. Add a `.PHONY` target in [`make/generators.mk`](../make/generators.mk).
3. If CI should run it, wire into [`make/test.mk`](../make/test.mk) or [`make/test-all.mk`](../make/test-all.mk).
4. Document the script in **this file**.
5. Mention in `make help` if contributors run it often.

---

## Shell helpers (`make/lib/`)

Not Python, but related: shell scripts invoked by Make for test orchestration, install, and benchmarks.

| Script | Used by | Purpose |
|--------|---------|---------|
| `test-all-gate.sh` | `test-all.mk` | Run one gate; record pass/fail; continue on failure |
| `test-all-progress.sh` | `test-all.mk` | Progress + timing for long test-all runs |
| `updateLang.sh` | `install-dev` | Build + `cargo install --path cli --force` |
| `bench.sh` | `make bench` | Cross-language benchmark driver |
| `build-webdocs.sh` | `build-webdocs` | Rebuild search index + skill markdown |
| `suite-clean.sh` | `test-compiletest` | Clean stale suite artifacts |

---

## Further reading

| Topic | Document |
|-------|----------|
| What to change in compiler/stdlib | [`contributor-map.md`](contributor-map.md) |
| Full contributing guide | [`../CONTRIBUTING.md`](../CONTRIBUTING.md) |
| Testing & CI | [`testing-runbook.md`](testing-runbook.md) |
| ABI policy | [`abi-manifest.toml`](abi-manifest.toml) · [`bindings.md`](bindings.md) |
| Builtin dev (detailed) | [`../make/py/builtin_dev/README.md`](../make/py/builtin_dev/README.md) |
