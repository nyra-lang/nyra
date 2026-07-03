# Changelog

## v1.40.0 (2026-07-03)

**Batteries-included errors and async**

- **Added** — official application error path in `stdlib/error.ny`: `Error`, context/cause formatting, stack trace hook, and `Result<T, Error>` helpers for common fallible pipelines.
- **Added** — fallible stdlib wrappers for file and JSON work (`read_text`, `write_text`, `append_text`, `json_string`, `json_i32`, `json_bool`) so apps can compose I/O + parsing with `?` without third-party packages.
- **Added** — official async runtime facade in `stdlib/async/mod.ny` (`NyraRuntime_default`, `NyraRuntime_run_until`, `sleep_ms_async`, `await_i32`) over Nyra's in-tree executor.
- **Fixed** — `?` and LLVM lowering for `Result<string, Error>` / struct error payloads, including string payload ownership when unwrapping with `?`.
- **Runtime** — `rt_error.c` adds `error_stack_trace`; `rt_json.c` adds JSON field-type probes for safe fallible wrappers.
- **Tests/examples** — `error_stdlib_test*`, `async_runtime_facade_test*`, `examples/errors_official*`, and `examples/async_runtime_official*`.

## v1.39.0 (2026-07-01)

**Systems-level language features**

- **Added** — C-style `union` with `repr(C)`, `align(N)`, and `packed` layout attributes
- **Added** — compile-time `size_of<T>()` and `align_of<T>()` intrinsics (`stdlib/mem/layout.ny`)
- **Fixed** — heterogeneous enum payloads (`Ok(string)` + `Err(i32)`) with union slot layout and tag-discriminated drop
- **Added** — first-class `bytes` type (distinct from `string`); indexing and `.to_string()`
- **Added** — `StackBuffer` stack-only buffers (`stdlib/buf/stack.ny`) with return-type escape rejection
- **Added** — portable SIMD vectors (`i32x4`, `f32x4`, `f64x2`) and platform intrinsics (`stdlib/simd/`)
- **Added** — arena bump allocator (`stdlib/alloc/arena.ny`, `rt_arena.c`)
- **Tests** — `union_test`, `layout_align_test`, `enum_hetero_payload_test`, `bytes_type_test`, `stack_buffer_test`, `simd_test`, `arena_test`, `systems_features_test`
- **Conformance** — `CONF-LANG-010` … `CONF-LANG-013`
- **Added** — metaprogramming guide: comptime + macros + struct JSON synthesis (`examples/toolchain/metaprogramming.ny`, `stdlib/meta/mod.ny`)
- **Changed** — `random()` / `random(min, max)` and `random_f64()` / `random_f64(min, max)` are compiler builtins; removed `Random()` and `random_range`; integer return type follows bounds or type args (`i32`, `i64`, `u64`, …)
- **Added** — `JoinHandle` and `.join()` for `spawn` expression form; statement `spawn { }` remains fire-and-forget
- **Added** — `spawn:task` (default `spawn`) lightweight task pool (`rt_task_pool.c`) and `spawn:thread` for dedicated OS threads (`pthread` / `CreateThread`)
- **Tests** — `spawn_join_test.ny`; examples `spawn_join.ny`, `spawn_thread.ny`
- **Runtime** — `spawn_capture` returns `void*` handle; `spawn_join` / `spawn_handle_drop`; `spawn_task_capture` / `spawn_task_join` / `spawn_task_handle_drop`

## v1.38.0 (2026-06-28)

**Comptime — Zig-style power (optional)**

- **Added** — comptime **strings** (literals, concat, equality) and **string literal match** arms
- **Added** — `.len()` on comptime arrays and strings; `[elem; N]` / `[elem; param]` array repeat
- **Added** — mutable comptime updates: `table[i] = v`, `s.field = v` (requires `let mut`); immutable `let` reassignment rejected
- **Added** — integer literal match patterns (`match n { 0 => …, 7 => … }`)
- **Added** — comptime modules retain exported **`pub struct` / `pub enum`** definitions
- **Added** — example `comptime_power.ny`; tests `tests/nyra/comptime/power_test*`
- **Docs** — `skills/skill.md` comptime philosophy and expanded capability list

## v1.37.5 (2026-06-28)

**Comptime — structs, enums, and tuples**

- **Added** — comptime struct literals, field access, spread (`{ ...s, x: 1 }`), and struct match patterns
- **Added** — comptime enum values with single or multi-arg payloads (`Opt.Some(42)`)
- **Added** — comptime tuple literals and tuple match patterns
- **Added** — examples `comptime_struct_enum.ny`; tests `tests/nyra/comptime/struct_enum_test*`
- **Docs** — `skills/skill.md` struct/enum/tuple comptime support

## v1.37.4 (2026-06-28)

**Comptime — `match` expressions**

- **Added** — comptime evaluation for `match` on enums, bools, and integers (with `_ if guard` arms)
- **Added** — comptime enum values (`Status.Ok`, payload variants with one argument)
- **Added** — `true` / `false` as bool match patterns in the parser
- **Added** — examples `comptime_match.ny`; tests `tests/nyra/comptime/match_test*`
- **Docs** — `skills/skill.md` match section

## v1.37.3 (2026-06-28)

**Comptime — `comptime { }` blocks and loop control**

- **Added** — `comptime { ... }` block expressions fold to a compile-time value
- **Added** — `while`, `break`, and `continue` in comptime evaluation (including inside `#[comptime]` functions and comptime modules)
- **Fixed** — fold `comptime { }` in `const` initializers even when no `#[comptime]` functions are present
- **Added** — examples `comptime_block_loops.ny`; tests `tests/nyra/comptime/loops_test*`
- **Docs** — `skills/skill.md` updated

## v1.37.2 (2026-06-28)

**Comptime — `#[comptime]` on single functions**

- **Added** — `#[comptime]` attribute on individual functions in normal files; calls fold at compile time and the function is stripped from the binary
- **Fixed** — typed `const` folding preserves declared integer kind (`i64`, etc.) after comptime evaluation
- **Added** — examples `examples/toolchain/comptime_fn_attr.ny` (+ `.typed.ny`); tests under `tests/nyra/comptime/fn_attr*`
- **Docs** — `skills/skill.md` documents file-level `comptime` vs `#[comptime]`

## v1.37.1 (2026-06-28)

**Comptime — `for x in arr` and generic calls**

- **Added** — comptime evaluation for `for x in arr` over fixed arrays and array literals/spreads/index
- **Added** — generic function calls in comptime modules (monomorph before const fold)
- **Changed** — monomorph collects and rewrites generic calls in top-level `const` initializers
- **Updated** — examples/tests for for-in + generics in comptime

## v1.37.0 (2026-06-28)

**Comptime modules — optional compile-time evaluation**

- **Added** — `comptime` file directive (first line only): entire unit is evaluated at compile time; export `pub const` to runtime code via `import`
- **Added** — comptime interpreter: pure functions, `for i in start..end`, integer/bool folding, `if`/`return`/`let mut`
- **Added** — examples `examples/toolchain/comptime_tables.ny` (+ `.typed.ny`) and `comptime_import_main.ny`; tests under `tests/nyra/comptime/`
- **Docs** — `skills/skill.md` comptime section
- **Fixed** — Windows CI link: stop passing `-lpthread` on MSVC (Win32 rt uses native threads; fixes LNK1181)
- **Fixed** — `nyra test` / link tests use `.exe` output names on Windows hosts
- **Fixed** — `cross_windows_uses_triple_subdir_and_exe` uses a non-host Windows triple so it passes on Windows MSVC runners
- **Fixed** — MSVC deprecation noise in `rt_args.c`, `rt_time.c`, `rt_tls.c` (`_strdup`, `memcpy` instead of `strncpy`)
- **Fixed** — flaky `async_state_machine_spawn_test` / macOS CI: nested `spawn`/`unsafe` poll loops no longer complete the outer async promise early (`async_state_machine.rs`)
- **Fixed** — thread-safe async sleep timers (`rt_async.c`: lock `g_timers` in `register_timer` / `process_timers`)

## v1.36.18 (2026-06-28)

**skills — sync from webDocs nyra-skill.md**

- **Updated** — `skills/skill.md` stability tier (v1.36 production-ready, Stable Extended traits) via `node webDocs/scripts/build-nyra-skill.mjs`
- **Docs** — see [`webDocs/CHANGELOG.md`](webDocs/CHANGELOG.md) in the standalone docs repo

## v1.36.17 (2026-06-28)

**CI — Linux + macOS + Windows on every push/PR**

- **CI** — three parallel jobs: `test-linux` (`make test-all`), `test-macos` (`make test-all-macos`), `test-windows` (`make test-all-windows`)
- **Added** — `make/test-platform.mk` shared platform core (cargo tests, conformance, nyra-lang, stdlib smokes)
- **Added** — `make/test-macos.mk` native macOS hello build + run smoke
- **Changed** — Windows/macOS platform core now includes `cargo test --workspace` (codegen snapshots)

## v1.36.16 (2026-06-28)

**Codegen snapshots — cross-platform CI**

- **Fixed** — `normalize_ir` canonicalizes `target triple` to `nyra-snapshot-host` so Linux/macOS/Windows CI share one insta baseline
- **Updated** — all `codegen_snapshots__*.snap` files to the canonical triple

## v1.36.15 (2026-06-28)

**Windows CI — runtime C headers (`unistd.h`)**

- **Fixed** — `stdlib/rt/rt_common.h` guards `<unistd.h>` on `_WIN32`; monotonic time via `QueryPerformanceCounter`
- **Fixed** — `stdlib/rt/rt_time.c` uses `_isatty(_fileno(stdout))` on Windows instead of POSIX `isatty`
- **Fixed** — `stdlib/rt/rt_tls.c` drops unused `<unistd.h>` include

## v1.36.14 (2026-06-28)

**Stdlib compile smoke — type annotations and import paths**

- **Fixed** — `stdlib/games/raylib_gfx.ny` explicit `Gfx3D_Vec3` / `Camera3D` parameter types
- **Fixed** — `stdlib/games/voxel.ny` `get`/`set` return types and wrapper signatures (avoids string `get` inference)
- **Fixed** — `stdlib/net/cache.ny` import paths (`../` not `../../`)
- **Fixed** — `stdlib/parser/ast_row.ny` `AstRow_kind` / `AstRow_text` index types
- **Fixed** — macOS `cross linux smoke` skips when no GNU linux cross toolchain (host clang lacks linux sysroot)
- **Fixed** — `make test-all` banner timestamp on macOS/BSD (`date` via progress helper)

## v1.36.13 (2026-06-27)

**Test fixes — `nyra test tests/nyra` and stdlib helpers**

- **Fixed** — `stdlib/testing.ny` imports `os/syscall.ny` for `os_exit` (removes W002 unused-import noise)
- **Fixed** — `TcpHub.add` returns `i32` status from `rt_tcp_hub_add` (invalid fd returns `-1`)
- **Fixed** — `tests/nyra/games_gaps.ny` bool comparison (`light == false`)
- **Fixed** — `tests/nyra/net/net_advanced_test.ny` `Channel_str` send/recv ownership chain
- **Fixed** — `tests/nyra/net/gaps_fix_test.ny` HTTP handler renamed to `http_handler` (not picked up as `nyra test` case)

## v1.36.12 (2026-06-27)

**Removed Sonic framework from the Nyra repository**

- **Removed** — `sonic/` tree, `import "sonic/..."` resolver, `make smoke-sonic`, and enterprise/microservice Sonic example projects
- **Tests** — retained workspace smoke patterns as `CONF-WS-*` in `conformance/workspace.rs` (`graph_arc_smoke`, `monolith_struct_smoke`, multi-module struct import)
- **Examples** — `examples/net_http_smoke.ny` now uses stdlib `HttpRouter` only (no Sonic HTTP layer)

## v1.36.11 (2026-06-27)

**Link fix — `rt_args.c` requires `rt_vec.c` on Linux**

- **Fixed** — programs that only reference `rt_args_init` now also link `rt_vec.c` (`vec_str_from_argv` calls `vec_str_new` / `vec_str_push`)
- **Fixed** — `examples/packages/ny-sqlite/rt/sqlite.c` includes `<stdint.h>` for `intptr_t` (strict Linux CI clang)

## v1.36.9 (2026-06-27)

**Testing — stronger `make test-all` gates**

- **CI** — Linux cross-compile (linux + mingw windows) runs by default; `TEST_FUZZ=1` on every push; compiletest uses `ci` profile (~3k cases)
- **CI** — Windows job runs `make test-all-windows` (conformance, nyra-lang, stdlib compile + runtime smoke, native build)
- **Tests** — expanded `tests/conformance/` (Option, Result, HashMap, Vec, break, generics fail cases)
- **Tests** — new `examples/stdlib_runtime_smoke.ny` (+ typed) and `smoke-stdlib-runtime` gate
- **Weekly CI** — regenerates `--profile full` compiletest grid + extended fuzz

## v1.36.8 (2026-06-27)

**Runtime fix — `clock_gettime` in `rt_common.h` on Linux**

- **Fixed** — `rt_common.h` includes `<time.h>` and enables `_DEFAULT_SOURCE` on Linux so `clock_gettime` / `CLOCK_MONOTONIC` are declared when runtime `.c` files are compiled by `clang` in CI
- **Fixed** — link step prefers in-repo `stdlib/rt/` when building from a source tree (avoids stale `~/.nyra` copies on CI/dev machines)
- **CI** — install `libsqlite3-dev` for NyraPkg sqlite shim compile tests

## v1.36.7 (2026-06-27)

**Runtime fix — export `async_future_done` for async state-machine spawn bodies**

- **Fixed** — `async_future_done` and `async_future_ptr_value` are now globally visible in `rt_async.c` (removed erroneous `static` forward declarations that hid them from the linker)
- **Fixed** — codegen runtime profile scan also covers `module_level` IR (spawn/closure helper functions)

## v1.36.6 (2026-06-27)

**Typecheck fix — reject bool vs integer comparisons**

- **Fixed** — `true == 1`, `1 >= false`, and similar bool/integer comparisons now report `Type mismatch in comparison` instead of compiling silently
- Removed the FFI-style bool↔integer comparison exemption that bypassed strict type checking

## v1.36.5 (2026-06-27)

**Compiler fix — explicit `Send` / `Sync` struct marker validation**

- **Fixed** — `struct Foo Send { ... }` is now validated against field-derived thread safety instead of trusting the marker circularly
- **Fixed** — self-referential structs like `struct Bad Send { next: &Bad }` correctly reject the explicit `Send` marker

## v1.36.4 (2026-06-27)

**Compiler fix — generic `Result` monomorph in zero-types programs**

- **Fixed** — `Result.Err(0)` and similar inferred `Result` variants monomorphize to concrete LLVM types (e.g. `Result__i32_i32`) like `Option.Some(42)` already did
- **Fixed** — lazy stdlib prelude no longer re-introduces unsized generic `Option`/`Result` LLVM types after monomorph
- **Fixed** — codegen skips emitting LLVM struct types for generic enums still present in the AST

## v1.36.3 (2026-06-27)

**Parser fix — fuzz stress infinite loops**

- **Fixed** — parser no longer hangs on malformed fuzz input when `async` appears without `fn`, or when `impl` blocks contain non-function tokens (e.g. bare `let`)
- **Fixed** — top-level parse loop advances past stuck tokens via `synchronize` when recovery would otherwise spin forever
- Regression tests in `compiler/parser/tests/fuzz_parser_hang.rs`

## v1.36.2 (2026-06-27)

**Compiler fix — serde bin eligibility, trait codegen, stdlib imports**

- **Fixed** — binary serde no longer references `{Struct}_bin_encode` for nested structs that only support JSON (e.g. `HashMap_*` handle wrappers inside auto-serde parents)
- **Fixed** — direct calls to synthesized trait helpers (`Deserialize_{Type}_from_json`) use the correct struct return type in LLVM codegen
- **Fixed** — broken stdlib import paths in `stdlib/net/ftp.ny` and `stdlib/net/icmp.ny`
- **Fixed** — `examples/async_state_machine.typed.ny` uses `Future_i32` for async call results
- Corpus manifest skips intentional error demo `examples/tooling/diag_json.ny`

## v1.36.1 (2026-06-27)

**Compiler fix — struct auto-serde eligibility**

- **Fixed** — lazy-prelude programs no longer fail with undefined `{Struct}_json_encode` when stdlib structs use unsupported field types (e.g. `f64` in `GameAudioSession`)
- Auto `Serialize` / `Deserialize` impls are now synthesized only for structs that actually receive JSON helpers
- **Fixed** — `///` doc comments on top-level `fn` items are preserved (parser no longer consumes docs when probing for `struct`)
- **Fixed** — scalar `match` arms with guards (`v if v == 3`) bind the scrutinee to `v`
- **Fixed** — lazy prelude re-runs after struct serde synthesis so `bin_buf_*` helpers resolve for `Serialize::to_bytes` fallbacks
- **Fixed** — escape analysis codegen: no-escape struct literals skip `str_clone`; runtime profile ignores `declare` lines (local channels no longer pull `rt_channel.c`)

## v1.36.0 (2026-06-24)

**DAP Phase 4 — production debugging** — real LLDB/GDB bridge for VS Code.

### Toolchain / DAP

- **LLDB command session** — interactive `-Q` I/O with prompt sync (macOS Apple lldb)
- **GDB MI2 session** — token-based MI on Linux when `--interpreter=mi2` is available
- **Real breakpoints** — `setBreakpoints` maps to lldb/gdb break commands (queued pre-launch)
- **Stack trace** — parsed from `thread backtrace` / MI stack-list-frames
- **Locals** — `frame variable` / `stack-list-variables` → DAP `variables`
- **Stepping** — distinct continue / next / step-in / step-out
- **Source request** — returns `.ny` file contents to the editor
- **Lifecycle** — `stopped` on entry, `terminated` on exit, launch errors surfaced

### VS Code extension (v1.36.0)

- **`Nyra: build (debug)`** task — `nyra build . --debug-symbols`
- **`nyra.debugAdapterPath`** — respected for DAP
- Default debug **preLaunchTask** uses debug build

### Tests / examples

- `examples/tooling/debug_demo.ny` (+ typed)
- `dap` unit tests — backtrace/variable parsers
- Extended `scripts/cli-smoke.sh` — DAP `setBreakpoints` round-trip

## v1.35.0 (2026-06-24)

**LSM flush & string ownership** — memtable flush to SST now preserves tree state; reads work after L0 flush.

### Stdlib

- **`stdlib/db/lsm.ny`** — extract string fields with `clone` before move; flush builds SST without corrupting `dir`; `LsmTree_lookup` returns `{ tree, value }` for repeated reads; `LsmTree_get` remains a single-use convenience
- **`stdlib/db/sstable.ny`** — `clone` on `StrVec.get` when building SST bodies and merges (fixes double-free on flush)

### Tests / examples

- `tests/nyra/stdlib_gaps.ny` (+ `.typed.ny`) — `test_lsm_flush` uses `LsmTree_lookup`
- `examples/database/lsm_compaction.ny` — lookup-based reads after flush

## v1.34.0 (2026-06-24)

**Official integrated serde** — `Serialize` / `Deserialize` traits, binary codec (NBF v1), compiler synthesis.

### Language / compiler

- **`trait Serialize`** — `to_json(self)`, `to_bytes(self)`; auto-`impl` for eligible structs
- **`trait Deserialize`** — `from_json(json) -> Self`; mangled `Deserialize_{Type}_from_json`
- **Binary codec** — `stdlib/rt/rt_bin.c` (length-prefixed LE fields); `{Struct}_bin_encode/decode` for scalar/nested structs
- **JSON helpers unchanged** — `{Struct}_json_encode/decode` remain for backward compatibility

### Stdlib / tests / examples

- `stdlib/serde/mod.ny`, `stdlib/serde/binary.ny`
- `tests/nyra/serde_traits_test.ny` (+ `.typed.ny`)
- `examples/serde_traits.ny` (+ `.typed.ny`)

## v1.33.0 (2026-06-24)

**Pattern matching — nested enum binds** — match through payload enums in one arm (`Ok(Some(x))`).

### Language / compiler

- **`Type.Variant(Inner.Some(x))`** — nested payload patterns inside `match` binds
- **Shorthand** — `Ok(Some(x))` infers inner enum from payload type (no repeat of `Option_i32`)
- **`MatchPayloadPattern`** — `Bind`, `Wildcard`, `Nested(MatchPattern)` replaces plain `String` bind
- **Codegen** — recursive tag checks on nested payloads; enum-as-payload LLVM layout fix (`llvm_type_of` for payload enums)

### Tests / examples

- `tests/nyra/match_nested_test.ny` (+ `.typed.ny`)
- `examples/language/match_nested.ny` (+ `.typed.ny`)
- `compiler/driver/tests/conformance/language_gaps.rs` — CONF-LANG-007

### Note

Enum variants with **different payload types** now use a max-size union payload slot with tag-discriminated drop (e.g. `Result<string, i32>`).

## v1.32.0 (2026-06-24)

**LSP Phase 3 — VS Code extension polish** — Test Explorer, format-on-save, tasks, snippets, packaging.

### Toolchain / CLI

- **`nyra test --list-json`** — JSON array of `{ file, name, line }` for IDE test discovery
- **`nyra test --filter NAME`** — run only tests whose name contains `NAME`

### VS Code extension (`extensions/nyra` v1.32.0)

- **Test Explorer** — `TestController` via `--list-json` + `--filter`
- **Format on save** — default for `[nyra]` in extension `configurationDefaults`
- **Status bar** — Nyra toolchain version (`nyra.showVersionInStatusBar`)
- **Bundled toolchain** — optional `nyra.useBundledToolchain` + `bin/nyra-<platform>`
- **Problem matcher** — `$nyra` for build/check/test tasks
- **Snippets** — fn, test, struct, let, import, match
- **Tasks** — build, run, check, test, fmt with problem matcher
- **Packaging** — `scripts/package-vscode-extension.sh` (`BUNDLE_NYRA=1` to embed binary)

### Tests / examples

- `examples/tooling/test_list_json.ny` (+ typed) — test discovery demo
- Extension TypeScript compile wired in `scripts/test-all.sh`

## v1.31.0 (2026-06-24)

**Async state-machine — `Future<string>`** — typed string awaits in cooperative poll loops.

### Language / compiler

- **`PollKind::String`** — `async_future_done` + `async_await_ptr` in state-machine desugar (not blocking `async_await_ptr` at top level)
- **Cooperative re-poll** — stay in the same state until `async_poll` / `async_future_done` reports ready (fixes premature advance)
- **`__nyra_await_result`** — initialized to `""` / `false` / `0` based on async return type
- **Handle extraction** — `await` on `Future_*` still pulls `.handle` before poll

### Tests / examples

- `tests/nyra/async_state_machine_string_test.ny` (+ `.typed.ny`)
- `examples/async_state_machine_string.ny` (+ `.typed.ny`)

## v1.30.0 (2026-06-24)

**Pattern matching — or-patterns** — combine multiple `match` patterns that share the same arm body.

### Language / compiler

- **`A | B => body`** — or-patterns on enum variants (`Color.Red | Color.Blue => 1`) and string literals (`"GET" | "HEAD" => 200`)
- **Expand desugar** — `compiler/expand/match_or.rs` flattens or-arms before typecheck/codegen (no LLVM changes)
- **Parser** — `|` (not `||`) between pattern atoms; `fmt` preserves or-syntax

### Tests / examples

- `tests/nyra/match_or_test.ny` (+ `.typed.ny`) — enum + string or-patterns
- `examples/language/match_or.ny` (+ `.typed.ny`)
- `compiler/driver/tests/conformance/language_gaps.rs` — CONF-LANG-006

## v1.29.0 (2026-06-24)

**Trait objects (complete MVP)** — multi-method vtables, heap drop, and incremental cache invalidation on compiler bump.

### Language / compiler

- **Multi-method vtable indexing** — `__dyn_{Trait}_{method}` loads the correct vtable slot (was always slot 0)
- **Trait object `Drop`** — vtable drop thunk per concrete type; `__dyn_{Trait}_drop` frees boxed heap data
- **`Dyn_*` custom drop** — synthesized `Drop` impl routes to `__dyn_{Trait}_drop`
- **Incremental cache** — source fingerprint includes compiler version so codegen fixes invalidate stale LLVM IR

### Tests / examples

- `tests/nyra/trait_dyn_multi_test.ny` (+ `.typed.ny`) — `add` + `mul` through `dyn Calc`
- `tests/nyra/trait_dyn_drop_test.ny` (+ `.typed.ny`) — boxed trait object cleanup
- `examples/trait_dyn_multi.ny` (+ `.typed.ny`)
- `compiler/driver/tests/conformance/trait.rs` — CONF-TRAIT-005 (vtable index 1 for `mul`)

## v1.28.0 (2026-06-24)

**LSP Phase 2 — IDE depth** — semantic tokens, inlay hints, code actions, signature help.

### Toolchain / LSP

- **Semantic tokens** — keywords, functions, variables, types, literals (beyond TextMate grammar)
- **Inlay hints** — inferred types on `let x = ...` without explicit annotation (zero-types DX)
- **Code actions** — quick fixes from compiler `help:` (`borrow instead`, `clone`, etc.)
- **Signature help** — active parameter inside `(` (trigger: `(` `,`)
- **Workspace symbols** — `#` / Go to Symbol in workspace
- **Document highlight** — read occurrences of symbol under cursor
- **Rename** — span-accurate `TextEdit`s instead of full-document replace

### Tests / examples

- `examples/tooling/lsp_inlay.ny` (+ typed) — inlay hint demo
- Extended `scripts/cli-smoke.sh` — semantic tokens + code actions capabilities

## v1.27.0 (2026-06-24)

**Move-safe `Vec<T>` for relocatable structs** — parallel column storage with nested struct flattening.

### Language / compiler

- **`synthesize_vec_reloc_helpers`** re-wired in compiler driver (was missing from pipeline)
- **Flattened nested reloc fields** — `Vec<NestedRow>` with `inner: InnerTag { tag: string, … }` → `inner_tag_vec`, `inner_weight_vec`, …
- **`StrVec` / `Vec<string>` field support** in reloc eligibility
- **Synthesis order** — `vec_pod` → `vec_reloc` → `vec_nested` before `struct_json` (avoids stale `handle` serde on `Vec__*` structs)
- **`Vec__*` excluded** from auto JSON serde
- **`dyn Trait` drop glue** — `__dyn_{Trait}_drop` invoked when `Dyn_*` locals go out of scope
- **Typed cooperative poll** — state-machine `await` on `Future<bool>` uses `async_poll_bool`; `Future_*` handle extraction before poll
- **Incremental fingerprint** — per-crate manifest hash folded into source fingerprint for cache keys

### Tests / examples

- `tests/nyra/vec_reloc_test.ny`, `vec_reloc_test.typed.ny`
- `examples/collections/vec_reloc.ny` (+ `.typed.ny`)
- `compiler/driver/tests/conformance/vec_reloc.rs` — CONF-VEC-RELOC-*
- **`dyn Trait` drop** — vtable drop thunk + `emit_drop_local` for `Dyn_*` locals (`trait_dyn_drop_test.ny`)
- **Typed async state-machine poll** — `async_poll_bool` for `Future<bool>` awaits (`async_state_machine_bool_test.ny`)
- **Incremental Stage 1** — `CrateManifest::combined_hash()` mixed into build fingerprint

## v1.26.0 (2026-06-24)

**Async runtime v2** — typed `Future<T>`, `select`, and compiler integration.

### Language / compiler

- **`Future<T>` monomorph** — `Future_i32`, `Future_bool`, `Future_string` (`Future<i32>` syntax aliases)
- **`async fn` call sites** — return `Future_T` (FFI `export async fn` still returns raw `i32` handle)
- **`await` on `Future_*`** — typed results (`i32` / `bool` / `string`); raw `i32` handles still work
- **Typed promise runtime** — `async_promise_complete_bool/ptr`, `async_await_bool/ptr`, `async_future_done`
- **Synthesized `Future_*` structs** — auto-emitted when `async fn` is present

### Stdlib / tests / examples

- `stdlib/async/future.ny` — `Future_*`, `Future_select2_*`, `SelectResult_*`
- `examples/async_future.ny`, `examples/async_select.ny` (+ `.typed.ny`)
- `tests/nyra/async_future_test.ny`, `async_select_test.ny` (+ `.typed.ny`)

## v1.25.0 (2026-06-24)

**Nested `Vec<Vec<i32>>` MVP** — 2D dynamic grids with generic syntax and deep-free.

**Maturity pass** — `Vec<string>`, string-array JSON, `Matrix2D`, `RowVec`.

### Language / compiler

- **`Vec<Vec<T>>` monomorph** — `Vec__Vec__i32` + synthesized `Vec_Vec_i32_new/push/get/len/free/push_handle`
- **Parser** — `Vec<Vec<i32>>` type syntax (`>>` split for nested generic closes)
- **Runtime** — `vec_bytes_push_ptr`, `vec_bytes_get_ptr` in `rt_vec.c`
- **`Vec<string>` generic syntax** — monomorph alias → `StrVec` + `Vec_string_*` helpers
- **Struct JSON** — `StrVec` / `Vec<string>` fields via `json_encode_str_array` / `json_decode_str_array`
- **`vec_pod` guards** — skip string-field structs and non-`handle` vec layouts

### Stdlib / tests / examples

- `stdlib/collections/nested_vec.ny` — import `Vec<Vec<i32>>` programs
- `stdlib/collections/matrix2d.ny` — growable row-major 2D matrix
- `stdlib/collections/row_vec.ny` — parallel-column vector for string+scalar rows
- `json_encode_str_array` / `json_decode_str_array` in `rt_json.c`
- `tests/nyra/nested_vec_test.ny`, `nested_vec_test.typed.ny`
- `tests/nyra/maturity_v120_test.ny` (+ `.typed.ny`) — zero-types + typed
- `examples/collections/nested_vec.ny`, `matrix2d.ny` (+ `.typed.ny`)
- `examples/trait_dyn_string.ny` (+ `.typed.ny`)

## v1.24.0 (2026-06-24)

**Database follow-up** — SQL UPDATE/DELETE parsing, B-tree ordered range scan.

### Stdlib

- **`stdlib/db/sql_parse.ny`** — `UPDATE table SET col = val WHERE …`, `DELETE FROM table WHERE …`
- **`stdlib/collections/btree_pages.ny`** — `BTreePaged_range`, `BTreePaged_keys` (in-order scan)

### Tests / tooling

- Extended `tests/nyra/stdlib_gaps.ny` — UPDATE/DELETE parse, btree range
- **`scripts/database-smoke.sh`** — stdlib gaps + sqlite smoke; wired into `scripts/test-all.sh`
- `examples/database/btree_range.ny`; updated `sql_parse.ny` example

## v1.23.0 (2026-06-24)

**Developer experience — diagnostics + LSP reliability** — `nyra explain`, richer JSON diagnostics, incremental LSP sync.

### Toolchain / LSP

- **`nyra explain E003`** — static explanations for all stable codes (`E*`, `P*`, `W*`, `L001`); `nyra explain --list`
- **`nyra diag --json`** — now includes `code`, `label`, `notes`, `helps`, `end_line`, `end_column`
- **LSP reliability** — incremental document sync, 250ms debounced diagnostics, `didClose` clears diagnostics, `didChangeWatchedFiles` refreshes open files
- **LSP diagnostics** — `related_information` uses real file URIs for labels, notes, and helps

### Tests / examples

- `examples/tooling/diag_json.ny` (+ typed) — intentional type error for JSON/explain demos
- Extended `scripts/cli-smoke.sh` — `explain`, enriched `diag --json`, LSP completion + goto-def

## v1.22.0 (2026-06-24)

**Games stdlib maturity** — dynamic 2D grids, ECS stores, voxel chunks, audio helpers, 3D camera ABI.

### Stdlib / runtime

- **`stdlib/games/`** — `Grid2D_i32`, `EcsWorld` + component stores, `VoxelChunk_i32`, `gfx3d` orbit/isometric math, `audio` path helpers; optional `raylib_audio` / `raylib_gfx`
- **`vec_i32_set`** — in-place vector update (`rt_vec.c`)

### Games / raylib ABI

- Games vendor `raylib.ny` — `Camera3D`, `Music`, typed `BeginMode3D` / music stream APIs
- `MinecraftClone` — stdlib voxel + isometric/3D toggle

### Tests / examples

- `tests/nyra/games_stdlib.ny` (+ typed); `examples/games/grid2d`, `ecs`, `voxel_chunk`, `audio_paths`

## v1.21.0 (2026-06-24)

**Database production maturity** — full LSM compaction, real B-tree internal descent, advanced SQL parser, SQLite streaming cursor.

### Stdlib / runtime

- **`stdlib/db/lsm.ny`** — memtable + WAL + leveled L0/L1 SST compaction, tombstones, WAL truncate on flush
- **`stdlib/db/sstable.ny`** — `sstable_merge_files` for sorted merge (newer file wins on duplicate keys)
- **`stdlib/collections/btree_pages.ny`** — internal node descent, leaf + internal splits, `BTREE_PAGE_MAX = 8`
- **`stdlib/db/sql_parse.ny`** — `SqlParse_parse` for `SELECT … WHERE col op val` and `INSERT INTO … VALUES (…)`
- **SQLite streaming cursor** — `SqliteDb.prepare`, `SqliteStmt.step/col/finalize`, `last_error` (`rt_sqlite.c`)
- **`stdlib/db/sql.ny`** — `SqlDb.query_rows` delegates to SQLite rowset

### Tests / examples

- Extended `tests/nyra/stdlib_gaps.ny` — btree pages, LSM, SQL parser
- `tests/fixtures/sqlite_smoke` — `query_rows` + `prepare` smoke
- `examples/database/lsm_compaction.ny`, `sql_parse.ny`; updated `btree_split.ny`, `sqlite_rows.ny`

## v1.20.0 (2026-06-24)

**Networking production maturity** — verified TLS client connections, production cert workflow, ICMP fallbacks without root.

### Stdlib / runtime

- **`tls_connect_verify` / `tls_connect_ca`** — HTTPS clients verify server certificates (system CA store or custom `NYRA_SSL_CA_FILE`)
- **`tls_validate_pem` / `tls_last_error`** — validate PEM cert/key before listen; OpenSSL error strings for ops
- **`stdlib/net/tls_prod.ny`** — `tls_listen_prod`, `tls_connect_prod`, `tls_upgrade_prod` via `NYRA_TLS_CERT` + `NYRA_TLS_KEY`
- **`ws_listen_prod_on`** — production `wss://` listener using env cert paths
- **`ping_icmp_system` / `ping_icmp_capable`** — OS `ping` fallback when raw ICMP unavailable; Linux unprivileged `SOCK_DGRAM` ICMP when permitted
- **`ping_auto`** — ICMP → system ping → TCP fallback chain
- **HttpPool / `get("https://…")` / SMTP TLS** — use certificate verification by default

### Tests / examples

- `tests/nyra/net/net_prod_test.ny`, `tests/nyra/net/net_prod_test.typed.ny`
- `examples/net/tls_prod_smoke.ny`

## v1.19.0 (2026-06-24)

**Remaining gaps closed** — Redis TCP server, compiler in-process FFI, raygui stdlib, B-tree page splits.

### Stdlib / runtime

- **`stdlib/db/redis_server.ny`** — `RedisServer_serve`, `RedisServer_serve_forever` (RESP + TCP)
- **`stdlib/collections/btree_pages.ny`** — `BTreePaged_str_str` with leaf split + HashMap node pool
- **`stdlib/gui/raygui.ny`** — `Raygui_button`, `GuiTextBox`, … (requires `link raylib`)
- **Compiler in-process FFI** — `libnyra_compiler.dylib` (`compiler-ffi` crate); `check_inprocess`, `diag_json_inprocess` in `stdlib/compiler.ny`

### CLI / link

- Auto-link `libnyra_compiler` when compiler FFI symbols are used (`rt_compiler.c` anchor)

### Tests / examples

- `examples/database/redis_server.ny`, `btree_split.ny`; `examples/dev/compiler_inprocess.ny`

## v1.18.0 (2026-06-24)

**Stdlib & toolchain gaps** — SQLite row cursor, RESP2, real sorted `BTreeMap`, SSTable + `fsync`, `stdlib/pkg.ny`, `--sanitize`, raygui catalog.

### Stdlib / runtime

- **SQLite row cursor** — `sqlite_query_rows`, `SqliteRowset.rows/cols/at/free` (`stdlib/db/sqlite.ny`, `rt_sqlite.c`)
- **RESP2 subset** — `stdlib/db/resp.ny` encode/decode for arrays and bulk strings
- **`BTreeMap_str_str` / `BTreeMap_str_i32`** — sorted `StrVec` + binary search (`stdlib/collections/btree_map.ny`; replaces HashMap alias in `advanced.ny`)
- **SSTable + durability** — `stdlib/db/sstable.ny`, `fsync_file()` in `rt_fs.c`
- **`stdlib/pkg.ny`** — `pkg_verify`, `pkg_install`, `pkg_publish`, `pkg_add` via `exec(nyra, …)`
- **`compiler.ny`** — `build()`, `fmt()`, `run()` subprocess helpers

### CLI

- **`nyra build --sanitize`** — AddressSanitizer (`-fsanitize=address`) for debug builds
- **`nyra pkg c add raygui`** — raygui header catalog entry (links `raylib`)

### Parser / compiler apps

- **`Comb_or`** alias for `Comb_or_literal`
- **JSONParser** — array summary parsing (`Json_parse_array_summary`)

### Tests / examples

- `tests/nyra/stdlib_gaps.ny`, `tests/nyra/stdlib_gaps.typed.ny`
- `compiler/driver/tests/conformance/stdlib_gaps.rs`
- `examples/database/sqlite_rows.ny`, `btree_map.ny`, `resp.ny`; `examples/dev/pkg_verify.ny`

## v1.17.0 (2026-06-24)

**Language gaps suite** — `i64_to_string`, `match` on strings, struct inference/return fixes, `continue` with multiple `mut` loop vars.

### Language / compiler

- **`i64_to_string(n: i64) -> string`** — format timestamps and large integers (`stdlib/strings.ny`, `rt_strings.c`)
- **`match` on strings** — string literal arms (`"GET" => …`) desugar to `str_cmp` branches
- **Struct inference across fn boundaries** — `StructLiteral` / struct `FieldAccess` call-site hints; `Point { … }` at call sites
- **Struct return with nested heap fields** — deep-copy strings and nested structs in struct literals; always heap-own string fields subject to drop
- **`continue` + multiple `mut` loop vars** — `sync_loop_latch_regs` before phi back-edge (SSA/PHI for 2+ carried locals)
- **Nested struct drop IR** — fix missing `%` on `drop_gep` in composite drop glue

### Compiler apps gaps (v1.17.0)

- **`Vec<T>` POD** — `vec_bytes_*` runtime + synthesized `Vec_{Struct}_*` helpers for Copy structs
- **`HashMap<K,V>` generic syntax** — monomorph aliases to `HashMap_str_i32` / `HashMap_str_str`
- **`Comb_or_literal` / `Comb_or_take` / `Comb_many`** — parser combinator alternation + repetition
- **`AstRow`** — parallel kind/text vectors for AST storage
- **`continue` + 2+ `mut` locals** — latch reg sync on `while` continue paths
- **Struct → `ptr` at FFI boundary** — Copy struct args coerce to pointer for `vec_bytes_push/get`
- **JSONParser** — top-level object key/value rows via `KvVec`

### Tests / examples

- `tests/nyra/parser_gaps_test.ny`, `tests/nyra/parser_gaps.typed.ny`
- `examples/parser/combinators.ny` — `Comb_or` + `Comb_many` demo

## v1.16.0 (2026-06-24)

**Games suite gaps** — trig stdlib, loop `continue` codegen, array repeat expressions, array-param inference, game helpers. **Networking runtime polish** — HashMap refcount, custom `Drop` codegen fix, dev TLS/ping helpers.

### Language / compiler

- **`[0; COLS * ROWS]`** — array repeat count may be a const-folded expression (not only a single literal name)
- **`continue` in `while`** — latch block fixes PHI backedges (nested `if` + `continue`)
- **`bool` vs `i32` compare** — allowed in conditions; codegen aligns operands
- **`i32` → `f64`** — numeric promotion in mixed arithmetic (e.g. paddle × `GetFrameTime()`)
- **Array parameter inference** — call-site `let` bindings + refreshed function signatures for codegen
- **Custom `Drop` glue** — drop calls pass `Struct*` (fixes stack corruption / SIGSEGV on `HashMap`, `TtlCache`, `StrVec`, sync handles)

### Stdlib / runtime

- **`sin` / `cos` / `atan2` / `tan`** — `stdlib/math.ny`, `rt_math.c` (compiler intrinsics; no libc `sin` symbol clash)
- **`random_f64()`** — unit interval random (`rand_f64` in `rt_random.c`)
- **`stdlib/terminal/raw.ny`** — `terminal_raw_on/off`, `terminal_read_key` (`stdin_set_raw_mode`, `stdin_read_key`)
- **`stdlib/time/fixed_step.ny`** — `FixedStep` accumulator for fixed-Hz simulation ticks
- **HashMap refcount** — `map_str_*_retain` + refcounted handles (`rt_map_handle.h`); `insert`/`remove` retain on copy; `TtlCache_put` mutates in place
- **`tls_require`**, **`tls_dev_ensure`**, **`tls_listen_dev`**, **`ws_listen_dev_on`** — dev self-signed certs when OpenSSL is present
- **`ping_auto_verbose`**, **`ping_icmp_hint`** — clearer ICMP/root vs TCP fallback messages

### Tests / examples

- `tests/nyra/games_gaps.ny`, `tests/nyra/games_gaps.typed.ny`
- `examples/games/trig_raycast.ny`, `array_repeat_mul.ny` (+ typed variants)
- `Apps/Games/shared/tetris.ny`, `flood_fill.ny`
- `tests/nyra/gui_gaps_test.ny`, `examples/stdlib/gui_helpers.ny` — GUI gap fixes (continue, argv, StringBuilder, …)
- `stdlib/gui/` — `TextBuffer`, `ScrollState`, `FilePicker`, syntax highlight helpers
- `Apps/GUI apps/` — seven raylib smoke apps updated to use new stdlib
- `tests/nyra/net/gaps_fix_test.ny`, `tests/nyra/net/map_drop_test.ny` — wired in `scripts/test-all.sh`
- `CONF-OWN-004b` — custom drop IR uses struct pointer

## v1.15.0 (2026-06-24)

**Networking polish** — handle-safe HashMap updates, dev TLS certs, clearer ping/TLS messages.

### Stdlib / runtime

- **HashMap / StrVec / sync handles** — mutating methods `return self` (fixes double-free on `insert` chains)
- **`tls_require`**, **`tls_dev_ensure`**, **`tls_listen_dev`** — self-signed dev certs (`rt_tls_gen_self_signed`)
- **`ws_listen_dev_on`** — `wss://` without manual cert files when OpenSSL is available
- **`ping_auto_verbose`**, **`ping_icmp_hint`** — explains ICMP/root fallback to TCP

### Tests

- `tests/nyra/net/map_drop_test.ny` — HashMap insert drop safety
- Extended `tests/nyra/net/gaps_fix_test.ny` — runtime `TtlCache_put`

## v1.14.0 (2026-06-24)

**Dev tooling APIs** — process capture, compiler bridge, doc comments, alloc tracking.

### Language

- **`continue`** — skip to next `while` / `for` iteration (documented + conformance tests)
- **`///` doc comments** — attached to the following `fn` / `struct` in the AST (`doc` field)

### Stdlib / runtime

- **`exec(program, args) -> ExecResult`** — subprocess with captured stdout/stderr (`command_exec_capture`)
- **`Command.output()`** — same as `exec` for a built command
- **`stdlib/compiler.ny`** — `check(path)`, `diag_json(path)` via `nyra` subprocess (`NYRA_HOME` or `PATH`)
- **`alloc_track_start` / `alloc_track_note` / `alloc_track_end`** — RSS + estimated byte notes (dev probes)

### Tests / examples

- `tests/conformance/pass/control/continue_*.ny`
- `examples/process_exec.ny`, `examples/compiler_check.ny`, `examples/doc_comments.ny`, `examples/control_continue.ny`

### Networking gap fixes (v1.14.0)

- **`Send`** on `TcpStream`, `TcpListener`, `TcpHub`, `WebSocket`, `WebSocketListener`
- **Callback inference** — zero-types handlers for `serve_handlers(host, …, fn(i32, RequestContext) -> HttpResponse)`
- **`HttpPool`** — HTTPS keep-alive via TLS handles (`POOL_TLS_BASE`)
- **`ws_listen_tls_on` / `ws_accept_tls`** — `wss://` server (`ws_listen_tls`, `ws_accept_tls_handshake` in `rt_websocket.c`)
- **`TtlCache`** — in-memory TTL + optional disk tier (`stdlib/net/cache.ny`)
- **Codegen** — hoist struct types before spawn helpers; drop duplicate `declare`/`define` pairs
- `tests/nyra/net/gaps_fix_test.ny`, `examples/net/gaps_fix_smoke.ny`

## v1.12.0 (2026-06-24)

**Advanced networking** — ICMP, STARTTLS, handler router, broadcast hub, FTP RETR, HTTP pool.

### Stdlib / runtime

- **`ping_icmp` / `ping_auto`** — raw ICMP when root (`rt_icmp_ping_ms`), else TCP fallback
- **`tls_upgrade_fd`** — STARTTLS on existing TCP (`rt_tls_upgrade_client`)
- **`Smtp_send_starttls`** — SMTP on port 587 with upgrade
- **`Ftp_retr` / `Ftp_stor`** — PASV download + upload
- **`HttpRouter_register_slot`**, **`Http_dispatch_slot`**, **`serve_handlers`**
- **`HttpPool` / `HttpPool_get`** — keep-alive connection reuse (plain HTTP)
- **`Channel_str`** — string channels for concurrent apps
- **`TcpHub`** — mutex-protected broadcast to TCP client fds (`spawn` chat)

### Tests / examples

- `tests/nyra/net/net_advanced_test.ny`
- `examples/net/advanced_smoke.ny` (+ typed)

## v1.11.0 (2026-06-24)

**Networking stdlib** — closes gaps for `Apps/Networking apps/`.

### Stdlib / runtime

- **`dns_lookup`** — `getaddrinfo` via `rt_dns_lookup` → `StrVec` of IPs
- **`tcp_connect_timeout`**, **`ping_tcp`** — timed connect + TCP RTT (`rt_tcp_ping_ms`)
- **WebSocket server** — `ws_listen`, `ws_accept_handshake`, `ws_send_text_server`
- **`stdlib/net/ftp.ny`** — `Ftp_login`, `Ftp_list` (PASV data channel)
- **`tcp_accept_task` / `tcp_accept_wait`** — background accept + poll
- **`Smtp_send_tls`** — SMTP over OpenSSL (`tls_connect`)
- **HTTP** — `wants_keep_alive`, chunked `body_from_raw`, `HttpRouter`, keep-alive server loop
- **`stdlib/net/poll.ny`** — `poll_wait`, `tcp_relay_poll` for proxies

### Tests

- `tests/nyra/net/stdlib_gaps_test.ny`

## v1.10.0 (2026-06-24)

**arm64 Apple FFI** — fix raylib `Image` / `Texture` crashes (sret + indirect args).

### Language / compiler

- **`repr(C)` structs &gt; 16 bytes on arm64-apple** — extern returns use `sret`; parameters use indirect `ptr` (matches Darwin ABI for `GenImageColor`, `LoadTextureFromImage`, `DrawTexture`, etc.)
- Conformance: `conf_ffi_014_arm64_indirect_texture_image_abi`

### Apps

- **Graphics suite** — shared `Gfx_window_*` helpers; ImageViewer uses `DrawTextureEx` zoom; all raylib apps run on Apple Silicon

## v1.9.1 (2026-06-24)

**Async spawn / for-in fixes** — nested spawn codegen, param array capture, range-for hoisting.

### Language / compiler

- **Nested `spawn { await … }`** — restore outer `emit_buf` when emitting inner spawn body IR (fixes invalid LLVM from nested poll loops)
- **`for x in arr` + `await` with param `arr`** — hoist pre-loop setup lets outside poll `while`; fix spawn capture size for `[N x T]` fields (promise handle was truncated)
- **`await` in `spawn`** — re-enabled in `async_state_machine_spawn_test.ny`

### Tests

- `async_state_machine_for_in_param_test.ny`
- `async_state_machine_spawn_test.ny` — `test_await_in_spawn`

## v1.9.0 (2026-06-24)

**MVP completion** — native race runtime, async CFG extensions, opaque ptr JSON.

### Toolchain

- **`nyra build --race-native`** — links `stdlib/rt/rt_race.c` (lightweight lock-set detector; alternative to TSan `--race`)
- `scripts/race-native-check.sh` wired in `test-all.sh`

### Language / compiler

- **Async post-typecheck pipeline** — `for-in` desugar → state-machine retry → blocking fallback (`finish_async_desugar`)
- **`await` in `spawn` / `unsafe`** — cooperative CFG lowering (nested poll loops)
- **`for x in arr` with `await`** — iterable desugar to indexed range loop + state-machine CFG (local, array literal, and function params)
- **Struct JSON `*T` / opaque ptr** — `json_encode_ptr_token` / `json_decode_ptr_token` for `RawPtr` fields

### Tests

- `async_state_machine_for_in_test.ny`, `async_state_machine_spawn_test.ny`
- `CONF-ASYNC-006`, `CONF-SERDE-STRUCT-003`

## v1.8.0 (2026-06-24)

**Print fixed arrays** — `print`, `write`, and `println` accept fixed-size arrays of printable scalars (Rust-style debug formatting).

### Language / compiler

- **`print([1, 2, 3])`** — formats as `[1, 2, 3]` (also `f32`/`f64`, `bool`, `string` element arrays)
- Runtime helpers: `array_i32_debug_string`, `array_f64_debug_string`, `array_f32_debug_string`, `array_bool_debug_string`, `array_str_debug_string`
- **`Vec_str_*` runtime aliases** — synthesized struct JSON helpers resolve to `vec_str_*` C symbols
- **Anonymous struct literals** (`__Anon*`) skip auto JSON synthesis (spread-only structs)

### Tests / examples

- `tests/nyra/print_array.ny`, `print_array.typed.ny`
- `examples/builtins/io/print_array.ny`, `print_array.typed.ny`

## v1.7.0 (2026-06-24)

**Race detector + async control flow + collection JSON** — completes remaining v1.6 gates.

### Toolchain

- **`nyra build --race`** — links with ThreadSanitizer (`-fsanitize=thread`) for runtime data-race detection
- `scripts/race-check.sh` wired in `test-all.sh`

### Language / compiler

- **Async CFG desugar** — `await` inside `if` / `while` / `for ..` (range) uses cooperative `async_poll` (hoisted locals + branch/loop states)
- **Struct JSON post-monomorph** — synthesis runs after monomorph (before prelude) so `Box__T`-style structs are eligible
- **Collection / array JSON** — `ptr` (`Vec_i32`), `Vec<i32>`, and `[T; N]` fields via `json_encode_i32_array` / `json_decode_i32_array`

### Tests

- `async_state_machine_if_test.ny`, `async_state_machine_while_test.ny`
- `struct_serde_vec_test.ny`, `struct_serde_array_test.ny`

## v1.6.0 (2026-06-24)

**Async state machines + Send/Sync checks + nested struct JSON** — completes v1.5 production priorities.

### Language / compiler

- **Async state-machine desugar** — linear `async fn` bodies with top-level `await` compile to cooperative poll loops (`async_poll` + `runtime_executor_tick`); nested control flow still uses spawn + blocking `async_await`
- **Send / Sync on `dyn` casts** — `type_is_send` / `type_is_sync` reject non-thread-safe types (e.g. raw pointers) for `dyn Trait + Send` / `+ Sync`
- **Nested struct JSON** — `{Struct}_json_encode/decode` supports nested struct fields (fixed-point eligibility)
- **LSP** — goto-definition for synthesized `{Struct}_json_encode` / `_json_decode` jumps to struct definition; hover notes for synthesized symbols

### Tests & examples

- `examples/async_state_machine.ny`, `examples/struct_serde_nested.ny` (+ `.typed.ny`)
- `tests/nyra/async_state_machine_test.ny`, `struct_serde_nested_test.ny`
- `CONF-ASYNC-005`, `CONF-SERDE-STRUCT-002`, `CONF-TRAIT-004`

### Arrays

- **`.sort_by(cmp)`** — custom comparator on fixed arrays; `cmp(a, b) -> i32` (`<0` / `0` / `>0`); any element type including structs
- **`.sort()`** unchanged — `i32` / `f64` numeric sort only

## v1.5.0 (2026-06-24)

**Production priorities** — non-blocking async fn, struct JSON, trait objects, LSP polish; plus string replace semantics.

### Language / compiler

- **`async fn` desugar** — body runs in `spawn`; call site returns promise handle immediately (`async_promise_new` + `async_promise_complete`)
- **Struct JSON synthesis** — `{Struct}_json_encode` / `{Struct}_json_decode` for concrete structs with `string` / `i32` / `bool` fields (skips runtime handles like `StrVec`)
- **`dyn Trait + Send + Sync`** — parser + typecheck for auto-trait bounds on trait object casts
- **LSP** — diagnostics include notes, helps, and labels; keyword completion snippets (`fn`, `async`, `struct`, `test`, …)

### String methods

- **`.replace(from, to)`** — replaces all occurrences (was first-only)
- **`.replacen(from, to, count)`** — replaces at most `count` occurrences (`1` = first only)

### Tests & examples

- `examples/async_spawn.ny`, `examples/struct_serde.ny`, `examples/trait_dyn_send.ny` (+ `.typed.ny`)
- `tests/nyra/async_spawn_desugar_test.ny`, `struct_serde_test.ny`, `dyn_send_test.ny`
- `CONF-ASYNC-004`, `CONF-SERDE-STRUCT-001`, `CONF-TRAIT-003`
- `tests/nyra/str_replace_replacen_test.ny`, `examples/syntax/string_replace.ny`

## v1.4.0 (2026-06-24)

**Production async executor** — event loop, timers, and IO pump while awaiting.

### Async runtime

- **`runtime_executor_tick(ms)`** — IO wait + timer dispatch in one tick
- **`runtime_executor_run_until(handle, timeout_ms)`** — drive executor until a promise completes
- **`async_sleep_ms(ms)`** — non-blocking sleep promise (requires executor pump / await)
- **`async_await`** — pumps executor + timed wait (fixes spawn+await deadlocks with IO)
- Stdlib: `Executor_tick`, `Executor_run_until`, `Executor_sleep_ms` in `stdlib/async_v1.ny`

### Tests & examples

- `examples/async_executor.ny`, `examples/async_executor.typed.ny`
- `tests/nyra/async_executor_test.ny`, `CONF-ASYNC-003`

## v1.3.0 (2026-06-24)

**Trait bounds on generic functions** — `fn f<T: Trait>(x: T)` with validation at monomorph sites.

### Language

- **Parser** — `T: Trait` and `T: A + B` on generic type parameters
- **Monomorph** — trait-bound errors when concrete type lacks `impl Trait for Type`
- **Typecheck** — method calls on bounded generic params (e.g. `x.hello()` when `T: Greet`)
- **Monomorph fixes** — collect/rewrite generic calls nested inside other call arguments (e.g. `assert_eq(sum_one(c), 11)`)

### Tests & examples

- `examples/trait_bounds.ny`, `tests/nyra/trait_bounds_test.ny`, `CONF-TRAIT-BOUND-*`

## v1.2.0 (2026-06-24)

**All Extended preview → Stable Extended** — no W001 for async, traits, macros, lifetimes, defer, serde.

### Language

- **Macros** — multi-param parse; expansion in `if`/`while`/`for`/`spawn` bodies and `impl` methods
- **`defer`** — runs LIFO on block fall-through and **`return`**
- **Stability** — `extended_tier_warnings` returns empty (v1.2)

### Async

- **`runtime_poll_io(ms)`** — IO executor tick; `Executor_poll_ms` in `async_v1.ny`

### Stdlib / serde

- **`json_get_bool`**, **`json_get_object`**, nested value encoding in `json_encode_object`
- **`decode_bool`**, **`decode_object`** in `json/mod.ny`

### Tests

- `tests/nyra/macro_expand_test.ny`, `defer_return_test.ny`, `json_nested_test.ny`

## v1.1.0 (2026-06-24)

**Stable Extended + Windows releases** — promote shipped MVP features; platform and stdlib gaps closed.

### Stability

- **Stable Extended tier** — `?`, enum payloads, `spawn`, `impl Drop`, channels ship **without `warning[W001]`**
- Extended preview (async, traits, macros, lifetimes, defer) still emits W001
- [`docs/stability-v1.md`](docs/stability-v1.md) and [`docs/status.md`](docs/status.md) — canonical status matrix

### Releases

- **Windows prebuilt** — `nyra-x86_64-windows.zip` on every tag; [`scripts/install.ps1`](scripts/install.ps1)
- Release archives include **full stdlib tree** under `share/stdlib/`
- `SHA256SUMS` attached to GitHub Releases

### Stdlib / runtime

- **`env_set`** — POSIX + Windows (`rt_os_setenv`)
- **`process` / `Command`** on Windows (`CreateProcess`)
- **`db/postgres`**, **`db/mysql`** — native libpq/mysqlclient when headers/libs linked at build time

### Examples & tests

- `examples/env_set_smoke.ny`, `tests/nyra/env_set_test.ny`

## Unreleased

### Language & compiler

- **`dyn Trait`** — trait object dynamic dispatch (vtable + box); static `impl Trait for Type`; tests: `tests/nyra/trait_dispatch_test.ny`, `CONF-TRAIT-*`
- **Block comments** `/* … */` — lexer + `CONF-COMMENT-*` + `tests/nyra/block_comments_test.ny`
- **`nyra pkg prune`** — auto-fix **W002** (unused import) and **W003** (unused variable); `--check` dry run

### Windows cross-compile

- **spawn**, **TCP/HTTP** (Winsock2), **async/await** enabled for `*-pc-windows-gnu` (Win32 threads/sync + `select` I/O)
- Stale installed stdlib auto-fallback to repo `stdlib/rt/` when cross-compiling to Windows

## v1.0.0 (2026-06-06)

**Nyra 1.0** — Core tier semver-stable; Extended tier experimental with compiler warnings.

### Stability policy

- [`docs/stability-v1.md`](docs/stability-v1.md) — Core vs Extended contract, SemVer rules for 1.x
- **`warning[W001]`** when using Extended features (`async`, traits, macros, `spawn`, `defer`, explicit lifetimes, generics)
- **`nyra check|build|run --deny-extended`** — reject Extended-tier code (Core-only CI)
- Updated [`docs/status.md`](docs/status.md) — Core marked **Stable** in v1.0

### Cross-compilation (from unreleased)

- **`nyra build --for windows|linux|macos|wasm`** — easy cross-compile alias; also **`--os`**, **`--arch`**, **`--target TRIPLE`**
- Cross artifacts under **`target/{triple}/{debug|release}/`** with target-correct extensions (`.exe` on Windows)
- **`clang -target`** wired for all foreign triples; link flags derived from target OS (not host)
- **`nyra run`** / **`nyra test`** reject cross targets; **`--native-cpu`** rejected when cross-compiling
- Windows cross: print/fs I/O, spawn (Win32 threads), TCP/HTTP (Winsock2), async/await (pthread + select on MinGW target)
- Docs: [`webDocs/targets.html`](webDocs/targets.html), CI smoke: [`scripts/cross-smoke.sh`](scripts/cross-smoke.sh)

## v0.5.0 (2026-06-05)

Unsafe memory, typed raw pointers, and freestanding builds for systems programming.

### Unsafe & raw memory

- **`unsafe { }`** blocks — raw deref, pointer stores (`*p = v`), pointer arithmetic, raw casts
- Typed raw pointers **`\*T`** (distinct from opaque FFI **`ptr`**)
- **`expr as Type`** casts (including `&x as *i32`, `ptr as i32`)
- Borrow checker bypass inside `unsafe` (safe rules apply outside)
- **`\*T`** is `!Send` / `!Sync`; opaque `ptr` handles remain `Send`

### `no_std` / freestanding

- Top-level **`no_std`** directive (or CLI **`--no-std`**) — skips automatic `nyra_rt` linking
- **`nyra build --freestanding`** — `-ffreestanding -nostdlib` for bare-metal / kernel-style images
- `print` / `spawn` rejected in `no_std` programs

### Stdlib & runtime

- [`stdlib/core/mem.ny`](stdlib/core/mem.ny) — `malloc`, `free`, `memcpy`, `memset`, volatile MMIO helpers
- [`stdlib/rt/rt_volatile.c`](stdlib/rt/rt_volatile.c) — experimental `nyra_volatile_*` symbols in ABI manifest
- Example: [`examples/unsafe/raw_memory/main.ny`](examples/unsafe/raw_memory/main.ny)
- Docs: [`docs/unsafe-memory.md`](docs/unsafe-memory.md)

### OS stdlib & inline asm

- **`stdlib/os.ny`** — platform, battery, `os_getenv`, POSIX/syscall wrappers (`rt_os.c`, `rt_syscall.c`)
- **`asm "template"`** — LLVM inline assembly inside `unsafe`
- **`libraries/os/`** — usage guide; examples under `examples/os/`
- macOS battery via IOKit (auto-linked); Linux sysfs; Windows `GetSystemPowerStatus`

### Tooling & tests

- Integration tests: `compiler/driver/tests/unsafe_memory.rs`, `os_asm.rs`
- Parser fix: `expr as *T` on one line no longer absorbs `*p = …` on the next line
- Dev fallback when installed `~/.nyra` stdlib is stale (`runtime_map.rs`)

## v0.4.0 (2026-06-05)

### FFI boundary expansion (shipped in 0.4.x)

- Allow **enum tags**, **`[T; N]`**, and **tuples** at `export fn` / `extern fn` boundaries
- Allow **`export async fn`** (returns `i32` promise handle; payload must be `i32`)
- Allow **generic `export fn`** with **`export inst name<T>`** for explicit monomorph exports (`id__i32` mangling)
- Generic templates are not codegen'd; only monomorph instances are linked

### ABI (stable)

Stable ABI freeze and contributor tooling for FFI.

### ABI (stable)

- [`docs/abi-manifest.toml`](docs/abi-manifest.toml) — single source of truth for `nyra_rt` symbols
- Generated [`stdlib/nyra_rt.h`](stdlib/nyra_rt.h) via `scripts/gen-abi-header.py`
- SemVer policy in [`docs/abi-policy.md`](docs/abi-policy.md) (stable since v0.4.0)
- Typechecker rejects non-ABI types on `export fn` / `extern fn` boundaries
- Parser fix: top-level `export fn` / `export async fn` recognized
- Tests: `abi_manifest.rs`, expanded `ffi_export.rs`, `scripts/abi-roundtrip.sh` in CI (macOS + Linux)

## v0.3.0 (2026-06-05)

FFI production path, Nyra-native HTTP/TCP stdlib, and NyraPkg native linking.

### FFI

- Types: `ptr`, `i64`, `u32`; struct attribute `repr(C)`
- CLI: `--link-lib`, `--link-search-path`, `--link-arg`; `nyra.mod` `link` / `link-arg` lines
- `--cdylib` emits `.dylib`/`.so`; auto-links `nyra_free` for host callers
- Examples: `examples/ffi/call_libc/`, `export_greet/` (Rust + Python), improved `hello_from_rust/`
- [`docs/abi-policy.md`](docs/abi-policy.md) outbound/inbound ownership rules

### Stdlib & networking

- `stdlib/net/` — syscall layer + `TcpListener` / `TcpStream` in Nyra
- `stdlib/http/` — pure Nyra HTTP/1.1 GET client + one-shot server; [`docs/rfc/http-v1.md`](docs/rfc/http-v1.md)
- String helpers: `nyra_char_at`, `nyra_substring`, `nyra_strstr_pos`

### Tooling & docs

- [`agents/skill.md`](agents/skill.md) — language update + webDocs sync workflow
- Integration guides: native bindings, mini-http, Tauri sidecar
- Cargo-style `target/` layout (from prior work on branch)

## v0.2.0 (2026-06-04)

Stability and deferred features release (enum remains tag-only; no ADT payloads).

- Spec 1.0 frozen; RFC policy in CONTRIBUTING
- Const scalar evaluation, slice types with bounds checks, generic monomorphization, `match` guards
- Borrow: NLL basics, lifetime elision/`'a`, `Send`/`Sync`, safer `spawn`; `nyra check`/`diag` run borrow
- Owned `string`, `defer`, improved struct call ABI; modular stdlib layout
- Performance CI on Linux; documented release profile
- Traits, operator overloading, declarative macros, `async`/`await` + scheduler
- `nyra build --target wasm32-wasi` with WASI runtime subset
- `nyra lsp`, stable error codes, ABI symbol tests, test attributes
- NyraPkg semver resolver, `nyra pkg verify`/`publish`/`login`, local registry service

## v0.2.0 (historical)

- Multi-file projects, `import`, `struct`, field access, `for ..` loops
- Calculator example under `examples/projects/calculator/`

## v0.3.0

- Stdlib runtime (`stdlib/nyra_rt.c`), `extern fn` declarations
- `nyra test`, `nyra fmt`, `nyra build --release`, richer error display
