<p align="center">
  <img src="assets/Nyrabgremoved.png" alt="Nyra logo" width="480">
</p>

<h1 align="center">Nyra</h1>

<p align="center">
  <strong>Go’s simplicity. Rust’s safety. C++’s speed.</strong>
</p>

## What is Nyra?

**Nyra** is a compiled language (`.ny`) with optional types, ownership and borrowing, LLVM native codegen, and one `nyra` CLI for run, build, test, fmt, and pkg. Write zero-types or explicit annotations — both are first-class.

## Highlights

- **No GC** — move semantics, borrow checker, `impl Drop`
- **Zero-types by default** — inference when possible; clear errors when not
- **Stable toolchain** — Core + Stable Extended (async, traits, spawn, macros, JSON serde)
- **Batteries included** — stdlib, LSP, cross-platform releases (Linux, macOS, Windows)

Details: [`docs/status.md`](docs/status.md) · [`docs/stability-v1.md`](docs/stability-v1.md)

## Syntax at a glance

```ny
fn greet(name) {
    return strcat("Hello, ", name)
}

let nums = [10, 20, 30]
let total = 0
for n in nums { total = total + n }
print(total)   // 60
```

**Ownership, no GC** — compiler tracks moves and borrows; `defer` for cleanup:

```ny
allow_extended

fn main() {
    defer print("cleanup")
}
```

**Optionals and errors** — `??`, `?.`, and `?` propagation:

```ny
import "stdlib/option.ny"

let y = x ?? 42
let m = opt?.method()

fn step(n) -> Result<i32, i32> {
    if n == 0 { return Result.Err(1) }
    return Result.Ok(n)
}

fn main() -> Result<i32, i32> {
    return step(step(1)? + 1)?
}
```

**Strings and loops:**

```ny
let line = "  hello,nyra,world  "
for part in line.trim().split(",") { print(part) }
print(`Hello ${name}`)
```

Demos ship as `foo.ny` (zero-types) and `foo.typed.ny` (explicit types) in [`examples/`](examples/).

## Quick start

**Requires:** clang (Xcode CLT on macOS, `clang` on Linux).

```bash
curl -fsSL https://raw.githubusercontent.com/nyra-lang/nyra/main/scripts/install.sh | sh
nyra --version
mkdir myapp && cd myapp && nyra pkg init
```

Windows: `scripts/install.ps1` from [GitHub Releases](https://github.com/nyra-lang/nyra/releases).

**Build from source** — [Rust](https://rustup.rs/) (stable) + clang:

```bash
git clone git@github.com:nyra-lang/nyra.git
cd Nyra
cargo build --release
cargo run -- run examples/syntax/hello.ny
```

Optional: `cargo install --path cli` then `nyra run examples/syntax/math.ny`.

## Documentation

| Resource | Link |
|----------|------|
| Status · Roadmap · Architecture | [`docs/status.md`](docs/status.md) · [`docs/roadmap-stable.md`](docs/roadmap-stable.md) · [`docs/architecture.md`](docs/architecture.md) |
| Contributor map | [`docs/contributor-map.md`](docs/contributor-map.md) |
| Stdlib · Contributing | [`stdlib/README.md`](stdlib/README.md) · [`CONTRIBUTING.md`](CONTRIBUTING.md) |

**Examples:** [`examples/`](examples/) · calculator [`examples/projects/calculator/`](examples/projects/calculator/) · syntax highlighting [`grammar/`](grammar/)

## Project layout

```
Nyra/
├── compiler/     # lexer → parser → typecheck → borrowck → LLVM IR
├── cli/          # nyra binary
├── lsp/ dap/     # language server + debug adapter
├── stdlib/       # Nyra stdlib + C runtime
├── tests/        # feature tests, conformance, suite
├── examples/     # demos and benchmarks
├── Apps/         # reference applications
└── Makefile      # make test-all, make bench, make help
```

## Contributing

Start with [`docs/contributor-map.md`](docs/contributor-map.md) — syntax → `compiler/`, stdlib → `stdlib/`, CLI → `cli/`, tests → `tests/nyra/`, demos → `examples/`. Quick scaffold: `make contribute`.

<!-- BENCH:START -->

## Performance benchmarks

Nyra is compared against **C, C++, Go, and Rust** on the same
programs under [`examples/comparison/`](examples/comparison/). **Lower runtime and RAM are better.**
Compile time is excluded; numbers are mean wall-clock over timed runs.

**Last run:** 2026-07-01T14:51:55Z · **Platform:** Darwin arm64 · **Runs:** 5 (warmup 1 discarded); micro=9 concurrency=11 · **Nyra:** release; cpu_bound_pgo=release+pgo; Nyra flags: --no-prelude (single-file suites), prelude (multi-file/struct_sum), -march=native

**[Interactive report →](examples/comparison/results/latest.html)** ·
raw data: [`data.tsv`](examples/comparison/results/data.tsv)

| Language | CPU hot loop | Nested loops | Linear sum | Hello I/O |
|----------|----------:|----------:|----------:|----------:|
| Nyra (Zero Types) | 825 ms | 53.9 ms | 1,507 ms | 2.8 ms |
| Nyra (Explicit Types) | 846 ms | 53.7 ms | 1,435 ms | 2.7 ms |
| C | 461 ms | 58.4 ms | 1,303 ms | 3.5 ms |
| C++ | 459 ms | 57.9 ms | 1,296 ms | 3.6 ms |
| Go | 570 ms | 59.6 ms | 1,321 ms | 4.7 ms |
| Rust | 861 ms | 69.2 ms | 1,544 ms | 4.0 ms |

**cpu_bound snapshot:** Nyra (Zero Types) `825 ms` vs fastest compiled (C++ `459 ms`) — **1.80×** wall time.

**Peak RAM (cpu_bound):**

| Language | Peak RSS |
|----------|----------:|
| Nyra (Zero Types) | 1.0 MB |
| Nyra (Explicit Types) | 1.1 MB |
| C++ | 1.1 MB |
| C | 1.1 MB |
| Rust | 1.2 MB |
| Go | 3.6 MB |

```bash
make bench              # full matrix + HTML report
BENCH_QUICK=1 make bench  # CI-friendly subset
```

<!-- BENCH:END -->

## License

**Proprietary** — All Rights Reserved. See [LICENSE.md](LICENSE.md).
