# Contributing to Nyra

Thank you for helping build Nyra. This guide explains **where things live**, **how to run and test changes**, and **what we expect in pull requests**.

Nyra is actively developed (current toolchain version: see `[workspace.package] version` in [`Cargo.toml`](Cargo.toml)). The compiler, CLI, stdlib, and docs evolve quickly. When in doubt, open an issue or a small PR and ask.

---

## Table of contents

1. [Your first contribution](#your-first-contribution-10-minutes)
2. [How Nyra programs become binaries](#how-nyra-programs-become-binaries)
3. [Contributor personas](#contributor-personas-who-edits-what)
4. [What to change → where to go](#what-to-change--where-to-go)
5. [Documentation map](#start-here--documentation-map)
6. [Mandatory checklist](#mandatory-checklist-language--stdlib--cli--runtime-changes)
7. [Where to edit](#where-to-edit-quick-reference)
8. [How to add a stdlib function](#how-to-add-a-stdlib-function) — patterns A–D + [strip_suffix case study](#case-study-strip_suffix-end-to-end)
9. [Contributor hub guide (`make contribute`)](#contributor-hub-guide-make-contribute) — menu, questions, full simulation
10. [Makefile & Python generators](#makefile--python-generators-make)
11. [Repository map](#repository-map)
12. [Testing](#testing) (+ [decision tree](#test-decision-tree))
13. [Documentation sources](#documentation-where-to-edit-what)
14. [Version bump policy](#version-bump-policy)
15. [Troubleshooting & FAQ](#troubleshooting--faq)
16. [Glossary](#glossary)
17. [IDE & diagnostics](#ide--diagnostics-tooling)
18. [CI overview](#ci-overview-what-runs-on-prs)
19. [NyraPkg workflow](#nyrapkg-workflow)
20. [Removing a feature](#removing-a-feature)
21. [Debugging the compiler](#debugging-the-compiler)
22. [Development setup · CLI · PRs · Release](#development-setup)
23. [Reporting issues · Performance · Naming · License](#reporting-issues)

**Suggested reading order (new contributor):**

1. [Your first contribution](#your-first-contribution-10-minutes) — clone, build, install-dev, smoke test
2. [Contributor personas](#contributor-personas-who-edits-what) — pick your path (stdlib vs compiler vs docs)
3. [Contributor hub guide](#contributor-hub-guide-make-contribute) — `make contribute` menu, questions, full simulation
4. [`docs/contributor-map.md`](docs/contributor-map.md) — flowchart for “where do I edit?”
5. [How to add a stdlib function](#how-to-add-a-stdlib-function) — patterns A–D + [strip_suffix case study](#case-study-strip_suffix-end-to-end)
6. [Testing](#testing) + [test decision tree](#test-decision-tree)
7. [Troubleshooting & FAQ](#troubleshooting--faq) — when `nyra test` fails but `nyra run` works
8. [Debugging the compiler](#debugging-the-compiler) — snapshots, single-crate tests
9. [Glossary](#glossary) — terms you will see in PRs and docs

---

## Your first contribution (10 minutes)

Minimal path from clone to a verified change:

```bash
git clone git@github.com:nyra-lang/nyra.git && cd nyra
cargo build --workspace
make install-dev              # installs `nyra` to ~/.cargo/bin — required!
nyra --version                # must match your built toolchain

# Quick smoke
nyra run examples/syntax/hello.ny
make test-preflight           # fast gate (~1–3 min)

# After changing compiler or stdlib wiring:
make install-dev              # again — cargo build alone is NOT enough
nyra test tests/nyra/         # or a single *_test.ny file
```

**Two binaries, one name — know the difference:**

| Command | What it updates | Used when you type `nyra …` |
|---------|-----------------|-----------------------------|
| `cargo build -p cli` | `./target/debug/nyra` only | Only if you call that path explicitly |
| `make install-dev` | `~/.cargo/bin/nyra` on PATH | **Yes** — default for `nyra test`, `nyra run` |

**Common first-PR flow:**

1. Pick a small issue or doc fix.
2. Edit the file(s) — use [contributor personas](#contributor-personas-who-edits-what) if unsure.
3. Add/update `tests/nyra/*_test.ny` and `examples/` for user-visible behavior.
4. Run `make test-preflight` or targeted tests (see [test decision tree](#test-decision-tree)).
5. Open PR with checklist completed.

---

## How Nyra programs become binaries

High-level path from source to executable (details in [`docs/architecture.md`](docs/architecture.md)):

```
.ny source file(s)
    │
    ▼  resolve/          imports, prelude, project graph (load time)
    │
    ▼  lexer → parser     tokens → AST
    ▼  expand/            desugar (??, ?, async, Vec, …)
    ▼  monomorph/         generics → monomorphic AST
    ▼  typecheck/         types, builtins, diagnostics (E001…)
    ▼  ownership/ borrowck/   drop plan, moves, borrows
    ▼  const_eval/        compile-time evaluation
    ▼  codegen/           LLVM IR (.ll)
    │
    ▼  clang link         stdlib/rt/*.c + nyra runtime → native binary
```

**Kinds of callable symbols (do not confuse them):**

| Kind | Example | Where wired | Typical contributor action |
|------|---------|-------------|----------------------------|
| **Language builtin** | `print(x)` | `compiler/typecheck/`, `codegen/` | Rare; compiler change |
| **String method** | `"hi".trim()` | typecheck + codegen + `stdlib/rt/` + `builtins_string.ny` | `make add-builtin` |
| **Stdlib `extern fn`** | `json_get_i32(...)` | `stdlib/*.ny` + `rt_*.c` + `runtime_map.rs` | Pattern B below |
| **Pure Nyra stdlib `fn`** | wrapper in `stdlib/json/` | `stdlib/**/*.ny` only | Pattern A below |
| **NyraPkg module** | `import "pkg/…"` | `examples/packages/`, `.nyra/cache` | Package, not stdlib |

**Six value kinds** the language must support (types always optional): strings, numbers, arrays, objects, booleans, and optional type annotations. Test both **zero-types** and **explicit types** (`foo.ny` + `foo.typed.ny`).

**Stability tiers:** Core features are stable CI gates; Extended features (`async`, traits, macros, …) may warn — see [`docs/stability-v1.md`](docs/stability-v1.md). Use `nyra check --deny-extended` to match strict CI.

---

## Contributor personas (who edits what)

| I want to… | Primary folders | Read first | Test with |
|------------|-----------------|------------|-----------|
| **Fix a stdlib bug (Nyra only)** | `stdlib/**/*.ny` | [`stdlib/README.md`](stdlib/README.md) | `nyra test tests/nyra/…` |
| **Add C-backed stdlib API** | `stdlib/`, `stdlib/rt/`, `runtime_map.rs` | [Pattern B](#b--extern-fn--c-runtime-typical-for-io-json-crypto) | `nyra test` + `make gen-abi-header` |
| **Add string method (`.foo()`)** | via `make add-builtin` or `make contribute` → 3 | [`make/py/builtin_dev/README.md`](make/py/builtin_dev/README.md) | `make install-dev` · `nyra test` |
| **Scaffold stdlib / tests / pkg / syntax** | `make contribute` | [`make/py/contrib_dev/README.md`](make/py/contrib_dev/README.md) | `nyra test` · `make test-contrib-py` |
| **Change syntax / types** | `compiler/lexer` … `codegen` | [`docs/architecture.md`](docs/architecture.md) | `cargo test -p compiler` · `make install-dev` |
| **Fix borrow / ownership error** | `ownership/`, `borrowck/` | contributor-map | `cargo test -p ownership` |
| **Add CLI flag** | `cli/src/app/args.rs`, `cli/src/commands/` | [`docs/architecture.md`](docs/architecture.md#cli-layout) | `cargo test -p cli` |
| **Improve docs site** | `webDocs/` + [docs repo](https://github.com/nyra-lang/docs) | [Docs section](#documentation-where-to-edit-what) | `make build-webdocs` |
| **Add generator / Make target** | `make/py/`, `make/generators.mk` | [`docs/make-and-generators.md`](docs/make-and-generators.md) | relevant `make test-*` |
| **LSP / IDE** | `lsp/`, `cli/src/commands/ide.rs` | architecture | `make smoke-cli` |
| **Large demo app** | `Apps/` | examples vs Apps below | `nyra run Apps/…` |

---

## What to change → where to go

**Canonical guide:** [`docs/contributor-map.md`](docs/contributor-map.md) — full decision flowchart, test placement, `expand/` module index, and large-file split targets.

**Removing a feature:** walk the same crates in reverse, then delete matching entries in `tests/nyra/`, `examples/`, `grammar/`, and docs. See [Removing a feature](#removing-a-feature) for a checklist.

---

## Start here — documentation map

| You want to… | Read |
|--------------|------|
| **Find the right folder for your change** | [`docs/contributor-map.md`](docs/contributor-map.md) |
| **Makefile targets & Python generators (`make/py/`)** | [`docs/make-and-generators.md`](docs/make-and-generators.md) |
| **Understand Nyra syntax & semantics** | [`skills/skill.md`](skills/skill.md) · [live site](https://nyra-lang.github.io/docs/) |
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
- **Batteries included** — crypto, serialization, networking, compression, and similar APIs belong **in-tree** in `stdlib/`, not NyraPkg-only redirects. Heavy native deps (e.g. DB drivers) may start in NyraPkg and graduate into stdlib when proven.

---

## Mandatory checklist (language / stdlib / CLI / runtime changes)

Complete this before merging any change that affects user-visible behavior:

| # | Requirement |
|---|-------------|
| 1 | **Tests** — add or update coverage; run `make test-all` (or at minimum `cargo test --workspace` + affected Nyra tests). Test **zero-types and explicit types**. Re-run **`tests/suite/fail/`** when changing typecheck, comptime, or diagnostics. |
| 2 | **Examples** — add or update under `examples/` (`feature.ny` + `feature.typed.ny` when applicable). |
| 3 | **No regressions** — unrelated features still pass. |
| 4 | **webDocs** — update the [docs repo](https://github.com/nyra-lang/docs) when syntax, stdlib, CLI, or ABI changes; rebuild skill + search index (see [Release workflow](#release-workflow-version--webdocs)). Published site: [nyra-lang.github.io/docs](https://nyra-lang.github.io/docs/). |
| 5 | **Makefile** — wire new test gates into the root `Makefile` (`make test-all` dependencies). |
| 6 | **Version** — bump **only** for bug fixes or notable user-facing features (see [Version bump policy](#version-bump-policy)); skip for refactors/tests/docs-only. |
| 7 | **Status** — update [`docs/status.md`](docs/status.md) when feature depth changes. |

Docs-only PRs need step 4 only.

---

## Where to edit (quick reference)

| Change type | Primary locations |
|-------------|-------------------|
| **New keyword / syntax** | `compiler/lexer/` → `compiler/parser/` → `compiler/ast/` → `compiler/expand/` (if sugar) → `compiler/typecheck/` → `compiler/codegen/` → `grammar/nyra.tmLanguage.json` |
| **Comptime behavior** | `compiler/const_eval/` (`comptime.rs`) · `compiler/parser/` · `compiler/typecheck/` |
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

Full pipeline order: [`docs/architecture.md`](docs/architecture.md). Desugaring passes in `compiler/expand/`: see the module index in [`docs/contributor-map.md`](docs/contributor-map.md#compilerexpand-module-index).

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
7. **Docs** — [stdlib.html](https://nyra-lang.github.io/docs/stdlib.html) · [bindings.html](https://nyra-lang.github.io/docs/bindings.html) ([docs repo](https://github.com/nyra-lang/docs)).

Optional friendly names: thin wrappers in `stdlib/builtins_*.ny` (e.g. [`stdlib/builtins_json.ny`](stdlib/builtins_json.ny)).

### C — Compiler intrinsic (rare)

For ops lowered directly to LLVM (`abs_i32`, `min_i32`, …): edit `compiler/types/src/intrinsics.rs` and codegen — see `examples/builtins/math_intrinsics.ny`.

### D — String/array **method builtin** (`.strip_suffix()`, …)

Use the **builtin developer tooling** instead of hand-editing ten compiler files:

```bash
make add-builtin                    # interactive wizard (explains each step)
make remove-builtin ARGS='--method strip_suffix'
make patch-builtin ARGS='-i'
make install-dev                    # required after compiler/stdlib wiring changes
```

The tool wires: C runtime stub, `stdlib/builtins_string.ny`, typecheck, codegen, ownership, examples, and tests. It prints a **monitor report** with your next steps.

**Monitor report legend** (after `make add-builtin` / `patch-builtin`):

| Section | Meaning |
|---------|---------|
| **DONE** | Files the tool changed automatically (compiler, stdlib, tests, examples) |
| **YOUR TASKS** | What you must do manually — usually C implementation in `stdlib/rt/` and fixing test expectations |
| **USAGE** | Copy-paste Nyra snippets; also written under `examples/builtins/…` |
| **NEXT STEPS** | Commands to run (`make install-dev`, `nyra test`, `make gen-abi-header`, …) |

**Wizard tips** (see [`make/py/builtin_dev/README.md`](make/py/builtin_dev/README.md)):

- **Receiver** (`string`, `array`, …) controls which compiler files get wired.
- **Method name** → Nyra API `.foo()` and C symbol `str_foo` (catalog suggests behavior).
- **Arguments** are parameters of `.method(arg)` — **not** the return type. Typing bare `string` alone is a hint, not an argument.
- **Return type** is separate (e.g. `string`, `bool`, `void`).
- Use **`make patch-builtin`** when wiring is wrong but the C body is already written — it re-wires and preserves C when the method name stays the same.

```bash
make patch-builtin ARGS='-i'    # interactive fix for existing method
make patch-builtin ARGS='--method strip_suffix --config make/py/builtin_dev/examples/strip_suffix.json'
```

**Array / bytes receivers:** same workflow — pick `array` or `bytes` in the wizard. Wiring targets differ (`builtins_array.ny`, `rt_vec.c`, …). If the method is not in `method_catalog.py`, the wizard still works; add catalog entry later for better defaults.

**JSON spec fields** (`--config path/to/spec.json`):

| Field | Required | Meaning |
|-------|----------|---------|
| `receiver` | yes | `string`, `array`, `bytes`, or `free` |
| `method` | yes | Nyra method name (e.g. `strip_suffix`) |
| `args` | no | `["name:type", …]` — types: `string`, `i32`, `i64`, `f64`, `bool`, `vec_str`, `bytes`, `array` |
| `returns` | no | Return Nyra type (default `string`) |
| `c_name` | no | C symbol (default `str_<method>` or `vec_<method>`) |
| `rt_module` | no | C file (default `rt_strings.c`, `rt_vec.c`, …) |
| `borrows_receiver` | no | Borrow receiver in typecheck (default `true`) |
| `free_fn_alias` | no | Also wire top-level `fn` alias (default `true`) |
| `stable_abi` | no | Add to `docs/abi-manifest.toml` (default `false`) |
| `abi_since` | no | Version string when `stable_abi` is true |

Example: [`make/py/builtin_dev/examples/strip_suffix.json`](make/py/builtin_dev/examples/strip_suffix.json).

### E — **Contributor hub** (`make contribute`)

Short pointer — patterns A/B/D above. **Full walkthrough:** [Contributor hub guide](#contributor-hub-guide-make-contribute) (menu, every question, example answers, full simulation).

```bash
make contribute                         # interactive menu (add)
make contribute-list                    # show [contrib-dev:…] markers
make contribute-remove ARGS='-i'        # remove scaffold
make contribute-patch ARGS='--marker … --config …'
make test-contrib-py                    # CI smoke for Python tooling
```

Full details: [`make/py/contrib_dev/README.md`](make/py/contrib_dev/README.md).

After stable ABI: `make gen-abi-header` · `make gen-bindings-doc`.

---

## Contributor hub guide (`make contribute`)

The contributor hub is a **step-by-step monitor** for new scaffolds. Every interactive question shows:

| Label | Meaning |
|-------|---------|
| **WHY** | Why we ask this question |
| **TOOL** | What `make contribute` writes automatically (wiring + stubs) |
| **YOU** | What you implement afterward (logic, tests, manual CLI wire) |

After the wizard you get a **PREVIEW** (files + confirm). After apply you get a **MONITOR** (TOOL DID / YOU DO / VERIFY / UNDO).

### Commands

| Command | Purpose |
|---------|---------|
| `make contribute` | Add scaffold — interactive menu (default) |
| `make contribute-list` | List all `[contrib-dev:…]` markers in the repo |
| `make contribute-remove ARGS='-i'` | Remove a scaffold by marker |
| `make contribute-patch ARGS='--marker … --config …'` | Remove + re-add with updated JSON spec |
| `make test-contrib-py` | Verify Python tooling (runs in `make test-preflight`) |

Non-interactive add:

```bash
make contribute ARGS='add --recipe test-example --config make/py/contrib_dev/examples/test_example.json'
```

JSON examples: [`make/py/contrib_dev/examples/`](make/py/contrib_dev/examples/).

### Monitor legend (after every add/remove/patch)

| Section | Meaning |
|---------|---------|
| **TOOL DID** | Files created/updated automatically — do not hand-wire the same paths |
| **YOU DO** | Your tasks — open these files, replace `TODO`, fix assertions |
| **WHERE you edit** | Paths to open in the editor |
| **VERIFY** | Commands to run before opening a PR |
| **WHY this split** | Tool handles repetitive wiring; you own semantics |
| **UNDO** | `make contribute-remove ARGS='--marker …'` |

---

### Main menu (what you see after `make contribute`)

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

**How to choose:** type `1`–`8` at `Select recipe [1-8]:`.

| # | Pick when… |
|---|------------|
| 1 | Nyra-only stdlib helper (wrapper over existing `extern fn`) |
| 2 | New C-backed stdlib API (`extern fn` + `stdlib/rt/`) |
| 3 | String/array method `.foo()` (compiler + C — many files) |
| 4 | Any feature needs tests + runnable demo |
| 5 | Community package (NyraPkg) under `examples/packages/` |
| 6 | New `nyra` subcommand or `--flag` |
| 7 | Stable language contract test |
| 8 | New keyword/syntax (checklist only) |

---

### Option 1 — Stdlib Pure (Pattern A)

**When:** Nyra `fn` in stdlib without new C.

| Step | Question | Example answer | WHY (short) |
|------|----------|----------------|-------------|
| 1 | Stdlib module path | `json/mod.ny` | Which file gets the new `fn` |
| 2 | Function name | `decode_user_id` | Public API name |
| 3 | Arguments | `json:string, key:string` | Parameter list |
| 4 | Return type | `i32` | Return type |
| 5 | Wrap existing extern fn | `json_get_i32` or empty | Auto-wrapper vs custom body |

**TOOL creates:** `stdlib/<module>.ny`, `tests/nyra/<fn>_test.ny` (+ `.typed.ny`), `examples/<topic>/<fn>.ny` (+ `.typed.ny`).

**YOU edit:** fn body in `stdlib/`, assertions in tests.

**Verify:** `nyra test tests/nyra/<fn>_test.ny`

---

### Option 2 — Stdlib Extern + C (Pattern B)

**When:** New C runtime symbol in core stdlib.

| Step | Question | Example answer | WHY (short) |
|------|----------|----------------|-------------|
| 1 | Stdlib module path | `json/mod.ny` | `extern fn` declaration file |
| 2 | Function name | `json_get_f64` | C symbol + Nyra name |
| 3 | Arguments | `json:string, key:string` | C parameters |
| 4 | Return type | `f64` | C return |
| 5 | C runtime file | `rt_json.c` | Where C stub lives |
| 6 | Stable ABI? | `n` | Public FFI manifest |

**TOOL creates:** `stdlib/<module>.ny`, `stdlib/rt/<rt>.c`, `runtime_map.rs`, tests, examples; optionally `abi-manifest.toml`.

**YOU edit:** C implementation in `stdlib/rt/`, test expectations.

**Verify:** `make install-dev && nyra test tests/nyra/<fn>_test.ny`

---

### Option 3 — Built-in Method

**When:** `.strip_suffix()`-style method on `string` / `array` / `bytes`.

Delegates to `make add-builtin` (same WHY/TOOL/YOU style). See [Pattern D](#d--stringarray-method-builtin-method-) above.

---

### Option 4 — Test + Example Pair

**When:** Every user-visible change needs coverage.

| Step | Question | Example answer | WHY (short) |
|------|----------|----------------|-------------|
| 1 | Feature name | `borrow_ref_deref` | Base file name |
| 2 | Example topic | `syntax` | Folder under `examples/` |
| 3 | Optional import | `stdlib/testing.ny` or empty | Pre-import in generated files |

**TOOL creates:** four files — `tests/nyra/<name>_test.ny`, `.typed.ny`, `examples/<topic>/<name>.ny`, `.typed.ny`.

**YOU edit:** replace `assert_eq(1, 1)` and `TODO` demo text.

**Verify:** `nyra test tests/nyra/<name>_test.ny && nyra run examples/<topic>/<name>.ny`

---

### Option 5 — NyraPkg Package

**When:** Package for community (not core stdlib). Scaffold goes to **`nyra/examples/packages/`** (not the separate `nyrapkg` tool repo).

| Step | Question | Example answer | WHY (short) |
|------|----------|----------------|-------------|
| 1 | Package name | `ny-redis` | Folder + import path |
| 2 | Version | `0.1.0` | Semver in `nyra.mod` |
| 3 | Native link library | `sqlite3` or empty | C shim + `link` line |

**TOOL creates:** `examples/packages/<name>/` with `nyra.mod`, `<module>.ny`, `main.ny`, `README.md`, optional `rt/*.c`.

**YOU edit:** API + C shims; install via `nyrapkg` / `NYRA_HOME` (see [NyraPkg workflow](#nyrapkg-workflow)).

**Verify:** `cd examples/packages/<name> && nyra run main.ny`

---

### Option 6 — CLI Command / Flag

**When:** New `nyra` subcommand or global flag.

| Step | Question | Example answer | WHY (short) |
|------|----------|----------------|-------------|
| 1 | Kind | `1` = subcommand, `2` = flag | clap snippet shape |
| 2 | Name | `fmt_check` | Module + CLI id |
| 3 | Description | `Deep format validation` | `--help` text |

**TOOL creates:** `docs/contrib_scaffold/cli_<name>/` (not `cli/src/` — avoids breaking build).

**YOU manually:** copy `args_snippet.rs` → `cli/src/app/args.rs`; move `command.rs` → `cli/src/commands/<name>.rs`; wire `mod.rs` + `session.rs`.

**Verify:** `cargo test -p cli && make smoke-cli`

---

### Option 7 — Conformance Test

**When:** Stable language contract (must pass or must fail compile).

| Step | Question | Example answer | WHY (short) |
|------|----------|----------------|-------------|
| 1 | Mode | `1` = pass, `2` = fail | `nyra test` vs `nyra check` |
| 2 | Area | `strings` | Subfolder under pass/ or fail/ |
| 3 | Test name | `string_concat` | File name |
| 4 | Description | `Concat preserves both operands` | Contract comment |

**TOOL creates:** `tests/conformance/pass/<area>/<name>.ny` or `fail/…`.

**YOU edit:** real assertions or failing code.

**Verify:** `make test-conformance`

---

### Option 8 — Syntax / Keyword Scaffold

**When:** New keyword — **checklist only** (no auto lexer/parser edits).

| Step | Question | Example answer | WHY (short) |
|------|----------|----------------|-------------|
| 1 | Keyword | `await` | Token name |
| 2 | Feature slug | `async_select` | File/dir names |
| 3 | Description | `Select among async branches` | Semantics |
| 4 | Needs expand? | `y` | Desugar pass in `expand/` |
| 5 | Needs comptime? | `n` | `const_eval/` |

**TOOL creates:** `docs/contrib_scaffold/syntax_<slug>/CHECKLIST.md`, grammar hint, tests, examples.

**YOU edit:** full compiler pipeline per checklist.

**Verify:** `cargo test -p compiler && nyra test tests/nyra/<slug>_syntax_test.ny`

---

### Full simulation — Option 4 (Test + Example)

What a new contributor sees from start to finish.

> **Important:** No files are created until you confirm **“Apply scaffold now? (Y/n)”** with `Y`.
> If you press **Ctrl+C** at any point before that (or answer `n`), nothing is written — commands like
> `nyra test tests/nyra/greet_user_test.ny` will fail with “not found” because the scaffold was never applied.

**1. Start**

```bash
cd nyra
make contribute
```

**2. Menu** — type `4` and Enter.

**3. Wizard** (each step shows WHY / TOOL / YOU):

```
── Step 1/3 ──
Q: Feature name (snake_case)
   WHY  → Base name for test and example files.
   TOOL → Creates <name>_test.ny and examples/<topic>/<name>.ny.
   YOU  → Write assertions and demo main().
   e.g. borrow_ref_deref

→ Feature name (snake_case) [my_feature]: greet_user

── Step 2/3 ──
Q: Example topic folder under examples/
   ...
→ Example topic folder under examples/ [syntax]: syntax

── Step 3/3 ──
Q: Optional stdlib import path
   ...
→ Optional stdlib import path []: 
```

**4. Preview + confirm**

```
  PREVIEW — confirm before writing files
    name: greet_user
    topic: syntax
    import: (none)

  TOOL will create/edit:
    • tests/nyra/greet_user_test.ny (+ .typed.ny)
    • examples/syntax/greet_user.ny (+ .typed.ny)

→ Apply scaffold now? (Y/n): y
```

**5. Monitor (after apply)**

```
✅ TOOL DID (automatic):
   • tests/nyra/greet_user_test.ny — created
   • tests/nyra/greet_user_test.typed.ny — created
   • examples/syntax/greet_user.ny — created
   • examples/syntax/greet_user.typed.ny — created

📋 YOU DO:
   1. Implement tests in tests/nyra/greet_user_test.ny
   2. Implement demo in examples/syntax/greet_user.ny
   3. Run: nyra test tests/nyra/greet_user_test.ny
   4. Run: nyra run examples/syntax/greet_user.ny

▶ VERIFY:
   1. nyra test … && nyra run …
   2. make install-dev     # if compiler/stdlib wiring changed
   3. make test-preflight
   4. make test-all

🔄 UNDO: make contribute-remove ARGS='--marker test_example:greet_user'
```

**6. Your work** — open `tests/nyra/greet_user_test.ny`, replace placeholder assertions; open `examples/syntax/greet_user.ny`, write a small demo.

**7. Verify**

```bash
nyra test tests/nyra/greet_user_test.ny
nyra run examples/syntax/greet_user.ny
```

**8. Undo (optional)**

```bash
make contribute-remove ARGS='--marker test_example:greet_user'
```

---

### Full simulation — Option 5 (NyraPkg Package)

**When:** Community package under `examples/packages/` (not the separate `nyrapkg` tool repo).

**1. Start**

```bash
cd nyra
make contribute
```

**2. Menu** — type `5` and Enter.

**3. Wizard**

```
── Step 1/3 ──
Q: Package name
   WHY  → Folder name and pkg import path (ny-foo).
   TOOL → Creates examples/packages/<name>/ layout.
   YOU  → Implement API; publish or use via nyrapkg.
   e.g. ny-redis

→ Package name [ny-example]: ny-redis

── Step 2/3 ──
Q: Version
   ...
→ Version [0.1.0]: 0.1.0

── Step 3/3 ──
Q: Native link library (or empty)
   WHY  → If set, adds link + rt/*.c shim.
   TOOL → Creates rt/<module>.c stub when link_lib set.
   YOU  → Implement C shims; document in README.
   e.g. sqlite3

→ Native link library (or empty) []: sqlite3
```

**4. Preview + confirm**

```
  PREVIEW — confirm before writing files
    name: ny-redis
    version: 0.1.0
    link_lib: sqlite3

  TOOL will create/edit:
    • examples/packages/ny-redis/nyra.mod
    • examples/packages/ny-redis/ny_redis.ny
    • examples/packages/ny-redis/main.ny
    • examples/packages/ny-redis/README.md
    • examples/packages/ny-redis/rt/ny_redis.c

→ Apply scaffold now? (Y/n): y
```

**5. Monitor (after apply)**

```
✅ TOOL DID (automatic):
   • examples/packages/ny-redis/nyra.mod — created
   • examples/packages/ny-redis/ny_redis.ny — created
   • examples/packages/ny-redis/main.ny — created
   • examples/packages/ny-redis/README.md — created
   • examples/packages/ny-redis/rt/ny_redis.c — created

📋 YOU DO:
   1. Implement API in examples/packages/ny-redis/ny_redis.ny
   2. Implement C shims in examples/packages/ny-redis/rt/ny_redis.c
   3. Smoke: cd examples/packages/ny-redis && nyra run main.ny

▶ VERIFY:
   cd examples/packages/ny-redis && nyra run main.ny

🔄 UNDO: make contribute-remove ARGS='--marker pkg:ny-redis'
```

**6. Your work**

- Open `examples/packages/ny-redis/ny_redis.ny` — replace `TODO` extern stubs with real API.
- Open `rt/ny_redis.c` — implement C calls to `sqlite3`.
- Update `main.ny` smoke test and `README.md`.

**7. Verify**

```bash
cd examples/packages/ny-redis
nyra run main.ny
```

**8. Non-interactive shortcut**

```bash
# Edit name/link_lib in JSON first, or copy examples/pkg.json
make contribute ARGS='add --recipe pkg --config make/py/contrib_dev/examples/pkg.json'
```

---

### Full simulation — Option 6 (CLI Command / Flag)

**When:** New `nyra` subcommand or global flag. TOOL writes a **scaffold only** — you wire into `cli/` manually.

**1. Start**

```bash
cd nyra
make contribute
```

**2. Menu** — type `6` and Enter.

**3. Wizard**

```
── Step 1/3 ──
Q: CLI kind (1=subcommand, 2=flag)
   WHY  → Subcommand = nyra foo; flag = nyra build --foo.
   TOOL → Generates matching args_snippet.rs template.
   YOU  → Copy snippet into cli/src/app/args.rs manually.

→ CLI kind (1=subcommand, 2=flag)
    1. Subcommand (nyra my_cmd …)
    2. Global flag (nyra build --my_flag)
  Choice: 1

── Step 2/3 ──
Q: Name (snake_case)
   ...
→ Name (snake_case) [my_cmd]: fmt_check

── Step 3/3 ──
Q: Short description
   ...
→ Short description [TODO: describe this command]: Deep format validation for projects
```

**4. Preview + confirm**

```
  PREVIEW — confirm before writing files
    kind: subcommand
    name: fmt_check
    description: Deep format validation for projects

  TOOL will create/edit:
    • docs/contrib_scaffold/cli_fmt_check/command.rs
    • docs/contrib_scaffold/cli_fmt_check/args_snippet.rs
    • docs/contrib_scaffold/cli_fmt_check/README.md

  YOU will implement:
    • cli/src/app/args.rs       — paste args_snippet
    • cli/src/commands/fmt_check.rs — implement run()
    • cli/src/commands/mod.rs   — pub mod
    • cli/src/app/session.rs    — dispatch match arm

→ Apply scaffold now? (Y/n): y
```

**5. Monitor (after apply)**

```
✅ TOOL DID (automatic):
   • docs/contrib_scaffold/cli_fmt_check/command.rs — created
   • docs/contrib_scaffold/cli_fmt_check/args_snippet.rs — created
   • docs/contrib_scaffold/cli_fmt_check/README.md — created

📋 YOU DO:
   1. Read docs/contrib_scaffold/cli_fmt_check/README.md
   2. Copy args_snippet.rs into cli/src/app/args.rs
   3. Move command.rs → cli/src/commands/fmt_check.rs and implement
   4. Wire mod + dispatch in cli/src/commands/mod.rs and cli/src/app/session.rs

⚠ NOTES:
   • CLI wiring is manual — scaffold avoids breaking the build.

▶ VERIFY:
   cargo test -p cli && make smoke-cli

🔄 UNDO: make contribute-remove ARGS='--marker cli:fmt_check'
```

**6. Your work (manual wiring)**

```bash
# 1. Read the scaffold README
cat docs/contrib_scaffold/cli_fmt_check/README.md

# 2. Paste clap snippet from args_snippet.rs into cli/src/app/args.rs
# 3. Move and implement:
#    docs/contrib_scaffold/cli_fmt_check/command.rs
#      → cli/src/commands/fmt_check.rs
# 4. Add to cli/src/commands/mod.rs:
#      pub mod fmt_check;
# 5. Add dispatch arm in cli/src/app/session.rs
```

**7. Verify**

```bash
cargo test -p cli
make smoke-cli
nyra fmt_check --help    # after wiring
```

**8. Non-interactive shortcut**

```bash
make contribute ARGS='add --recipe cli --config make/py/contrib_dev/examples/cli.json'
```

---

### Full simulation — Option 2 (Stdlib Extern) — abbreviated

```bash
make contribute
# choose 2
# answer: json/mod.ny, json_get_f64, json:string,key:string, f64, rt_json.c, stable_abi=n
# confirm Y
# TOOL wires: stdlib/json/mod.ny, stdlib/rt/rt_json.c, runtime_map.rs, tests, examples
# YOU: implement C in rt_json.c, fix tests, make install-dev, nyra test …
```

---

### Related resources

| Resource | Purpose |
|----------|---------|
| [`make/py/contrib_dev/README.md`](make/py/contrib_dev/README.md) | Hub file map + subcommands |
| [`make/py/contrib_dev/examples/`](make/py/contrib_dev/examples/) | JSON specs for non-interactive runs |
| [`make/py/builtin_dev/README.md`](make/py/builtin_dev/README.md) | Option 3 (builtin methods) |
| [`docs/contributor-map.md`](docs/contributor-map.md) | Where to edit in the compiler/stdlib |

---

### Subcommands — list, remove, patch

#### `make contribute-list`

Shows every scaffold tagged with `[contrib-dev:…]`:

```
══════════════════════════════════════════════════════════════
  CONTRIBUTE — WIRED SCAFFOLDS
══════════════════════════════════════════════════════════════
  • test_example:greet_user
      tests/nyra/greet_user_test.ny
      examples/syntax/greet_user.ny
  …

  Remove: make contribute-remove ARGS='-i'
  Patch:  make contribute-patch ARGS='--marker <m> --config …'
```

**WHY:** See what the tool wired before you edit or open a PR.

#### `make contribute-remove ARGS='-i'`

Interactive remove — same monitor style:

```
── Remove scaffold ──
  WHY  → Undo a scaffold wired by make contribute (marked [contrib-dev:…]).
  TOOL → Removes markers, deletes scaffold files, cleans runtime_map if needed.
  YOU  → Search for leftover references; run make test-preflight.

  1. test_example:greet_user  (greet_user_test.ny, greet_user.ny)

→ Select number or paste marker: 1
```

**TOOL:** Deletes marked files and unwires `runtime_map.rs` / ABI entries when applicable.

**YOU:** Grep for leftover imports; run `make test-preflight`.

#### `make contribute-patch ARGS='--marker … --config …'`

Remove + re-add with an updated JSON spec (rename args, add stable ABI, etc.):

```bash
make contribute-patch ARGS='--marker test_example:my_feature --config make/py/contrib_dev/examples/test_example.json'
```

**WHY:** Safer than hand-editing multiple wired files — tool re-applies the recipe from JSON.

**Example JSON** (`make/py/contrib_dev/examples/test_example.json`):

```json
{
  "name": "borrow_ref_deref",
  "example_topic": "syntax",
  "import_path": "stdlib/testing.ny"
}
```

---

### Troubleshooting — “not found” after `make contribute`

| Symptom | Cause | Fix |
|---------|-------|-----|
| `not found: tests/nyra/greet_user_test.ny` | Wizard cancelled (Ctrl+C) or answered `n` at confirm | Re-run `make contribute`, finish all steps, confirm **Y** |
| Same, but you saw the menu only | Interrupted at `Select recipe [1-8]:` | No recipe chosen — start again |
| Files exist but `nyra test` fails | Scaffold applied but TODO placeholders remain | Edit test file — replace `assert_eq(1, 1)` |
| Unsure if scaffold exists | | `make contribute-list` |

**Quick create without interactive wizard** (Option 4, `greet_user`):

```bash
printf '%s\n' '{"name":"greet_user","example_topic":"syntax"}' > /tmp/greet_user.json
make contribute ARGS='add --recipe test-example --config /tmp/greet_user.json'
```

Or copy and edit [`make/py/contrib_dev/examples/test_example.json`](make/py/contrib_dev/examples/test_example.json).

---

### Case study: `strip_suffix` end-to-end

Worked example for Pattern D (string method builtin):

```bash
# 1. Wire compiler + stdlib stubs (wizard explains each step)
make add-builtin
# or: make add-builtin ARGS='--config make/py/builtin_dev/examples/strip_suffix.json'

# 2. Implement C logic — search [builtin-dev:strip_suffix:string] in:
#    stdlib/rt/rt_strings.c

# 3. Fix test expectations
#    tests/nyra/string_strip_suffix_test.ny

# 4. Install fresh CLI (mandatory after compiler wiring)
make install-dev

# 5. Verify
nyra test tests/nyra/string_strip_suffix_test.ny
nyra run examples/builtins/strings/strip_suffix.ny

# 6. If stable ABI was enabled in wizard:
make gen-abi-header && make gen-bindings-doc
```

Files touched automatically: `compiler/typecheck/`, `compiler/codegen/`, `stdlib/builtins_string.ny`, `stdlib/strings.ny`, `examples/builtins/strings/strip_suffix.ny`, tests. Monitor output lists each path.

### Stdlib design rules

- **Small files** — micro-modules under `stdlib/`; avoid monolithic files.
- **Types optional** — APIs must work with inference (strings, numbers, arrays, objects, booleans).
- **Static dispatch** — prefer monomorph/generics over dynamic dispatch for LLVM inlining.
- **NyraPkg** — community packages live in `examples/packages/`; proven modules may graduate into stdlib.

### C runtime & prebuilt artifacts

After changing `stdlib/rt/*.c`, rebuild and reinstall:

```bash
make install-dev    # rebuilds prebuilt runtime via make/lib/build-prebuilt-rt.sh
```

Prebuilt static libs live under `stdlib/prebuilt/<triple>/`. The stamp `stdlib/prebuilt/.../rt-sources.stamp` tracks which C sources were linked. Contributors rarely edit prebuilt paths by hand — `make install-dev` refreshes them.

---

## NyraPkg workflow

Use **NyraPkg** for optional/community modules with native deps or semver boundaries. Core batteries-included APIs belong in `stdlib/` (see [Design philosophy](#start-here--documentation-map)).

**Layout** (see [`examples/packages/ny-sqlite/`](examples/packages/ny-sqlite/)):

```
my-pkg/
  nyra.mod          # name, version, link / link-source
  nyra.lock         # pinned deps (generated)
  *.ny              # module API (extern fn + wrappers)
  rt/*.c            # optional C shim (auto-linked on nyra build)
```

**Typical flow:**

```bash
nyra pkg init
# edit nyra.mod — add link sqlite3, link-source rt/sqlite.c
nyra pkg verify
nyra build
nyra run main.ny
```

**In another project:**

```bash
nyra pkg install ny-sqlite@^0.1.0   # copies to .nyra/cache/, updates nyra.lock
import "pkg/ny-sqlite/sqlite.ny"
```

| When | Use |
|------|-----|
| New stdlib-quality API, no heavy deps | `stdlib/**/*.ny` (Pattern A/B) |
| Heavy native lib, experimental driver | NyraPkg under `examples/packages/` |
| Proven package, wide use | Graduate into `stdlib/` |

Compiler/package code: `pkg/` · CLI: `cli/src/commands/pkg.rs`.

---

## Removing a feature

Walk the **add path in reverse**, then clean up tests and docs:

| Step | Action |
|------|--------|
| 1 | **String/array method** → `make remove-builtin` (interactive or `ARGS='--method foo'`) |
| 2 | **Compiler syntax** → revert lexer → parser → ast → expand → typecheck → codegen → ownership → borrowck → const_eval |
| 3 | **Stdlib `extern fn`** → remove Nyra decl, C impl, `runtime_map.rs` entry, ABI manifest entry |
| 4 | **Tests** → delete `tests/nyra/*`, conformance entries, suite files that only exist for the feature |
| 5 | **Examples / Apps** → delete or rewrite demos |
| 6 | **Grammar** → remove keyword from `grammar/nyra.tmLanguage.json` |
| 7 | **Docs** → docs repo, `webDocs/`, `docs/status.md`, `CHANGELOG.md` if user-visible |
| 8 | **Regenerate** → `make gen-abi-header` · `make gen-bindings-doc` · `make gen-suite-tests` if suite grid changed |
| 9 | **Verify** → `make install-dev` · `make test-all` (include `tests/suite/fail/`) |

**Do not** leave orphaned `[builtin-dev:…]` markers or half-removed match arms in Rust — `make remove-builtin` handles most string-method cleanup.

---

## Ways to contribute

| Area | You can… |
|------|-----------|
| **Language** | Fix bugs or add features across the compiler pipeline; add tests in `tests/nyra/` — see [`docs/contributor-map.md`](docs/contributor-map.md) |
| **Stdlib** | Add modules under `stdlib/` + `stdlib/rt/` — see [How to add a stdlib function](#how-to-add-a-stdlib-function) |
| **Examples** | Add or improve `.ny` samples under `examples/` (zero-types + typed pairs) — **small demos and builtins** |
| **Apps** | Extend multi-file projects under [`Apps/`](Apps/) — **full reference applications** (games, IDE, databases) |
| **Tooling** | CLI (`cli/`), formatter, `nyra diag`, `nyra lsp`, NyraPkg (`pkg/`) |
| **Docs** | `docs/` (this repo), [docs repo](https://github.com/nyra-lang/docs) (web site source), `grammar/README.md` |
| **Grammar** | Update [`grammar/nyra.tmLanguage.json`](grammar/nyra.tmLanguage.json) when keywords change |
| **Benchmarks** | Fair cross-language benches in `examples/comparison/` |
| **Runtime** | C runtime `stdlib/rt/`, headers `stdlib/nyra_rt.h`, Rust helpers `rt/` |
| **Make / generators** | `make/py/` Python scripts, `make/*.mk` targets — see [`docs/make-and-generators.md`](docs/make-and-generators.md) · **`make contribute`** hub |

---

## Makefile & Python generators (`make/`)

Nyra uses **Make** for test orchestration and **Python** (`make/py/`) for code/doc generators. Contributors should invoke generators via **`make <target>`**, not raw `python3 make/py/…` (unless debugging the script).

**Canonical reference:** [`docs/make-and-generators.md`](docs/make-and-generators.md)

### Quick map

| Area | Location | Common commands |
|------|----------|-----------------|
| Make modules | `make/*.mk` | `make help` · `make test-all` |
| Generators | `make/py/*.py` | `make gen-abi-header` · `make gen-bindings-doc` |
| Builtin tooling | `make/py/builtin_dev/` | `make add-builtin` · `make patch-builtin` |
| Contributor hub | `make/py/contrib_dev/` | `make contribute` · `make contribute-list` · `make contribute-remove` |
| Shell helpers | `make/lib/*.sh` | Used internally by test-all, install, bench |

### Generators contributors use most

| Make target | Python script | When |
|-------------|---------------|------|
| `make add-builtin` | `builtin-dev.py` | New `.method()` on string/array + C runtime |
| `make gen-abi-header` | `gen-abi-header.py` | After `docs/abi-manifest.toml` changes |
| `make gen-bindings-doc` | `gen-bindings-doc.py` | Refresh bindings docs/HTML |
| `make gen-suite-tests` | `gen-suite-tests.py` | Regenerate compiletest grid |
| `make sync-webdocs-code-tabs` | `sync-webdocs-code-tabs.py` | Sync easy/typed doc code tabs |
| `make install-dev` | `make/lib/updateLang.sh` | Install fresh `nyra` CLI to PATH |

See the full script catalog (benchmarks, comparison sync, snippet-types, …) in [`docs/make-and-generators.md`](docs/make-and-generators.md).

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
├── Apps/              # Reference multi-file applications (games, IDE, databases, …)
├── examples/          # Small demos, builtins, comparison benchmarks
├── docs/              # Architecture, contributor map, status, ABI, testing runbook
│                      #   contributor-map.md — what to change → where to go
│                      # Web docs also in webDocs/ (this repo) + github.com/nyra-lang/docs
├── skills/            # Language reference (skill.md) & contributor guidelines
├── agents/            # Agent/release workflow (skill.md)
├── Makefile           # Primary entry (make test-all, make help, …)
├── make/              # Modular Make targets, lib recipes, py generators
│                      #   See docs/make-and-generators.md for make/py/ catalog
├── scripts/           # install.sh (curl), install.ps1 only
└── benchmarks/        # CI perf baselines
```

**Compiler pipeline:** `resolve/` runs at **load time** (imports, prelude). **Compile time** (`compiler/driver`): lexer → parser → expand → monomorph → typecheck → ownership → borrowck → const_eval → codegen. Details: [`docs/architecture.md`](docs/architecture.md).

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

Expected output starts with `Dungeon Steps`, then score lines and `3`. See [`examples/comparison/dungeon/README.md`](examples/comparison/dungeon/README.md) and [dungeon-steps.html](https://nyra-lang.github.io/docs/dungeon-steps.html).

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
5. Mention in the PR; optionally add to [examples.html](https://nyra-lang.github.io/docs/examples.html) in the [docs repo](https://github.com/nyra-lang/docs).

---

## Testing

### One command — full suite

```bash
make test-all
```

Logs to `target/test-all.txt`. Optional: `TEST_PERF=1` for perf gate. See [`docs/testing-runbook.md`](docs/testing-runbook.md) for CI stages, snapshot updates, and rollback policy.

### Where to put tests

| Test kind | Location | When to use |
|-----------|----------|-------------|
| **Feature test (default)** | `tests/nyra/<name>_test.ny` (+ `.typed.ny`) | Every user-visible language/stdlib change — **start here** |
| **Small repro / fixture** | `tests/nyra/<name>.ny` | Paired with a `*_test.ny` runner |
| **Rust unit tests** | Same Rust module (`#[cfg(test)]`) | Internal helper logic |
| **Driver integration** | `compiler/driver/tests/` | Pipeline, snapshots, ABI manifest |
| **CONF-LANG contract** | `tests/conformance/pass/` or `fail/` | Stable language contract tests |
| **Compiletest grid** | `tests/suite/` | Large pass/fail/run corpus (usually generated) |
| **Runnable demo** | `examples/<topic>/` | User-facing samples (`foo.ny` + `foo.typed.ny`) |

**Rule of thumb:** new language features → `tests/nyra/` first. Add conformance or suite entries only when you need a stable contract or grid coverage.

### Test decision tree

```
User-visible language/stdlib change?
├─ YES → tests/nyra/<feature>_test.ny (+ .typed.ny if types matter)
│         + examples/<topic>/foo.ny (+ foo.typed.ny)
│         + nyra test … && make test-nyra-lang
│
├─ Need stable language contract (CONF-LANG)?
│    → tests/conformance/pass/ or fail/
│
├─ Need combinatorial compile grid?
│    → tests/suite/ (often via make gen-suite-tests)
│
├─ Internal Rust helper only?
│    → #[cfg(test)] in same module OR compiler/driver/tests/
│
├─ LLVM IR / diagnostic output regression?
│    → compiler/driver/tests/snapshots/
│       INSTA_UPDATE=1 cargo test -p compiler --test codegen_snapshots
│       (review diff carefully before commit)
│
└─ ABI / FFI symbol added?
     → compiler/driver/tests/abi_manifest.rs + make test-abi-roundtrip
```

**Nyra test file conventions:**

| Pattern | Meaning |
|---------|---------|
| `test fn foo()` inside `*_test.ny` | Native test runner (`nyra test`) |
| File named `something_test.ny` | Discovered as test root |
| `// run-stdout: …` in `tests/suite/` | Compiletest expected stdout |
| `//~ ERROR …` in suite files | Expected diagnostic at line |

**Compiletest directives** (in `tests/suite/`): see `compiler/compiletest/src/directives.rs` — common ones are `// ignore-test`, `// tier:`, `// run-stdout:`.

**Fail suite gate:** after changing **typecheck**, **comptime**, **parser**, or **diagnostic text**, run compiletest fail corpus — `make test-compiletest` or full `make test-all`. Do not merge if unrelated `tests/suite/fail/` cases start passing (broken test) or failing (regression).

**Example corpus:** `tests/corpus/manifest.toml` lists every `examples/` entry CI compiles. If an example breaks, fix it or set `expect_compile = false` with a comment (see [`docs/testing-runbook.md`](docs/testing-runbook.md)).

**Conformance:** map failing `CONF-*` tests to specs in `docs/conformance/*.md` — fix regression or update spec + test together.

**Fuzz & sanitizer (optional gates):**

```bash
make test-fuzz-smoke              # short fuzz smoke (part of test-all on Linux CI weekly)
TEST_FUZZ=1 make test-all         # extended fuzz gates locally
TEST_SAN=1 make test-all          # ASan/UBSan-style gates (slow; Linux/macOS)
```

Fuzz targets live in `fuzz/`. If fuzz finds a compiler panic, add a minimal repro to `tests/nyra/` or `tests/suite/fail/`.

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

**Compiler pipeline crates** (inside `compiler/` — see [`docs/architecture.md`](docs/architecture.md#crate-map-contributor-quick-reference)):

| Crate | You change this when… |
|-------|---------------------|
| `lexer` | Tokens, keywords, literals |
| `parser` / `ast` | Grammar, syntax nodes |
| `expand` | Desugaring (`??`, `?`, async, Vec, …) |
| `resolve` | Imports, prelude, project graph |
| `monomorph` | Generics → monomorphic AST |
| `typecheck` / `types` | Type rules, builtins, inference |
| `ownership` / `borrowck` | Moves, borrows, drop plan |
| `const_eval` | Comptime evaluation |
| `codegen` | LLVM IR, `runtime_map.rs` |
| `compiletest` | Suite runner, directives |
| `errors` | Diagnostic codes, `nyra explain` text |
| `driver` | Orchestration, cache, integration tests |

```bash
cargo build --workspace
cargo test --workspace
cargo test -p typecheck          # single pipeline stage
cargo test -p compiler           # driver + integration
```

---

## Debugging the compiler

| Goal | Command |
|------|---------|
| Single Rust test | `cargo test -p typecheck test_name -- --nocapture` |
| Full compiler integration | `cargo test -p compiler` |
| One Nyra file (no link) | `nyra check path/to/file.ny` |
| JSON diagnostics | `nyra diag path/to/file.ny --json` |
| Explain error code | `nyra explain E018` |
| Fresh CLI (avoid stale PATH) | `./target/debug/nyra test …` or `make install-dev` |
| Clear incremental cache | `rm -rf path/to/project/target .nyra-cache` |

**Snapshot tests** (review every hunk before commit):

```bash
# IR output
INSTA_UPDATE=1 cargo test -p compiler --test codegen_snapshots

# Diagnostic text
INSTA_UPDATE=1 cargo test -p compiler --test diagnostics_snapshots
```

**Compiletest one file:**

```bash
cargo test -p compiler suite_pass_my_test -- --nocapture
```

**Verbose build:** `nyra build --verbose path/` shows linker/toolchain details. Escape-analysis lines appear when ownership verbose mode is enabled in driver tests.

When adding a diagnostic, update `compiler/errors/src/explain.rs` and matching `//~ ERROR` lines in `tests/suite/`.

---

## Development setup

1. Install [Rust](https://rustup.rs/) (stable).
2. Install **clang** and **libclang** (required for `nyra build`, `nyra bind c`, `nyra-c-bindgen`).
3. Clone and build:

```bash
git clone git@github.com:nyra-lang/nyra.git
cd nyra
cargo build --workspace
```

4. Install `nyra` on your PATH:

```bash
./scripts/updateLang.sh   # or: make install-dev
nyra --version
```

### Platform-specific dependencies

| Platform | Install |
|----------|---------|
| **macOS** | Xcode Command Line Tools (`xcode-select --install`); optional Homebrew `llvm` for `NYRA_LLVM_BIN` |
| **Linux (Debian/Ubuntu)** | `sudo apt-get install -y clang lld libclang-dev llvm-dev libsqlite3-dev zlib1g-dev libssl-dev` |
| **Windows** | [GitHub Release](https://github.com/nyra-lang/nyra/releases) + [`scripts/install.ps1`](scripts/install.ps1); or build from source with Rust + clang |
| **WSL** | Same as Linux; use Linux prebuilt or local `make install-dev` |

CI installs the same Linux packages — see [`.github/workflows/ci.yml`](.github/workflows/ci.yml).

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
| `nyra build --for windows\|linux\|macos` | Cross-compile (see [targets.html](https://nyra-lang.github.io/docs/targets.html)) |
| `nyra build --target wasm32-wasi` | Wasm subset (`stdlib/nyra_rt_wasi.c`) |
| `nyra check --deny-extended` | Core-only CI (reject Extended tier features) |
| `nyra pkg init` / `verify` / `build` | NyraPkg workflow |
| `nyra lsp` | Language server (stdio) |

Details: [tooling.html](https://nyra-lang.github.io/docs/tooling.html) · [Installation](https://nyra-lang.github.io/docs/install.html).

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
5. **Docs** — `docs/` (this repo) and the [docs repo](https://github.com/nyra-lang/docs); feature depth → [`docs/status.md`](docs/status.md).
6. **Style** — `cargo fmt` on touched Rust files.
7. **CI** — see [`.github/workflows/ci.yml`](.github/workflows/ci.yml) and [`docs/testing-runbook.md`](docs/testing-runbook.md).

**Parser / ABI policy:**

- Do not change parser behavior for Core-tier syntax without discussion and tests.
- **Breaking** FFI ABI changes require explicit review; **adding** stable symbols follows [`docs/abi-manifest.toml`](docs/abi-manifest.toml) + version bump.
- Extended-tier features (`async`, traits, macros, enum payloads with storage, `defer`, …) may emit `warning[W001]`; see [`docs/stability-v1.md`](docs/stability-v1.md).

---

## Release workflow (version + webDocs)

User-facing docs publish to **[nyra-lang.github.io/docs](https://nyra-lang.github.io/docs/)** from the [docs repo](https://github.com/nyra-lang/docs). In-repo **`webDocs/`** mirrors and skill sources ship with the nyra tag.

For any user-visible language/stdlib/CLI/ABI change **that warrants a version bump** (see [Version bump policy](#version-bump-policy)):

1. Bump **`[workspace.package] version`** in [`Cargo.toml`](Cargo.toml).
2. Add section to [`CHANGELOG.md`](CHANGELOG.md).
3. Update **in-repo** docs when applicable:
   ```bash
   # edit webDocs/*.html if needed, then:
   node webDocs/scripts/build-nyra-skill.mjs    # → skills/skill.md
   node webDocs/scripts/build-search-index.mjs
   make build-webdocs                           # optional full HTML rebuild
   ```
4. Update the **[docs repo](https://github.com/nyra-lang/docs)** — relevant `*.html` and `nyra-skill.md` for the public site.
5. Update [`docs/status.md`](docs/status.md) when feature depth changes.

---

## Backend / stdlib runtime checklist

For async, TCP, HTTP, JSON, TLS, crypto, and other runtime-backed stdlib APIs:

1. **Nyra stub** — `stdlib/<area>/*.ny` with `fn` and/or `extern fn`.
2. **C runtime** — `stdlib/rt/rt_*.c`; register every `extern` symbol in [`compiler/codegen/src/runtime_map.rs`](compiler/codegen/src/runtime_map.rs).
3. **ABI** — [`docs/abi-manifest.toml`](docs/abi-manifest.toml) + `make gen-abi-header` + [`compiler/driver/tests/abi_manifest.rs`](compiler/driver/tests/abi_manifest.rs).
4. **Integration test** — `compiler/driver/tests/integration.rs` or `nyra run` on an example.
5. **Example** — `examples/builtins/` or `examples/projects/`.
6. **Docs** — [stdlib.html](https://nyra-lang.github.io/docs/stdlib.html), [backend.html](https://nyra-lang.github.io/docs/backend.html) if applicable ([docs repo](https://github.com/nyra-lang/docs)).
7. **Reinstall** — `./scripts/updateLang.sh   # or: make install-dev` after pulling runtime changes.

DB drivers that need heavy native deps often start in **NyraPkg** (`examples/packages/ny-sqlite/`) before graduating into stdlib.

---

## Documentation: where to edit what

Nyra docs live in **three places**. Do not duplicate prose — know which to update:

| Location | Contents | When to edit |
|----------|----------|--------------|
| **`docs/`** (this repo) | Architecture, contributor guides, ABI manifest, testing runbook, status | Toolchain/process/ABI changes |
| **`webDocs/`** (this repo) | HTML mirrors, search index inputs, `nyra-skill.md` source | User-facing docs **in-repo**; rebuild with commands below |
| **[github.com/nyra-lang/docs](https://github.com/nyra-lang/docs)** | Published site source ([nyra-lang.github.io/docs](https://nyra-lang.github.io/docs/)) | Public HTML pages, tutorials, stdlib reference pages |
| **`skills/skill.md`** | Language reference (synced from skill build) | Syntax/stdlib semantics for AI + humans |
| **`CHANGELOG.md`** | Release notes | User-visible fixes/features (with version bump) |
| **`grammar/`** | VS Code/Cursor syntax (`nyra.tmLanguage.json`) | New keywords/tokens |

### webDocs: in-repo vs docs repo

| Task | Where | Commands |
|------|-------|----------|
| Edit HTML shipped with nyra repo | `webDocs/*.html` | `make build-webdocs` |
| Rebuild AI skill + search index **in nyra repo** | `webDocs/scripts/` | `node webDocs/scripts/build-nyra-skill.mjs` · `node webDocs/scripts/build-search-index.mjs` |
| Edit **published** site pages | [docs repo](https://github.com/nyra-lang/docs) | Same node scripts there; syncs to GitHub Pages |
| Sync easy/typed code tabs in HTML | `webDocs/` | `make sync-webdocs-code-tabs` |

**Rule:** language/stdlib/CLI changes usually need **both** an example in this repo **and** prose in the [docs repo](https://github.com/nyra-lang/docs) (stdlib.html, tooling.html, …). Patch releases: follow [`agents/skill.md`](agents/skill.md).

**Typical doc PR for a new stdlib function:**

1. Example in `examples/builtins/…`
2. [`docs/bindings.md`](docs/bindings.md) (or `make gen-bindings-doc` if ABI entry added)
3. [docs repo](https://github.com/nyra-lang/docs) stdlib.html section (human prose)
4. Optional: `webDocs/` + skill rebuild if in-repo HTML/skill must ship with the tag
5. Optional: `make sync-webdocs-code-tabs` if HTML has easy/typed code pairs

---

## Version bump policy

**Do not bump version on every PR.** Bump `[workspace.package] version` in [`Cargo.toml`](Cargo.toml) and add a [`CHANGELOG.md`](CHANGELOG.md) entry **only when**:

| Bump? | Situation |
|-------|-----------|
| **Yes (patch `1.x.Y`)** | Real bug fix — correctness, runtime, typecheck, linker, user-visible failure |
| **Yes (minor `1.Y.0`)** | Notable feature, stdlib addition, ABI/toolchain change worth release notes |
| **No** | Refactors, internal cleanup, tests-only, Makefile/CI, docs-only (unless docs ship new public behavior) |

Full release workflow: [`agents/skill.md`](agents/skill.md) · webDocs rebuild steps in [Release workflow](#release-workflow-version--webdocs) below.

---

## Troubleshooting & FAQ

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `E018: method call requires struct receiver` after adding `.method()` | Stale `nyra` on PATH (old compiler) | `make install-dev` · verify `which nyra` |
| `nyra run` works, `nyra test` fails for same feature | Incremental cache from old good build | `rm -rf examples/.../target tests/nyra/target` · `make install-dev` |
| `cargo build -p cli` OK but `nyra test` still fails | Built `./target/debug/nyra` but PATH uses `~/.cargo/bin/nyra` | `make install-dev` or `./target/debug/nyra test …` |
| `make gen-abi-header` TOML parse error | Invalid comment in `abi-manifest.toml` (use `#`, not `//`) | Fix manifest; TOML blocks from `add-builtin` use `# [builtin-dev:…]` |
| Link errors / undefined symbol | Missing `runtime_map.rs` entry or C impl | Register symbol; rebuild runtime |
| Changes not visible in IDE/LSP | LSP uses installed `nyra` | `make install-dev`, restart LSP |
| `make test-all` failed — where is the log? | Gate logs | `target/test-all.txt`, `target/.nyra-test-all-failures` |
| `INSTA_UPDATE` snapshot huge diff | IR/diagnostic format changed | Review every hunk; don't blind-commit |
| `unrecognized subcommand 'examples/…'` | Wrong CLI syntax | Use `nyra run path/to/file.ny` not `nyra path/…` |

**Diagnostic codes:** stable codes like `E004` (cannot infer type), `E018` (unknown method) live in `compiler/errors/`. Add explanations in `compiler/errors/src/explain.rs`. Users run `nyra explain E018`.

**Incremental compile cache:** binaries and metadata under `<project>/target/debug/` and `.nyra-cache/`. Delete when debugging stale behavior.

---

## Glossary

| Term | Meaning |
|------|---------|
| **Zero-types** | Nyra code without explicit type annotations (default style) |
| **Typed / explicit-types** | `.typed.ny` or annotated programs — must work alongside zero-types |
| **Prelude** | Auto-imported stdlib symbols (`compiler/resolve/prelude.rs`) |
| **extern fn** | Nyra declaration calling C in `stdlib/rt/` |
| **runtime_map** | Maps C symbol → source file for linking (`runtime_map.rs`) |
| **ABI manifest** | Stable C symbols (`docs/abi-manifest.toml`) → `nyra_rt.h` |
| **builtin-dev** | Python tooling (`make add-builtin`) wiring string methods |
| **compiletest** | Large generated pass/fail grid under `tests/suite/` |
| **CONF-LANG** | Conformance contract tests in `tests/conformance/` |
| **Core / Extended tier** | Stability classification — [`docs/stability-v1.md`](docs/stability-v1.md) |
| **NyraPkg** | Package manager (`pkg/`, `nyra pkg …`) |
| **Thin LTO** | Default release link optimization |
| **expand/** | Desugar passes before typecheck — see [contributor-map expand index](docs/contributor-map.md#compilerexpand-module-index) |
| **insta snapshot** | Golden-file test for IR/diagnostics — update with `INSTA_UPDATE=1`, review diff |
| **E004 / E018** | Common diagnostics — cannot infer type / unknown method; run `nyra explain E018` |

**Diagnostic codes** live in `compiler/errors/`. When adding or changing a code, update `compiler/errors/src/explain.rs` and any compiletest `//~ ERROR` lines.

---

## IDE & diagnostics tooling

| Component | Path | Notes |
|-----------|------|-------|
| **LSP** | `lsp/` · `nyra lsp` | Go-to-def, diagnostics in editors |
| **DAP** | `dap/` | Debugger adapter (where enabled) |
| **`nyra diag`** | `cli/src/commands/check.rs` | JSON diagnostics for tooling |
| **Explain codes** | `compiler/errors/src/explain.rs` | `nyra explain E018` |

When changing diagnostic text or codes, update explain entries and any compiletest `//~ ERROR` expectations.

---

## CI overview (what runs on PRs)

Full detail: [`docs/testing-runbook.md`](docs/testing-runbook.md) · workflow: [`.github/workflows/ci.yml`](.github/workflows/ci.yml).

**Pipeline stages:** build → tier1 (fast) → tier2 (medium) → tier3 (heavy) → native — on **Linux, macOS, and Windows**.

| Local command | CI equivalent (approx.) |
|---------------|-------------------------|
| `make test-preflight` | Fast smoke before deep work |
| `make test-triage` | Common failing gates with one report |
| `make test-all` | Full core suite (build, cargo test, nyra-lang, conformance, compiletest, …) |
| `make test-all-linux` / `-macos` / `-windows` | Platform-specific CI core |
| `make test-all-linux-native` etc. | Native smoke per OS |
| `NYRA_SUITE_PROFILE=fast make test-all` | Quicker compiletest subset for iteration |
| `TEST_SAN=1 make test-all` | Optional sanitizer gates |
| `TEST_PERF=1 make test-all` | Performance regression gate |
| `TEST_FUZZ=1 make test-all` | Extended fuzz gates |

`make test-all` runs gates **even after failure** and summarizes at the end — check `target/test-all.txt` and `target/.nyra-test-all-failures` for the full log.

**Tier quick map** (see runbook for full gate list):

| Tier | Examples |
|------|----------|
| **1 fast** | `test-optional-types`, `test-conformance`, `test-cargo-workspace` |
| **2 medium** | `test-nyra-lang`, `smoke-stdlib-priority` |
| **3 heavy** | `smoke-stdlib`, compiletest corpus, runtime smoke |
| **native** | Platform link/run smoke; Windows extras: package + DAP |

---

## Reporting issues

Open a [GitHub issue](https://github.com/nyra-lang/nyra/issues) with:

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
5. **Split large files** before they exceed ~800–1200 lines (see [`docs/contributor-map.md`](docs/contributor-map.md#large-files--split-before-extending) · [`docs/architecture.md`](docs/architecture.md)).

---

## License & contributions

By contributing to Nyra, you agree that your contributions are licensed under the same terms as the project — see [`LICENSE.md`](LICENSE.md) (BSD 3-Clause License).

Report security-sensitive bugs privately via [GitHub Security Advisories](https://github.com/nyra-lang/nyra/security/advisories) or open a minimal issue without exploit details.

---

## Further reading

| Topic | Document |
|-------|----------|
| **What to change → where to go** | [`docs/contributor-map.md`](docs/contributor-map.md) |
| **Makefile & Python generators** | [`docs/make-and-generators.md`](docs/make-and-generators.md) |
| **First contribution & troubleshooting** | This file — [Your first contribution](#your-first-contribution-10-minutes) · [FAQ](#troubleshooting--faq) |
| **NyraPkg & removing features** | [NyraPkg workflow](#nyrapkg-workflow) · [Removing a feature](#removing-a-feature) |
| **Debugging compiler / snapshots** | [Debugging the compiler](#debugging-the-compiler) |
| Language reference (AI + humans) | [`skills/skill.md`](skills/skill.md) · [live docs](https://nyra-lang.github.io/docs/) |
| Toolchain architecture | [`docs/architecture.md`](docs/architecture.md) |
| Stdlib design & auto-prelude | [`stdlib/README.md`](stdlib/README.md) |
| Testing & CI debugging | [`docs/testing-runbook.md`](docs/testing-runbook.md) |
| Stability tiers (Core vs Extended) | [`docs/stability-v1.md`](docs/stability-v1.md) |
| Native C / `nyra cc` | [`docs/native-cc.md`](docs/native-cc.md) |
| C bindgen | [`docs/c-bindgen.md`](docs/c-bindgen.md) |
| Roadmap | [`docs/roadmap-stable.md`](docs/roadmap-stable.md) |
| Design sketches | [`skills/`](skills/) |

Thank you for contributing to Nyra.
