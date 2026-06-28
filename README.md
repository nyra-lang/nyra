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
| **Stdlib layout** | [`stdlib/README.md`](stdlib/README.md) |
| **Contributing** | [`CONTRIBUTING.md`](CONTRIBUTING.md) |

**Examples:** [`examples/`](examples/) · [`Apps/`](Apps/) · calculator [`examples/projects/calculator/`](examples/projects/calculator/)

**Syntax highlighting:** [`grammar/nyra.tmLanguage.json`](grammar/nyra.tmLanguage.json) · [setup](grammar/README.md)

## Project layout

```
Nyra/
├── compiler/     # lexer → parser → typecheck → borrowck → LLVM IR
├── cli/          # nyra binary
├── lsp/ dap/     # language server + debug adapter
├── stdlib/       # Nyra stdlib + C runtime
├── examples/     # samples and cross-language benchmarks
├── Apps/         # reference applications
├── docs/         # status, roadmap, architecture
└── Makefile      # make test-all, make bench, make help
```

<!-- BENCH:START -->

## Performance benchmarks

Nyra is compared against **C, C++, Go, Rust, Node, Python, and Java** on the same
programs under [`examples/comparison/`](examples/comparison/). **Lower runtime and RAM are better.**
Compile time is excluded; numbers are mean wall-clock over timed runs.

**Last run:** 2026-06-23T21:27:07Z · **Platform:** Darwin arm64 · **Runs:** 5 (warmup 1 discarded) · **Nyra:** release; cpu_bound_pgo=release+pgo; Nyra flags: --no-prelude, -march=native (host release default)

**[Interactive report →](examples/comparison/results/latest.html)** ·
raw data: [`data.tsv`](examples/comparison/results/data.tsv)

| Language | CPU hot loop | Nested loops | Linear sum | Hello I/O |
|----------|----------:|----------:|----------:|----------:|
| Nyra (Zero Types) | 808 ms | 52.3 ms | 1,179 ms | 2.1 ms |
| Nyra (Explicit Types) | 809 ms | 52.5 ms | 1,182 ms | 2.2 ms |
| C | 463 ms | 57.8 ms | 1,299 ms | 2.3 ms |
| C++ | 468 ms | 57.7 ms | 1,295 ms | 2.2 ms |
| Go | 579 ms | 58.5 ms | 1,308 ms | 3.1 ms |
| Rust | 865 ms | 68.8 ms | 1,544 ms | 2.6 ms |

**cpu_bound snapshot:** Nyra (Zero Types) `808 ms` vs fastest compiled (C `463 ms`) — **1.75×** wall time.

**Peak RAM (cpu_bound):**

| Language | Peak RSS |
|----------|----------:|
| Nyra (Zero Types) | 1.0 MB |
| Nyra (Explicit Types) | 1.0 MB |
| C | 1.0 MB |
| C++ | 1.0 MB |
| Rust | 1.2 MB |
| Go | 3.5 MB |

```bash
make bench              # full matrix + HTML report
BENCH_QUICK=1 make bench  # CI-friendly subset
```

<!-- BENCH:END -->

## License

**Proprietary** — All Rights Reserved. See [LICENSE.md](LICENSE.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
