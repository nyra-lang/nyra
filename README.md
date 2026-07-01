<p align="center">
  <img src="assets/Nyrabgremoved.png" alt="Nyra logo" width="480">
</p>

<h1 align="center">Nyra</h1>

<p align="center">
  <strong>Go’s simplicity. Rust’s safety. C++’s speed.</strong>
</p>

## What is Nyra?

**Nyra** is a compiled programming language (`.ny` source files) with optional types, ownership and borrowing, LLVM-backed native codegen, and a single `nyra` CLI for run, build, test, fmt, and pkg.

Write with zero types or explicit annotations — both styles are first-class.

## Highlights

- **No GC** — move semantics, borrow checker, `impl Drop`
- **Zero-types by default** — inference when possible; clear errors when not
- **Stable toolchain** — Core + Stable Extended (async, traits, spawn, macros, JSON serde)
- **Batteries included** — stdlib, LSP, cross-platform releases (Linux, macOS, Windows)

Details: [`docs/status.md`](docs/status.md) · [`docs/stability-v1.md`](docs/stability-v1.md)

## Quick start

### Install

**Requires:** clang (Xcode CLT on macOS, `clang` on Linux).

```bash
curl -fsSL https://raw.githubusercontent.com/nyra-lang/nyra/main/scripts/install.sh | sh
nyra --version
mkdir myapp && cd myapp && nyra pkg init
```

Windows: use `scripts/install.ps1` from [GitHub Releases](https://github.com/nyra-lang/nyra/releases).

### Build from source

**Requires:** [Rust](https://rustup.rs/) (stable), clang.

```bash
git clone git@github.com:nyra-lang/nyra.git
cd Nyra
cargo build --release
cargo run -- run examples/syntax/hello.ny
cargo run -- run examples/syntax/math.ny    # prints 30
```

Optional: `cargo install --path cli` then `nyra run examples/syntax/math.ny`.

## Documentation

| Resource | Link |
|----------|------|
| **Project status** | [`docs/status.md`](docs/status.md) |
| **Roadmap** | [`docs/roadmap-stable.md`](docs/roadmap-stable.md) |
| **Architecture** | [`docs/architecture.md`](docs/architecture.md) |
| **Contributor map** | [`docs/contributor-map.md`](docs/contributor-map.md) — what to change → where to go |
| **Stdlib layout** | [`stdlib/README.md`](stdlib/README.md) |
| **Contributing** | [`CONTRIBUTING.md`](CONTRIBUTING.md) |

**Examples:** [`examples/`](examples/) (small demos) · [`Apps/`](Apps/) (full reference apps) · calculator [`examples/projects/calculator/`](examples/projects/calculator/)

**Syntax highlighting:** [`grammar/nyra.tmLanguage.json`](grammar/nyra.tmLanguage.json) · [setup](grammar/README.md)

## Project layout

```
Nyra/
├── compiler/     # lexer → parser → expand → typecheck → borrowck → LLVM IR
├── cli/          # nyra binary
├── lsp/ dap/     # language server + debug adapter
├── stdlib/       # Nyra stdlib + C runtime
├── tests/        # nyra/ (feature tests), conformance/, suite/
├── examples/     # small demos, builtins, cross-language benchmarks
├── Apps/         # full reference applications (games, IDE, databases, …)
├── docs/         # architecture, contributor map, status, roadmap
└── Makefile      # make test-all, make bench, make help
```

## Contributing — what to change → where to go

New contributor? Start with [`docs/contributor-map.md`](docs/contributor-map.md) for the full guide. Quick map:

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
   Comptime eval        Remove / deprecate
   const_eval/          reverse paths above;
   (comptime.rs)        delete tests, examples,
   + parser/            docs, grammar entries
   + typecheck/
```

| Task | Primary location |
|------|------------------|
| New syntax | `compiler/lexer/` → `parser/` → `ast/` → `expand?` → `typecheck/` → `codegen/` |
| Stdlib API | `stdlib/` (+ `stdlib/rt/` + `runtime_map.rs` if C) |
| CLI | `cli/src/app/args.rs` · `cli/src/commands/` |
| Tests for a feature | `tests/nyra/<name>_test.ny` (+ `.typed.ny`) |
| Runnable demo | `examples/<topic>/` (`foo.ny` + `foo.typed.ny`) |

Full details, test placement rules, and `expand/` module index: [`docs/contributor-map.md`](docs/contributor-map.md) · [`CONTRIBUTING.md`](CONTRIBUTING.md).

<!-- BENCH:START -->

## Performance benchmarks

Nyra is compared against **C, C++, Go, Rust, Node, Python, and Java** on the same
programs under [`examples/comparison/`](examples/comparison/). **Lower runtime and RAM are better.**
Compile time is excluded; numbers are mean wall-clock over timed runs.

**Last run:** 2026-07-01T14:14:09Z · **Platform:** Darwin arm64 · **Runs:** 5 (warmup 1 discarded); micro=9 concurrency=11 · **Nyra:** release; cpu_bound_pgo=release+pgo; Nyra flags: --no-prelude (single-file suites), prelude (multi-file/struct_sum), -march=native

**[Interactive report →](examples/comparison/results/latest.html)** ·
raw data: [`data.tsv`](examples/comparison/results/data.tsv)

| Language | CPU hot loop | Nested loops | Linear sum | Hello I/O |
|----------|----------:|----------:|----------:|----------:|
| Nyra (Zero Types) | 809 ms | 52.2 ms | 1,414 ms | 1.8 ms |
| Nyra (Explicit Types) | 809 ms | 52.0 ms | 1,413 ms | 1.8 ms |
| C | 474 ms | 57.9 ms | 1,296 ms | 3.8 ms |
| C++ | 475 ms | 57.4 ms | 1,296 ms | 2.9 ms |
| Go | 582 ms | 58.9 ms | 1,298 ms | 4.6 ms |
| Rust | 878 ms | 71.3 ms | 1,552 ms | 3.7 ms |

**cpu_bound snapshot:** Nyra (Zero Types) `809 ms` vs fastest compiled (C `474 ms`) — **1.70×** wall time.

**Peak RAM (cpu_bound):**

| Language | Peak RSS |
|----------|----------:|
| Nyra (Zero Types) | 1.0 MB |
| Nyra (Explicit Types) | 1.0 MB |
| C | 1.0 MB |
| C++ | 1.1 MB |
| Rust | 1.2 MB |
| Go | 3.6 MB |

```bash
make bench              # full matrix + HTML report
BENCH_QUICK=1 make bench  # CI-friendly subset
```

<!-- BENCH:END -->

## License

**Proprietary** — All Rights Reserved. See [LICENSE.md](LICENSE.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and the contributor map: [docs/contributor-map.md](docs/contributor-map.md).
