<p align="center">
  <img src="assets/Nyrabgremoved.png" alt="Nyra — geometric black panther wordmark" width="480">
</p>

<h1 align="center">Nyra</h1>

<p align="center">
  <strong>Go’s simplicity. Rust’s safety. C++’s speed. One language — Nyra.</strong>
</p>

<p align="center">
  <em>Fast like a panther. Sharp by design.</em>
</p>

> 
## What is Nyra?

**Nyra** is a programming language in active development. The name evokes agility and focus—the same qualities the language aims to deliver: code that reads cleanly, runs efficiently, and stays easy to reason about as projects grow.

The Nyra identity centers on a geometric black panther: faceted, deliberate, and forward-looking. That mascot is more than branding—it reflects how we think about the language itself: modern surfaces, strong fundamentals underneath, and motion without noise.

**Source files** use the `.ny` extension (for example, `main.ny`, `server.ny`).

## Why Nyra?

Most languages make you pick one strength and live with the rest. **Nyra** is being designed to combine the best traits from proven systems—without copying any single language wholesale.

### Design targets

Each pillar below names what Nyra is aiming for and which ecosystem sets the bar.

| Goal | Target | What we borrow |
|------|--------|----------------|
| **Ease of writing** | **Go** | Small keyword set, flat learning curve, readable control flow, and idioms that stay obvious in large codebases |
| **Memory safety** | **Rust** | Ownership and borrowing (or equivalent guarantees) so data races and use-after-free are caught at compile time, not in production |
| **Execution speed** | **C++** | Zero-cost abstractions, predictable layout, and LLVM-backed codegen tuned for native performance |
| **Tooling** | Developer experience | **Go** | Fast builds, built-in formatting, simple module layout, and a cohesive `nyra` CLI for fmt, test, doc, and build |
| **Concurrency** | Parallel programs | **Go** | Lightweight tasks, clear communication primitives, and a runtime model that makes concurrent code the default path—not a special case |
| **Compilation** | Backend | **LLVM** | A single industrial-strength IR pipeline: optimization passes, multi-arch targets, and a path to platform-specific tuning without rewriting the compiler |

### How the pieces fit together

```
  Source (.ny)
       │
       ▼
  Nyra compiler  ──►  LLVM IR  ──►  native binaries
       │
       ├── Go-like syntax & modules (ease of writing, tooling)
       ├── Rust-like safety checks (memory safety)
       ├── C++-class performance goals (execution speed)
       └── Go-style concurrency model
```

Nyra does not try to be Go, Rust, or C++ with a new logo. It **learns from** each: write like Go, protect memory like Rust, run fast like C++, ship tools like Go, scale concurrency like Go, and compile through **LLVM** so performance and portability stay on one foundation.

## Design goals

These principles follow from the table above and guide every language and toolchain decision:

- **Readable core** — Go-level simplicity on the **Core** surface ([`docs/status.md`](docs/status.md)); **semver-stable in v1.0** ([`docs/stability-v1.md`](docs/stability-v1.md))
- **Safe by default** — Ownership + borrow checker + Send/Sync for `spawn` captures (compile-time; no runtime race detector yet)
- **Types optional, proof mandatory** — `let x = expr` and `fn f() { ... }` infer types; auto-borrow at calls (`save(user)` → `save(&user)` when the callee takes `&T`); explicit annotations only when inference fails ([RFC 0006](docs/rfcs/0006-ownership-ux-and-inference.md))
- **Fast where it counts** — Release builds use LLVM `opt` + clang `-O3` + LTO; Nyra codegen keeps improving (immutable scalar SSA today)
- **Batteries-included tooling** — One `nyra` command for format, test, build, check, pkg, and LSP diagnostics
- **Strong batteries-included stdlib** — Collections, FS, HTTP/TCP, crypto, databases, serialization, WebSocket, and compression ship **in-tree** with the compiler ([`stdlib/README.md`](stdlib/README.md)); NyraPkg adds community and optional extensions
- **Extended tier** — `async`, traits, macros, `spawn`, etc. compile with **`warning[W001]`**; use **`--deny-extended`** for Core-only CI
- **LLVM as the compilation backbone** — One backend for debug, release, and cross-compilation

## Core vs Extended

Learn and ship with **Core** first; treat **Extended** as preview until RFC + stable semantics.

| Tier | Examples | Status |
|------|----------|--------|
| **Core** | `let`, `struct`, enum tags, `match`, `import`, `for`, `impl Type { }`, `nyra run/build/check` | **Stable in v1.0** — see [`docs/stability-v1.md`](docs/stability-v1.md) |
| **Extended preview** | traits, macros, `async`/`await`, `'a`, **`defer`**, explicit lifetimes | Experimental — **`warning[W001]`**; use **`allow_extended`** or `--deny-extended` for CI |
| **Stable Extended** | `?`, enum payloads, `spawn { }`, `impl Drop`, channels | **Stable (1.1+)** — no W001 |

Full lists: [`docs/spec-v1.md`](docs/spec-v1.md) · [`docs/status.md`](docs/status.md)

## Memory model

Nyra has **no garbage collector**. Heap values (`string`, structs with heap fields) use **move-by-default** ownership; the compiler emits `nyra_free` / `Drop_*_drop` via a static **DropPlan** ([`docs/conformance/ownership.md`](docs/conformance/ownership.md)).

**Write like JavaScript, check like Rust:**

```ny
struct User { name: string age: i32 }

fn save(user: &User) -> void { print(user.name) }

fn main() {
    let user = User { name: "Ahmed" age: 25 }
    save(user)          // auto-borrow: save(&user)
    print(user.name)    // still valid — no move
}
```

If `save` took owned `User`, the call would **move** and a later `print(user.name)` is a compile error with hints to use `&User` or `.clone()`.

| Mechanism | When |
|-----------|------|
| **Auto-borrow** | Pass owned binding to `&T` / `&mut T` parameter ([CONF-COERCE](docs/conformance/coercion.md)) |
| **Clone** | `string.clone()` and synthesized `Clone` for structs with cloneable fields |
| **Inference** | `let`, return types, generic call sites `id(x)` ([CONF-INF](docs/conformance/inference.md)) |
| **Struct sugar** | `User("Ada")` / `Point(1, 2)` desugar to struct literals |

Examples: [`examples/ownership_basics.ny`](examples/ownership_basics.ny) · [`examples/inference_generics.ny`](examples/inference_generics.ny)

## Project status

> **Beta.** Core + Stable Extended are ready for application development. Extended preview (async, traits) remains experimental. See [`docs/status.md`](docs/status.md).

**Canonical matrix:** [`docs/status.md`](docs/status.md)

| Milestone | Status |
|-----------|--------|
| **v1.0.0** | **Core tier semver-stable**; Extended warnings + `--deny-extended`; cross-compile shipped |
| **v0.5.0** | `unsafe`, `*T`, `no_std`, `asm`; `stdlib/core/mem.ny` + `stdlib/os.ny` (syscalls, battery) |
| **v0.4.0** | Stable C ABI; expanded FFI boundary types |
| **v0.3.0** | FFI (`ptr`, `repr(C)`, `--link-lib`, `--cdylib`); Nyra HTTP/TCP stdlib; NyraPkg `link` lines |
| **v0.2.0** | Spec 1.0 frozen; borrow/NLL; modular C runtime; LLVM release profile |
| **Language core depth** | **Core stable** — enum tags only; full ADT post-1.0 |
| **Concurrency** | **Stable Extended** — spawn + channels; async preview |
| **Tooling** | CLI **Done**; LSP diagnostics **MVP** |
| **Windows releases** | **Prebuilt** — `nyra-x86_64-windows.zip` + `install.ps1` |
| **Stability policy** | [`docs/stability-v1.md`](docs/stability-v1.md) |

**Toolchain today:**

- Lexer → Parser → Type checker → Borrow pass → **LLVM IR** → `clang` (+ `stdlib/nyra_rt.c`)
- CLI: `nyra run`, `build`, `check`, `test`, `fmt`, `pkg` (init/add/build)
- Examples: [`examples/`](examples/), calculator [`examples/projects/calculator/`](examples/projects/calculator/)
- **Web docs:** [`webDocs/`](webDocs/) (static HTML — open `webDocs/index.html`)
- **Arabic guide (دليل عربي):** [`docs/ar/README.md`](docs/ar/README.md) — full beginner-to-contributor walkthrough of the language, compiler, and repo
- Spec: [`docs/spec-v1.md`](docs/spec-v1.md) · **Status:** [`docs/status.md`](docs/status.md) · **Roadmap:** [`docs/roadmap-stable.md`](docs/roadmap-stable.md)
- **Syntax highlighting (TextMate):** [`grammar/nyra.tmLanguage.json`](grammar/nyra.tmLanguage.json) · [raw GitHub](https://raw.githubusercontent.com/hamdymohamedak/Nyra/main/grammar/nyra.tmLanguage.json) · [setup](grammar/README.md)
- NyraPkg: [`docs/nyrapkg-v1.md`](docs/nyrapkg-v1.md)
- Optional integration notes (not language core): [`docs/integration-ideas/`](docs/integration-ideas/)

Contributions welcome — see [CONTRIBUTING.md](CONTRIBUTING.md).

## Identity at a glance

```
Language:   Nyra
Mascot:     Black Panther
Philosophy: Fast • Safe • Minimal
```

**CLI commands** (from a project root, path defaults to `.`):

```bash
nyra run              # compile + run → target/debug/main
nyra build            # debug binary
nyra build --release  # optimized → target/release/main
nyra test
```

**Release / speed** (recommended v0.3.0 profile):

```bash
nyra build --release path/to/main.ny
# equivalent: nyra build --opt 3 --lto path/to/main.ny
```

See [`docs/performance.md`](docs/performance.md) for `--opt`, `--lto`, PGO, and `--target wasm32-wasi`.

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

## Quick start

### Install (users)

**Requirements:** **clang** (Xcode CLT on macOS, `clang` on Linux).

```bash
curl -fsSL https://raw.githubusercontent.com/hamdymohamedak/Nyra/main/scripts/install.sh | sh
source ~/.zshrc   # or open a new terminal
nyra --version
mkdir myapp && cd myapp && nyra pkg init
```

See [`install.md`](install.md) (macOS, Linux, Windows, release tags, first project) and [`docs/install.md`](docs/install.md) for a short summary.

### Hack on the compiler (contributors)

**Requirements:** [Rust](https://rustup.rs/) (stable), **clang**.

```bash
git clone https://github.com/hamdymohamedak/Nyra.git
cd Nyra
cargo build --release

# Hello world
cargo run -- run examples/syntax/hello.ny

# MVP demo: arithmetic + print → 30
cargo run -- run examples/syntax/math.ny

# Type-check only
cargo run -- check examples/syntax/math.ny
```

Optional global install:

```bash
cargo install --path cli
nyra run examples/syntax/math.ny
```

### Example (`examples/math.ny`)

```ny
fn main() {
    let x = 10
    let y = 20
    print(x + y)
}
```

Output: `30`

## Project structure

```
Nyra/
├── compiler/               # Compiler pipeline (workspace crates)
│   ├── driver/             # Public API + compile pipeline
│   ├── errors/, ast/, lexer/, parser/, resolve/, expand/
│   ├── types/, typecheck/, ownership/, borrowck/
│   ├── const_eval/, monomorph/, codegen/
│   └── driver/tests/       # integration, v0_2_features, abi_symbols
│
├── cli/                    # `nyra` binary (run, build, check, test, fmt, pkg, lsp)
├── lsp/                    # Language server library (`nyra lsp`)
├── rt/                     # Rust runtime (spawn, async MVP)
├── pkg/                    # nyra.mod / lock / sum / semver
├── pkg-registry/           # Local package registry (dev, port 9470)
│
├── stdlib/                 # C runtime + Nyra wrappers
│   ├── nyra_rt.c           # Native runtime (channels, alloc, spawn)
│   ├── nyra_rt_wasi.c      # Wasm subset
│   ├── alloc.ny, strings.ny, fs.ny, io.ny
│
├── grammar/                # TextMate grammar (VS Code / Cursor)
│   ├── nyra.tmLanguage.json
│   └── README.md
│
├── App/                    # Reference multi-file applications
│   └── NyraApp/            # Dungeon Steps demo (imports, enum, match, tests)
│       ├── main.ny
│       ├── nyra.mod
│       └── src/            # config, types, world, engine
│
├── examples/               # Samples and cross-language benchmarks
│   ├── syntax/             # hello.ny, math.ny (minimal smoke)
│   ├── language_features/  # enum + match demo
│   ├── v0_2/               # macro, trait sample
│   ├── projects/           # calculator, http_hello, read_file
│   ├── ffi/                # extern fn + Rust cdylib
│   └── comparison/         # Nyra vs Go/Rust/JS/Python/Java
│       ├── hello/, arithmetic/, loop/, dungeon/
│       └── results/        # output from make bench
│
├── docs/                   # Spec, roadmap, tooling, ABI, performance
│   ├── spec-v1.md          # Frozen language spec
│   ├── roadmap-stable.md
│   └── integration-ideas/  # wasm, mini-http, … (non-normative)
│
├── webDocs/                # Static HTML documentation site
├── skills/                 # Contributor design notes (informal)
├── Makefile                # make test-all, make help, make bench, …
├── make/                   # Make targets, lib recipes, py generators
├── scripts/                # install.sh (curl), install.ps1
├── benchmarks/               # CI performance baselines
├── assets/                 # Logo and branding
├── .github/workflows/      # CI (build, test, perf macOS + Linux)
│
├── CONTRIBUTING.md         # How to contribute (detailed)
├── install.md              # User install guide
├── CHANGELOG.md
└── Cargo.toml              # Rust workspace (v0.3.0)
```

For folder-by-folder contributor notes, see [CONTRIBUTING.md](CONTRIBUTING.md).

## Name and branding

**Nyra** (pronounced *NYE-rah* or *NEE-rah*—pick what feels natural to your team) pairs a bold wordmark with a low-poly panther: sharp angles, dark tones, and a single clear gaze forward. Use `assets/nyra-logo.png` when referencing the project in docs, slides, or community pages.

## License

**Proprietary** — All Rights Reserved. See [LICENSE](LICENSE).

The Software is not open source. Use, copying, modification, and distribution
require permission from the copyright holder unless otherwise agreed in writing.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

<p align="center">
  <sub>Nyra — ease of writing · memory safety · execution speed</sub>
</p>
