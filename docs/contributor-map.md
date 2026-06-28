# Contributor map — what to change → where to go

Quick navigation for Nyra contributors. Use this when you know **what** you want to change but not **which folder** to open.

For compile pipeline details see [`architecture.md`](architecture.md). For the full contributing checklist see [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

---

## Decision flowchart

```
┌─────────────────────────────────────────────────────────┐
│              What do you want to add or change?           │
└─────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
   Syntax / keyword     Stdlib function       CLI flag
        │                   │                   │
   lexer → parser       stdlib/**/*.ny        cli/src/commands/
   → ast → expand?      (+ rt/*.c if C)       cli/src/app/args.rs
   → typecheck          (+ runtime_map.rs)
   → codegen?
   → const_eval? (comptime)
        │
   tests/nyra/foo.ny + foo.typed.ny
   examples/foo.ny + foo.typed.ny
   grammar/nyra.tmLanguage.json

        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
   Type rules          Ownership / borrow    Generics
   typecheck/          ownership/             monomorph/
   types/              borrowck/              expand/ (synthesis)

        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
   Builtin (print)     Import / prelude      Package manager
   typecheck +         resolve/              pkg/
   codegen +           (prelude.rs)          cli/src/commands/pkg*
   stdlib/rt/

        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
   Comptime eval        Remove / deprecate    (see table below)
   const_eval/          reverse the paths
   (comptime.rs)        above; delete tests,
   + parser/            examples, docs,
   + typecheck/         grammar entries
```

**Removing a feature:** walk the same crates in reverse, then delete matching entries in `tests/nyra/`, `examples/`, `grammar/`, and docs.

---

## Task → location (quick table)

| You want to… | Start here | Also touch |
|--------------|------------|------------|
| **Add syntax / keyword** | `compiler/lexer/` → `parser/` → `ast/` | `expand/` (if sugar), `typecheck/`, `codegen/`, `const_eval/` (comptime), `grammar/nyra.tmLanguage.json` |
| **Add stdlib function** | `stdlib/<module>/` | `stdlib/rt/rt_*.c`, `runtime_map.rs`, `docs/abi-manifest.toml` (if new C symbol) |
| **Add CLI flag / command** | `cli/src/app/args.rs` | `cli/src/commands/` or `cli/src/app/session.rs` |
| **Change type rules** | `compiler/typecheck/` | `compiler/types/` |
| **Fix borrow / move errors** | `compiler/ownership/` | `compiler/borrowck/` |
| **Change generics** | `compiler/monomorph/` | `compiler/expand/` (synthesis helpers) |
| **Comptime behavior** | `compiler/const_eval/` | `parser/`, `typecheck/` if new comptime syntax |
| **Imports / prelude** | `compiler/resolve/` | `prelude.rs`, `symbols.rs` |
| **LLVM / codegen** | `compiler/codegen/` | `runtime_map.rs`, `stdlib/rt/` |
| **Remove a feature** | Same crates as when adding | Delete `tests/nyra/*`, `examples/*`, docs, grammar entries |

---

## Where tests go

| Test kind | Location | When to use |
|-----------|----------|-------------|
| **Feature test (default)** | `tests/nyra/<name>_test.ny` (+ `.typed.ny`) | Every user-visible language/stdlib change |
| **Small repro / fixture** | `tests/nyra/<name>.ny` | Paired with a `*_test.ny` runner |
| **Rust unit tests** | Same Rust module (`#[cfg(test)]`) | Internal helper logic |
| **Driver integration** | `compiler/driver/tests/` | Pipeline, snapshots, ABI manifest |
| **CONF-LANG contract** | `tests/conformance/pass/` or `fail/` | Stable language contract tests |
| **Compiletest grid** | `tests/suite/` | Large pass/fail/run corpus (usually generated) |
| **Runnable demo** | `examples/<topic>/` | User-facing samples (`foo.ny` + `foo.typed.ny`) |

**Rule of thumb:** new language features → `tests/nyra/` first. Only add conformance or suite entries when you need a stable contract or grid coverage.

Run locally:

```bash
nyra test tests/nyra/my_feature_test.ny
make test-nyra-lang
make test-all          # full CI-equivalent suite
```

---

## `examples/` vs `Apps/`

| Folder | Purpose | Size |
|--------|---------|------|
| **`examples/`** | Small demos, builtins, benchmarks, toolchain samples | Single file or small multi-file |
| **`Apps/`** | Reference applications (games, IDE, databases, networking) | Full multi-module projects |

Both use zero-types and typed pairs where applicable. Prefer **`examples/`** for new feature demos; use **`Apps/`** when showcasing a complete application.

---

## `compiler/expand/` module index

The expand crate desugars surface syntax before typecheck. Each file is one pass:

| Module | Desugars |
|--------|----------|
| `arrows.rs` | Arrow / pipeline syntax |
| `async_desugar.rs`, `async_for_in.rs`, `async_state_machine.rs` | Async control flow |
| `future_async.rs`, `future_await.rs`, `future_structs.rs` | `async`/`await` lowering |
| `clone.rs` | Auto-clone synthesis |
| `coerce.rs`, `string_borrow.rs`, `ownership_prefix.rs` | Borrow/coercion helpers |
| `main_argv.rs` | `main` argv handling |
| `match_or.rs` | `match` with `\|` patterns |
| `nullish.rs` | Nullish coalescing (`??`) |
| `struct_ctors.rs`, `struct_serde.rs`, `serde_traits.rs` | Struct helpers |
| `trait_objects.rs` | Dynamic dispatch glue |
| `try_op.rs` | `?` operator |
| `vec_pod.rs`, `vec_nested.rs`, `vec_reloc.rs` | `Vec` synthesis |

When adding syntax sugar, check whether an existing expand pass already handles a similar transform before adding a new one.

---

## Compiler pipeline (load + compile)

**Load time** (before `compile_program`):

```
resolve/  — imports, project graph, prelude injection
```

**Compile time** (`compiler/driver`):

```
lexer → parser → expand → monomorph → typecheck → ownership → borrowck → const_eval → codegen
```

Comptime modules may run `const_eval` during load; see `compiler/driver/src/lib.rs`.

---

## Large files — split before extending

These files exceed the [800–1200 line guideline](architecture.md#file-size-guideline). Prefer splitting them when you next touch the area:

| File | Lines (approx.) | Suggested split |
|------|-----------------|-----------------|
| `compiler/const_eval/src/comptime.rs` | 2100+ | `comptime_expr.rs`, `comptime_stmt.rs`, `comptime_types.rs` |
| `compiler/monomorph/src/lib.rs` | 1900+ | By concern: calls, traits, bounds |
| `compiler/lexer/src/lib.rs` | 1480+ | Token kinds, lexing, literals |
| `compiler/typecheck/src/checker_expr.rs` | 1140+ | By expression category |
| `compiler/parser/src/lang_features.rs` | 1180+ | One module per feature gate |

---

## Further reading

| Topic | Document |
|-------|----------|
| Architecture & file size rules | [`architecture.md`](architecture.md) |
| Full contributing guide | [`../CONTRIBUTING.md`](../CONTRIBUTING.md) |
| Stdlib layout | [`../stdlib/README.md`](../stdlib/README.md) |
| Testing & CI | [`testing-runbook.md`](testing-runbook.md) |
| Release & version bump | [`../agents/skill.md`](../agents/skill.md) |
