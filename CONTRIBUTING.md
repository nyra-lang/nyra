# Contributing to Nyra

Thank you for helping build Nyra. This guide explains **where things live**, **how to run and test changes**, and **what we expect in pull requests**.

Nyra is actively developed (current toolchain version: see `Cargo.toml`, e.g. **1.9.x**). The compiler, CLI, stdlib, and docs evolve quickly. When in doubt, open an issue or a small PR and ask.

---

## Start here — documentation map

| You want to… | Read |
|--------------|------|
| **Understand Nyra syntax & semantics** | [`webDocs/nyra-skill.md`](webDocs/nyra-skill.md) (canonical language reference) |
| **Know where compiler code goes** | [`docs/architecture.md`](docs/architecture.md) |
| **Understand stdlib layout & auto-prelude** | [`stdlib/README.md`](stdlib/README.md) |
| **Run the full test suite & debug CI** | [`docs/testing-runbook.md`](docs/testing-runbook.md) |
| **Ship a language/stdlib change (version + webDocs)** | [`agents/skill.md`](agents/skill.md) |
| **Mandatory change checklist (short)** | [`skills/guidelines.md`](skills/guidelines.md) · [`.cursor/rules/nyra-guidelines.mdc`](.cursor/rules/nyra-guidelines.mdc) |
| **Feature depth & stability tiers** | [`docs/status.md`](docs/status.md) · [`docs/stability-v1.md`](docs/stability-v1.md) |
| **FFI / C ABI symbols** | [`docs/abi-manifest.toml`](docs/abi-manifest.toml) · [`docs/bindings.md`](docs/bindings.md) |
| **Roadmap** | [`docs/roadmap-stable.md`](docs/roadmap-stable.md) |

**Design philosophy (read before changing the language):**

- **Zero-types by default** — like Go/JavaScript; type annotations are optional.
- **Inference first** — the compiler infers types from usage; if it cannot, compilation stops with `E004` and asks for a manual annotation (rare).
- **Both styles must work** — every user-visible feature must pass tests **without types** and **with explicit types** (`foo.ny` + `foo.typed.ny` examples).
- **Performance & memory** — primary goals; stdlib uses small modules + demand-driven linking so LLVM can eliminate dead code.

---

## Mandatory checklist (language / stdlib / CLI / runtime changes)

Complete this before merging any change that affects user-visible behavior:

| # | Requirement |
|---|-------------|
| 1 | **Tests** — add or update coverage; run `make test-all` (or at minimum `cargo test --workspace` + affected Nyra tests). Test **zero-types and explicit types**. |
| 2 | **Examples** — add or update under `examples/` (`feature.ny` + `feature.typed.ny` when applicable). |
| 3 | **No regressions** — unrelated features still pass. |
| 4 | **webDocs** — update [`webDocs/`](webDocs/) when syntax, stdlib, CLI, or ABI changes; rebuild skill + search index (see [Release workflow](#release-workflow-version--webdocs)). |
| 5 | **Makefile** — wire new test gates into the root `Makefile` (`make test-all` dependencies). |
| 6 | **Version** — bump `[workspace.package] version` in [`Cargo.toml`](Cargo.toml) + [`CHANGELOG.md`](CHANGELOG.md) for language updates (see [`agents/skill.md`](agents/skill.md)). |
| 7 | **Status** — update [`docs/status.md`](docs/status.md) when feature depth changes. |

Docs-only PRs need step 4 only.

---

## Where to edit (quick reference)

| Change type | Primary locations |
|-------------|-------------------|
| **New keyword / syntax** | `compiler/lexer/` → `compiler/parser/` → `compiler/expand/` → `grammar/nyra.tmLanguage.json` |
| **Type rules / inference** | `compiler/typecheck/` · `compiler/types/` |
| **Ownership / borrow errors** | `compiler/ownership/` · `compiler/borrowck/` |
| **Generics / monomorph** | `compiler/monomorph/` |
| **LLVM IR / codegen** | `compiler/codegen/` |
| **Imports / multi-file / prelude** | `compiler/resolve/` (`prelude.rs`, `symbols.rs`) |
| **Stdlib Nyra API** | `stdlib/**/*.ny` |
| **Stdlib C runtime** | `stdlib/rt/rt_*.c` · register in [`compiler/codegen/src/runtime_map.rs`](compiler/codegen/src/runtime_map.rs) |
| **Compiler math intrinsics** (`abs_i32`, …) | `compiler/types/src/intrinsics.rs` + codegen (not stdlib) |
| **Language builtins** (`print`, …) | `compiler/typecheck/` · `compiler/codegen/` |
| **CLI flags** | `cli/src/commands/` |
| **NyraPkg** | `pkg/` · `cli/src/commands/pkg*` |
| **Conformance contracts** | `tests/conformance/` · `compiler/driver/tests/conformance/` |

Full pipeline order: [`docs/architecture.md`](docs/architecture.md).

---

## How to add a stdlib function

Most new APIs live in **stdlib**, not the compiler. Pick the pattern:

### A — Pure Nyra wrapper (no new C)

Add a top-level `fn` in the right module, e.g. [`stdlib/json/mod.ny`](stdlib/json/mod.ny):

```ny
fn decode_i32(json: string, key: string) -> i32 {
    return json_get_i32(json, key)
}
```

No manual prelude registration — the compiler builds a **virtual symbol table** from all `stdlib/**/*.ny` files and lazy-loads only what your program uses ([`stdlib/README.md`](stdlib/README.md) · `compiler/resolve/src/prelude.rs`).

### B — `extern fn` + C runtime (typical for I/O, JSON, crypto)

1. **Declare** in `stdlib/<module>.ny`:
   ```ny
   extern fn json_get_i32(json: string, key: string) -> i32
   ```
2. **Implement** in `stdlib/rt/rt_<area>.c` (e.g. `rt_json.c`).
3. **Register** the symbol in [`compiler/codegen/src/runtime_map.rs`](compiler/codegen/src/runtime_map.rs):
   ```rust
   ("json_get_i32", "rt_json.c"),
   ```
4. **ABI** — add entry to [`docs/abi-manifest.toml`](docs/abi-manifest.toml); run `make gen-abi-header` → updates `stdlib/nyra_rt.h`; extend [`compiler/driver/tests/abi_manifest.rs`](compiler/driver/tests/abi_manifest.rs).
5. **Test** in `tests/nyra/<feature>_test.ny`.
6. **Example** — `examples/builtins/.../foo.ny` + `foo.typed.ny`.
7. **Docs** — [`webDocs/stdlib.html`](webDocs/stdlib.html) · [`webDocs/bindings.html`](webDocs/bindings.html).

Optional friendly names: thin wrappers in `stdlib/builtins_*.ny` (e.g. [`stdlib/builtins_json.ny`](stdlib/builtins_json.ny)).

### C — Compiler intrinsic (rare)

For ops lowered directly to LLVM (`abs_i32`, `min_i32`, …): edit `compiler/types/src/intrinsics.rs` and codegen — see `examples/builtins/math_intrinsics.ny`.

### Stdlib design rules

- **Small files** — micro-modules under `stdlib/`; avoid monolithic files.
- **Types optional** — APIs must work with inference (strings, numbers, arrays, objects, booleans).
- **Static dispatch** — prefer monomorph/generics over dynamic dispatch for LLVM inlining.
- **NyraPkg** — community packages live in `examples/packages/`; proven modules may graduate into stdlib.

---

## Ways to contribute

| Area | You can… |
|------|-----------|
| **Language** | Fix bugs in lexer/parser/typecheck/borrow/codegen; add tests in `compiler/driver/tests/` — see [`docs/architecture.md`](docs/architecture.md) |
| **Stdlib** | Add modules under `stdlib/` + `stdlib/rt/` — see [How to add a stdlib function](#how-to-add-a-stdlib-function) |
| **Examples** | Add or improve `.ny` samples under `examples/` (zero-types + typed pairs) |
| **Apps** | Extend multi-file projects under [`Apps/`](Apps/) (Basics, Graphics, FileSystem, learn, …) |
| **Tooling** | CLI (`cli/`), formatter, `nyra diag`, `nyra lsp`, NyraPkg (`pkg/`) |
| **Docs** | `docs/`, `webDocs/`, `install.md`, `grammar/README.md` |
| **Grammar** | Update [`grammar/nyra.tmLanguage.json`](grammar/nyra.tmLanguage.json) when keywords change |
| **Benchmarks** | Fair cross-language benches in `examples/comparison/` |
| **Runtime** | C runtime `stdlib/rt/`, headers `stdlib/nyra_rt.h`, Rust helpers `rt/` |

---

## Repository map

```
Nyra/
├── compiler/          # Compiler pipeline (workspace crates)
│   ├── driver/        # Public API (`compiler` crate): orchestration + tests
│   ├── lexer/ parser/ expand/ resolve/ monomorph/
│   ├── typecheck/ types/ ownership/ borrowck/ const_eval/
│   └── codegen/       # LLVM IR + runtime_map.rs
├── cli/               # `nyra` binary (run, build, check, test, fmt, pkg, lsp, diag)
├── lsp/               # Language server (via `nyra lsp`)
├── rt/                # Rust runtime hooks (spawn, async MVP)
├── pkg/ pkg-registry/ # NyraPkg lock/sync + dev registry
├── stdlib/            # .ny modules + rt/*.c C runtime
├── tests/
│   ├── nyra/          # Native Nyra tests (`nyra test tests/nyra`)
│   ├── suite/         # Compiletest pass/fail/run corpus
│   └── conformance/   # CONF-LANG pass/fail/fixtures
├── Apps/              # Reference multi-file applications
├── examples/          # Samples, builtins, comparison benchmarks
├── docs/              # Architecture, status, ABI, testing runbook
├── webDocs/           # Static HTML site + nyra-skill.md
├── skills/            # Contributor guidelines & design notes
├── Makefile           # Primary entry (make test-all, make help, …)
├── make/              # Modular Make targets, lib recipes, py generators
├── scripts/           # install.sh (curl), install.ps1 only
└── benchmarks/        # CI perf baselines
```

**Compiler pipeline (compile order):** lexer → parser → expand → resolve → monomorph → typecheck → ownership → borrowck → const_eval → codegen. Details: [`docs/architecture.md`](docs/architecture.md).

---

## Reference application — Dungeon Steps

[`examples/comparison/dungeon/`](examples/comparison/dungeon/) is the canonical **Dungeon Steps** multi-module demo: imports, `const`, `struct`, `enum`, `match`, loops, and `test fn`. The same layout is used for cross-language benchmarks (Nyra, Go, Rust, JS, Python, Java, C, C++).

| Path | Role |
|------|------|
| `examples/comparison/dungeon/main.ny` | Entry point |
| `examples/comparison/dungeon/src/config.ny` | Constants |
| `examples/comparison/dungeon/src/types.ny` | `enum`, `struct` |
| `examples/comparison/dungeon/src/world.ny` | Map / movement |
| `examples/comparison/dungeon/src/engine.ny` | Game loop + `test fn` |

**Run:**

```bash
nyra run examples/comparison/dungeon
# or without install:
cargo run -p cli -- run examples/comparison/dungeon
```

Expected output starts with `Dungeon Steps`, then score lines and `3`. See [`examples/comparison/dungeon/README.md`](examples/comparison/dungeon/README.md) and [webDocs/dungeon-steps.html](webDocs/dungeon-steps.html).

Larger app collections live under [`Apps/`](Apps/) (Basics algorithms, Graphics, GhostTerm, FileSystem tools, learn track, …).

**After compiler changes**, reinstall the CLI:

```bash
./scripts/updateLang.sh   # or: make install-dev
# equivalent: cargo install --path cli --force
nyra --version
```

---

## `examples/` — samples and benchmarks

| Folder | Purpose |
|--------|---------|
| [`examples/syntax/`](examples/syntax/) | Minimal programs (`hello.ny`) — first smoke tests |
| [`examples/builtins/`](examples/builtins/) | Stdlib & builtin demos (often `.ny` + `.typed.ny` pairs) |
| [`examples/language_features/`](examples/language_features/) | Enum + `match` demo |
| [`examples/projects/`](examples/projects/) | Small apps: calculator, HTTP hello, read_file |
| [`examples/ffi/`](examples/ffi/) | `extern fn` + Rust cdylib sample |
| [`examples/comparison/`](examples/comparison/) | Fair benches — same algorithm across languages |
| [`examples/packages/`](examples/packages/) | NyraPkg community packages |

**Quick commands:**

```bash
cargo run -p cli -- run examples/syntax/hello.ny
cargo run -p cli -- check examples/syntax/math.ny
nyra test tests/nyra

# Comparison smoke
cargo run -p cli -- run examples/comparison/hello/hello.ny

# Full runtime benchmark → examples/comparison/results/latest.html
make bench
```

**Adding a new example**

1. Put files under `examples/<topic>/`.
2. For user-facing features, ship **`foo.ny`** (zero-types) and **`foo.typed.ny`** (explicit types) when both styles apply.
3. Multi-file projects: `main.ny` at project root (or `nyra.mod` for packages).
4. Run `nyra check` and `nyra run` on your paths.
5. Mention in the PR; optionally add to [`webDocs/examples.html`](webDocs/examples.html).

---

## Testing

### One command — full suite

```bash
make test-all
```

Logs to `target/test-all.txt`. Optional: `TEST_PERF=1` for perf gate. See [`docs/testing-runbook.md`](docs/testing-runbook.md) for CI stages, snapshot updates, and rollback policy.

### Test layers

| Layer | Location | How to run |
|-------|----------|------------|
| **Rust unit/integration** | `compiler/**`, `cli/`, … | `cargo test --workspace` |
| **Driver integration** | `compiler/driver/tests/` | `cargo test -p compiler` |
| **Codegen/diagnostic snapshots** | `compiler/driver/tests/snapshots/` | `INSTA_UPDATE=1 cargo test -p compiler --test codegen_snapshots` (review diff!) |
| **Compiletest corpus** | `tests/suite/` | `cargo test -p compiler suite_` |
| **Native Nyra tests** | `tests/nyra/` | `nyra test tests/nyra` · `make test-nyra-lang` |
| **Conformance (CONF-LANG)** | `tests/conformance/` | `make test-conformance` |
| **Example corpus** | `examples/` | wired in CI / `make test-all` |
| **ABI roundtrip** | manifest + header | `make test-abi-roundtrip` |
| **Apps smoke** | `Apps/Basics`, `Apps/Graphics` | part of `make test-all` |
| **Fuzz smoke** | `fuzz/` | `make test-fuzz-smoke` |

**Zero-types + typed:** language and stdlib changes must work in both styles. Add paired examples and, where relevant, both untyped and typed test programs.

### Quick iteration

```bash
cargo test -p compiler
cargo run -p cli -- check path/to/file.ny
nyra test tests/nyra/my_feature_test.ny
```

---

## Workspace crates (Rust)

| Crate | Responsibility |
|-------|----------------|
| `compiler` | Driver + public API; tests in `compiler/driver/tests/` |
| `cli` | User-facing `nyra` binary; linking via `clang` + stdlib C runtime |
| `lsp` | LSP library (in-process via `nyra lsp`) |
| `rt` | Optional Rust runtime symbols |
| `pkg` | `nyra.mod` / lock parsing |
| `pkg-registry` | Dev registry on port 9470 |

```bash
cargo build --workspace
cargo test --workspace
```

---

## Development setup

1. Install [Rust](https://rustup.rs/) (stable).
2. Install **clang** and **libclang** (Xcode CLT on macOS; on Linux: `clang` + `libclang-dev` for `nyra bind c` / `nyra-c-bindgen`).
3. Clone and build:

```bash
git clone https://github.com/hamdymohamedak/Nyra.git
cd Nyra
cargo build --workspace
```

4. Install `nyra` on your PATH:

```bash
./scripts/updateLang.sh   # or: make install-dev
nyra --version
```

The root [`run`](run) file lists handy one-liners (examples, bench, test).

---

## Nyra CLI cheat sheet

| Command | Use |
|---------|-----|
| `nyra run <file or dir>` | Compile, link, execute |
| `nyra build <file or dir>` | Emit binary under `target/debug` or `target/release` |
| `nyra check <path>` | Typecheck + borrow (no codegen) |
| `nyra diag <path> [--json]` | Diagnostics for editors |
| `nyra test [path]` | Run `test fn` / `test_*` / `*_test.ny` |
| `nyra fmt [--write] <path>` | Format `.ny` sources |
| `nyra build --release` | `-O3` + LLVM opt + thin LTO |
| `nyra build --for windows\|linux\|macos` | Cross-compile (see `webDocs/targets.html`) |
| `nyra build --target wasm32-wasi` | Wasm subset (`stdlib/nyra_rt_wasi.c`) |
| `nyra check --deny-extended` | Core-only CI (reject Extended tier features) |
| `nyra pkg init` / `verify` / `build` | NyraPkg workflow |
| `nyra lsp` | Language server (stdio) |

Details: [`webDocs/tooling.html`](webDocs/tooling.html) · [`install.md`](install.md).

---

## Editor / syntax highlighting

When you add a keyword to the lexer (`compiler/lexer/src/lib.rs`), update in the same PR:

- [`grammar/nyra.tmLanguage.json`](grammar/nyra.tmLanguage.json)
- [`grammar/README.md`](grammar/README.md) (VS Code / Cursor setup)

---

## Pull requests

1. **Branch** from `main`; keep PRs focused (one concern per PR when possible).
2. **Checklist** — complete the [mandatory checklist](#mandatory-checklist-language--stdlib--cli--runtime-changes) above.
3. **Tests** — `cargo test --workspace` at minimum; `make test-all` for language/stdlib work.
4. **Examples** — user-visible behavior needs `examples/` updates (zero-types + typed where applicable).
5. **Docs** — `docs/` and `webDocs/`; feature depth → [`docs/status.md`](docs/status.md).
6. **Style** — `cargo fmt` on touched Rust files.
7. **CI** — see [`.github/workflows/ci.yml`](.github/workflows/ci.yml) and [`docs/testing-runbook.md`](docs/testing-runbook.md).

**Parser / ABI policy:**

- Do not change parser behavior for Core-tier syntax without discussion and tests.
- **Breaking** FFI ABI changes require explicit review; **adding** stable symbols follows [`docs/abi-manifest.toml`](docs/abi-manifest.toml) + version bump.
- Extended-tier features (`async`, traits, macros, enum payloads with storage, `defer`, …) may emit `warning[W001]`; see [`docs/stability-v1.md`](docs/stability-v1.md).

---

## Release workflow (version + webDocs)

For any user-visible language/stdlib/CLI/ABI change:

1. Bump **`[workspace.package] version`** in [`Cargo.toml`](Cargo.toml) (minor bump for language updates after 1.0 — see [`agents/skill.md`](agents/skill.md)).
2. Add section to [`CHANGELOG.md`](CHANGELOG.md).
3. Update relevant `webDocs/*.html` and [`webDocs/nyra-skill.md`](webDocs/nyra-skill.md).
4. Rebuild derived docs:

```bash
node webDocs/scripts/build-nyra-skill.mjs    # → skills/skill.md
node webDocs/scripts/build-search-index.mjs
# or: make build-webdocs
```

5. Update [`docs/status.md`](docs/status.md) when feature depth changes.

---

## Backend / stdlib runtime checklist

For async, TCP, HTTP, JSON, TLS, crypto, and other runtime-backed stdlib APIs:

1. **Nyra stub** — `stdlib/<area>/*.ny` with `fn` and/or `extern fn`.
2. **C runtime** — `stdlib/rt/rt_*.c`; register every `extern` symbol in [`compiler/codegen/src/runtime_map.rs`](compiler/codegen/src/runtime_map.rs).
3. **ABI** — [`docs/abi-manifest.toml`](docs/abi-manifest.toml) + `make gen-abi-header` + [`compiler/driver/tests/abi_manifest.rs`](compiler/driver/tests/abi_manifest.rs).
4. **Integration test** — `compiler/driver/tests/integration.rs` or `nyra run` on an example.
5. **Example** — `examples/builtins/` or `examples/projects/`.
6. **Docs** — [`webDocs/stdlib.html`](webDocs/stdlib.html), [`webDocs/backend.html`](webDocs/backend.html) if applicable.
7. **Reinstall** — `./scripts/updateLang.sh   # or: make install-dev` after pulling runtime changes.

DB drivers that need heavy native deps often start in **NyraPkg** (`examples/packages/ny-sqlite/`) before graduating into stdlib.

---

## Reporting issues

Open a [GitHub issue](https://github.com/hamdymohamedak/Nyra/issues) with:

- A **minimal `.ny` reproducer** (or path to a failing example).
- Output of `nyra check <file>` and/or `nyra run <file>`.
- OS, `nyra --version`, and `clang --version` if linking fails.

---

## Performance work

- Local bench: `make bench` → `examples/comparison/results/latest.html`
- CI smoke: `make test-perf` vs `benchmarks/ci-baseline.json`

Do not commit large generated bench artifacts unless the PR explicitly updates published results.

---

## Naming conventions (Rust workspace)

1. **Folder name = crate name** for compiler stages (`lexer`, `borrowck`, `cli`) — no `nyra-` prefix on directories.
2. **User-facing binary** stays `nyra`; C runtime entry stays `nyra_rt.c` / `nyra_rt.h` for ABI stability.
3. **Public compiler API** is exported from the `compiler` driver crate only.
4. **Integration tests** live in `compiler/driver/tests/`.
5. **Split large files** before they exceed ~800–1200 lines (see [`docs/architecture.md`](docs/architecture.md)).

---

## Further reading

| Topic | Document |
|-------|----------|
| Language reference (AI + humans) | [`webDocs/nyra-skill.md`](webDocs/nyra-skill.md) |
| Toolchain architecture | [`docs/architecture.md`](docs/architecture.md) |
| Stdlib design & auto-prelude | [`stdlib/README.md`](stdlib/README.md) |
| Testing & CI debugging | [`docs/testing-runbook.md`](docs/testing-runbook.md) |
| Stability tiers (Core vs Extended) | [`docs/stability-v1.md`](docs/stability-v1.md) |
| Native C / `nyra cc` | [`docs/native-cc.md`](docs/native-cc.md) |
| C bindgen | [`docs/c-bindgen.md`](docs/c-bindgen.md) |
| Roadmap | [`docs/roadmap-stable.md`](docs/roadmap-stable.md) |
| Design sketches | [`skills/`](skills/) |

Thank you for contributing to Nyra.
