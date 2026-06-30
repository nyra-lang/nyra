# Dev apps — Nyra toolchain smoke tests

Developer-tool reimplementations that expose gaps in **compiler introspection**, **process spawning**, **stdlib tooling APIs**, and **runtime observability** before building a full Nyra SDK.

Each directory is an independent **`nyra pkg init` project** (own `nyra.mod`, `main.ny`, `src/`). Shared helpers live in `shared/` (`cli.ny`, `walk.ny`). Apps use the **auto-prelude** stdlib and only import project-local + shared files.

## Projects

| App | What it does | Run | Stdlib / runtime used |
|-----|--------------|-----|------------------------|
| `Linter/` | Style rules + optional `check()` | `cd Linter && nyra run . src` · `nyra run . --check .` | `check`, file walk |
| `DocumentationGenerator/` | `///` + `fn`/`struct` scan → `API.md` | `cd DocumentationGenerator && nyra run . src API.md` | `write_file` |
| `TestRunner/` | Discover + run tests via `exec(nyra, …)` | `cd TestRunner && nyra run . .` | `exec`, `compiler_nyra_bin` |
| `BenchmarkTool/` | `bench_loop` + `benchmark { }` demo | `cd BenchmarkTool && nyra run . 3` | `bench_loop` |
| `Fuzzer/` | Mutate strings, stress tiny parser | `cd Fuzzer && nyra run . 50 abc` | `random_range` |
| `Profiler/` | CPU + RSS regions | `cd Profiler && nyra run . 2000` | `profile_start` |
| `MemoryLeakDetector/` | `alloc_track_*` + StrVec batches | `cd MemoryLeakDetector && nyra run . 2 100` | `alloc_track_*` |

Build all:

```bash
BASE="Apps/dev"
for d in Linter DocumentationGenerator TestRunner BenchmarkTool Fuzzer Profiler MemoryLeakDetector; do
  (cd "$BASE/$d" && nyra build .) || exit 1
done
```

## Resolved in v1.19.0

| Gap | Resolution |
|-----|------------|
| Compiler in-process FFI | `libnyra_compiler.dylib` + `check_inprocess` / `diag_json_inprocess` |
| raygui widgets | `stdlib/gui/raygui.ny` + `nyra pkg c add raygui` |

## Resolved in v1.18.0

| Gap | Resolution |
|-----|------------|
| Package install / verify | Use standalone [nyrapkg](https://github.com/nyra-lang/nyrapkg) (`init`, `add`, `install`, `verify`) |
| No ASan CLI flag | `nyra build --sanitize` (`-fsanitize=address`) |
| Limited `compiler.ny` | Added `build()`, `fmt()`, `run()` subprocess helpers |

## Remaining gaps

| Gap | Notes |
|-----|-------|
| No per-allocation stacks | `alloc_track_note` is manual RSS estimate only |
| No coverage-guided fuzzing | Fuzzer uses random mutation |
| Windows `exec` | Stub JSON error on Windows |

## Shared code pattern

`shared/walk.ny` recursively collects `.ny` files. `shared/cli.ny` wraps argv in `DevPathList` / `DevFileList` so `.get()` keeps a struct receiver after cross-file returns.
