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

## Syntax examples

Nyra reads like a scripting language but compiles to native code — **no types required**, inference when possible, clear errors when not. Each topic below is a pain point in other languages; Nyra keeps the fix short.

### No type ceremony

**Java / Rust:** annotate almost every variable, parameter, and return type.

**Nyra (zero-types):**

```ny
fn greet(name) {
    return strcat("Hello, ", name)
}

let nums = [10, 20, 30]
let total = 0
for n in nums {
    total = total + n
}
print(total)   // 60
```

Want explicit types for a public API? Add them — same program, same binary. See `examples/syntax/math.ny` and `math.typed.ny`.

### Fast memory — no GC, no manual `free`

| Language | Trade-off |
|----------|-----------|
| **Go** | Easy syntax, but a garbage collector adds pauses and extra RAM |
| **C / C++** | Full control, but `malloc`/`free` bugs and use-after-free are your problem |
| **Nyra** | Ownership + borrow checker at compile time — no GC, safe by default |

```ny
allow_extended

fn main() {
    defer print("cleanup")   // runs on return — like Go's defer, no GC
    // compiler tracks moves and borrows; no manual free in normal code
}
```

### Null and optionals in one line

**Java / C# (before modern helpers):** nested `if (x != null)` checks. **C/C++:** null dereference is undefined behavior.

```ny
import "stdlib/option.ny"

let y = x ?? 42              // default when None
let m = opt?.method()        // skip call safely when None
```

### Errors without `if err != nil` everywhere

**Go:** repeat `if err != nil { return err }` after every fallible call.

**Nyra:** propagate with `?` and handle once where it matters:

```ny
fn step(n) -> Result<i32, i32> {
    if n == 0 {
        return Result.Err(1)
    }
    return Result.Ok(n)
}

fn main() -> Result<i32, i32> {
    let a = step(1)?
    let b = step(a + 1)?
    return step(b)?
}
```

Runnable: `examples/try_operator_generic.ny`.

### Strings and loops that read like scripts

**Verbose string APIs** in C/C++ and Java vs. **method chaining + `for-in`** in Nyra:

```ny
let name = "Ada"
print(`Hello ${name}`)                    // template strings

let line = "  hello,nyra,world  "
for part in line.trim().split(",") {
    print(part)
}
```

Runnable: `examples/syntax/template_strings.ny`, `examples/syntax/string_methods.ny`, `examples/syntax/for_in.ny`.

**Two styles, one language:** demos in [`examples/`](examples/) ship as `foo.ny` (zero-types) and `foo.typed.ny` (explicit types) — use whichever fits your project.

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
| **Contributor hub (`make contribute`)** | [`make/py/contrib_dev/README.md`](make/py/contrib_dev/README.md) |

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
| **Scaffold (quick start)** | `make contribute` — stdlib, tests, pkg, syntax checklist |
| New syntax | `compiler/lexer/` → `parser/` → `ast/` → `expand?` → `typecheck/` → `codegen/` |
| Stdlib API | `stdlib/` (+ `stdlib/rt/` + `runtime_map.rs` if C) |
| CLI | `cli/src/app/args.rs` · `cli/src/commands/` |
| Tests for a feature | `tests/nyra/<name>_test.ny` (+ `.typed.ny`) |
| Runnable demo | `examples/<topic>/` (`foo.ny` + `foo.typed.ny`) |

Full details, test placement rules, and `expand/` module index: [`docs/contributor-map.md`](docs/contributor-map.md) · [`CONTRIBUTING.md`](CONTRIBUTING.md).

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

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and the contributor map: [docs/contributor-map.md](docs/contributor-map.md).
