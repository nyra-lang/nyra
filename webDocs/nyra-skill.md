# Nyra Programming Language

> **Standalone reference** — attach this file alone in Cursor/ChatGPT/Claude; no repo checkout required.
> **Online docs (human-readable):** [nyra-lang.github.io/docs](https://nyra-lang.github.io/docs/) · [Learn intro](https://nyra-lang.github.io/docs/learn-intro.html)

Use this file as the **sole authoritative reference** for Nyra syntax, semantics, stdlib, toolchain, PGO, and escape analysis.
Do not invent features not listed here. Supplementary guides live at **https://nyra-lang.github.io/docs/**.

> **Project status — v1.40.x production-ready tier:** **Core** and **Stable Extended** (async, traits, macros, lifetimes, defer, serde, `?`, official `Error`, `spawn` / `spawn:task` / `spawn:thread`, `JoinHandle`, enum payloads, generic `random()`) ship **without W001**. Prebuilt Linux, macOS, and Windows releases. See [Stability](#stability-v10) · [roadmap](https://nyra-lang.github.io/docs/roadmap.html).

## Table of contents

1. [Identity & compiler pipeline](#identity)
2. [Design philosophy — easy syntax & optional types](#design-philosophy)
3. [Toolchain & CLI](#toolchain)
4. [Syntax conventions & variables](#syntax-conventions)
5. [Language reference — keywords, operators, statements](#language-reference)
6. [Types & functions](#types)
7. [Control flow, match, structs, enums & payloads, imports](#control-flow)
8. [Generics & monomorphization](#generics)
9. [Built-in API, methods & I/O](#io--builtins) — strings, arrays, math, Vec, HashMap, helpers
10. [Async & await](#async--await) · [Concurrency & sync](#concurrency--sync-primitives)
11. [Ownership & memory](#ownership-summary)
12. [Performance — monomorph, DCE, release, PGO, escape analysis](#performance--optimization)
13. [Stdlib, NyraPkg, FFI & C interop](#stdlib-modular)
14. [Macros](#macros-extended)
15. [Unsafe, OS, tests, layout](#unsafe--no_std-v050)
16. [Diagnostics](#diagnostics)
17. [Do NOT hallucinate](#do-not-hallucinate)

> **Enum payloads (read first):** Default enums are **tag-only** (`Color.Red`). **`Option.Some(42)` stores a real value** only when you `import "stdlib/option.ny"` (or define `enum Option_i32 { None, Some(i32) }`). Built-in `Option`/`Result` without import are tag **names** for `??`/`?.` — not storage. See [Enums & payloads](#enums--payloads).

> **Collections naming:** Core stdlib uses **monomorph names** (`Vec_i32_push`, `HashMap_str_i32`). **Generic syntax** (`Vec<T>`, `Arc<T>`, `Box<T>`) is Extended — current for smart pointers, but tutorials and docs use `Vec_i32` style. See [Naming: current vs legacy](#naming-current-style-vs-legacy-read-this).

## Identity

- **Nyra** — systems language: Go-like syntax, Rust-like ownership, LLVM backend.
- Source: `.ny` / `.nyra` files → lexer → parser → expand → monomorph (+ generic call inference) → auto-borrow coercion → typecheck → ownership (Copy inference) → borrow + lifetimes + Send/Sync → **escape analysis** → drop plan → LLVM IR → `opt` → clang + runtime C modules.
- CLI: `nyra` (Rust). Package manager: `nyra pkg` (NyraPkg).
- Version baseline: **v1.40.x** — **Core tier semver-stable**; **Stable Extended** shipped ([roadmap & status](https://nyra-lang.github.io/docs/roadmap.html)).
- **v1.2:** template strings, arrow functions, `net/http` handler dispatch, language bridge (Python/Node/Java workers), NyraPkg semver + registry, `link-source` auto-link, bindings reference, native C interop pattern.
- **v2.1:** stack closures (loop-safe), arrow param inference, tuple destructure in arrow params, `??` nullish coalescing, `?.` optional chaining.
- **v2.2:** heap closure promotion; `?.method()`; **`Option.Some(T)` payloads** when using `import "stdlib/option.ny"` (replaces tag-only built-in `Option` for that module).
- **v2.3:** composite struct field drop, auto-owned `extern fn -> string`, `Box_string` (superseded by `Box<string>` in v2.4), `OptionStr`.
- **v2.4:** generic `enum Option<T>` / `enum Result<T,E>` monomorph; enum payload drop; `struct Box<T>` + `Box_new(string)` (replaces `Box_string`).
- **v2.5:** generic `struct Arc<T>` (`Arc<i32>`, `Arc<string>`); auto Drop for monomorph instances; `Arc_i32` kept as legacy alias in `stdlib/arc.ny`.
- **v2.6:** async bootstrap patterns, HTTP health via stdlib `net/http`.
- **v2.7:** `nyra.mod` workspaces, `CONF-WS-*` conformance. See [Modules](https://nyra-lang.github.io/docs/modules.html). Tracing/service mesh = external.
- **v2.8:** return type inference (`void` default), generic call-site inference (`id(7)`), auto-borrow at calls (`T` → `&T`), `string.clone()` + synthesized struct `Clone`, struct ctor sugar `User("Ada")` / `Point()`.
- **v2.9:** Swift-style use-after-move diagnostics (`was moved into save()` + fix-it notes), `move` / `clone` prefix at call sites (`save(move user)`, `save(clone user)`).
- **v3.0:** auto Copy inference for all-Copy structs (`Point`, `Rect`); `#[derive(Copy)]` validation; no annotation needed for value types.
- **v3.1:** `f64` IEEE-754 double — float literals (`3.14`), mixed `i32`/`f64` promotion, LLVM `double` codegen.
- **v3.2:** `char` Unicode scalar — `'a'`, `'\n'`, `'\u{...}'`; LLVM `i32`; `print` via `%c`.
- **v1.3:** **CONF-LANG** Nyra-source conformance suite; `stdlib/testing.ny` assertion helpers.

## Design philosophy

**Ease of syntax is a primary design goal.** Nyra is meant to feel approachable first — like Go or modern scripting languages — while still compiling to LLVM with Rust-like ownership under the hood.

### Types are optional, not mandatory

- **Default style (zero-types):** like JavaScript, write scripts and apps **without** type annotations, structs, or enums unless you want them for organization. The compiler infers locals, parameters, return types, and many call sites.
- **Opt-in types:** add `: type`, `struct`, `enum`, or `-> return` when you want clarity, public APIs, FFI (`extern`), or when inference cannot disambiguate.
- **Zero runtime cost:** types exist only at compile time; inference does not slow down the binary.
- **Two valid styles, same language:** learn-track examples ship as **`foo.ny`** (no annotations) and **`foo.typed.ny`** (explicit). Docs default to **Without types**.

**Inference contract:** Nyra infers types from usage (operators, calls, methods, call sites). If it **cannot** infer a parameter, local, or return type, compilation **stops** with `E004` and tells you exactly what to annotate manually. Nyra never guesses `i32` for an unknown parameter — **manual types are only for rare ambiguities.**

Inference sources (combined across multi-pass signature registration):

- **Method calls** on parameters (e.g. `args.get(i)`, `args.len()` → `StrVec`)
- **Call sites** (e.g. `Cat_run(StrVec_from_argv(1))` → parameter `StrVec`)
- **Operators** (`a + b` → numeric; `strcat("x", name)` → `string`; `printed < count` → `i32`)
- **Return statements** (`return v` after `let v = StrVec_new()` → `StrVec`)
- **`extern fn`** signatures in the merged program (prelude)

**Multi-file projects:** in `main.ny`, import every module the app needs (e.g. `import "src/cli.ny"` **and** `import "src/cat.ny"`). Shared helpers like `cli.ny` should be imported from `main.ny`, not only from a sibling file — import shared modules from `main.ny`.

```ny
// Zero-types (preferred for learning, scripts, CLI tools)
let score = 10
fn add(a, b) {
    return a + b
}
fn greet(name) {
    print(strcat("Hello, ", name))
}

// Typed (optional — same ideas, explicit where you want)
let score: i32 = 10
fn add(a: i32, b: i32) -> i32 {
    return a + b
}
struct User {
    name: string
    age: i32
}   // only when you need a named shape
```

**For AI assistants:** when generating Nyra for beginners or app code, **omit types unless the user asks for them** or inference would fail. Do not “Rust-ify” Nyra by sprinkling `: i32` everywhere. Optional types: [stdlib guide](https://nyra-lang.github.io/docs/stdlib.html#optional-types) · [learn track](https://nyra-lang.github.io/docs/learn-get-started.html).

## Stability (v1.0)

Nyra has **two shipped tiers** (see [roadmap & stability](https://nyra-lang.github.io/docs/roadmap.html)):

| Tier | Status | Examples | CI flag |
|------|--------|----------|---------|
| **Core** | Semver-stable | types, control flow, `match`, tag-only enums, `impl`, ownership, FFI, `unsafe`/`no_std`, monomorph generics (`fn id<T>`, `Vec_i32`), optional annotations | `nyra check --deny-extended` |
| **Stable Extended** | Shipped in v1.39+ **without W001** in default builds | enum payloads, `async`/`await`, traits, `dyn`, macros, `defer`, explicit lifetimes, `spawn` / `spawn:task` / `spawn:thread`, `JoinHandle`, arrow fns, spread, compiler `random()` | default `nyra build` |

- **Legacy note:** older docs called Extended "experimental" with **`warning[W001]`**. Current production tier (v1.40.x) ships Extended features in prebuilt releases; use `--deny-extended` only for Core-only CI gates.
- **Core-stable generics:** `fn id<T>`, `Option<T>`, `Result<T,E>`, `Arc<T>`, `Box<T>` monomorph at compile time.

## Toolchain

From a project root (directory with `main.ny`), path arguments default to **`.`** — same idea as `cargo test` with no path.

```bash
nyra run                      # compile + run (target/debug/main)
nyra run .                    # same
nyra run main.ny              # single file only (no imports)
nyra build                    # debug binary → target/debug/main
nyra build --release          # release binary → target/release/main
nyra build . --release --for windows   # cross → target/x86_64-pc-windows-gnu/release/main.exe
nyra build . --release --for linux
nyra build . --release --os linux --arch aarch64
nyra build . -o mybin         # custom name under target/{profile}/
nyra check
nyra test .
nyra test . --list-json          # IDE test discovery (file, name, line)
nyra test . --filter adds        # run matching tests only
nyra fmt .
nyra diag . --json
nyra explain E003              # explain stable diagnostic codes
nyra explain --list
nyra check . --deny-extended   # Core-only (reject Extended tier)
nyra pkg init
nyra pkg install ny-sqlite@^0.1.0   # semver + fetch + merge link / link-source
nyra pkg verify                     # lock checksums + semver constraints
nyra pkg build                      # verify lock then compile
nyra pkg prune                      # remove unused imports, prefix unused locals (W002/W003)
nyra pkg prune --check              # dry run — report only, no edits
nyra build lib.ny -o mylib --cdylib # shared lib for Python/Node/Rust hosts
nyra debug .                         # build -g + launch lldb/gdb (CLI)
nyra dap                             # DAP adapter (stdio) — VS Code extension
nyra build . --debug-symbols         # required before source-level debugging
```

### Build output layout (Cargo-style)

```
myapp/
  main.ny
  target/
    debug/main          # nyra build  or  nyra run (host)
    release/main        # nyra build --release
    x86_64-pc-windows-gnu/release/main.exe   # nyra build --release --for windows
```

- **Projects** (`nyra build` / `nyra build .`): binary name is **`main`** (from `main.ny`).
- **Single file** (`nyra build app.ny`): binary stem matches the file (`app`).
- **`-o name`**: override binary name inside `target/{profile}/` or `target/{triple}/{profile}/` when cross-compiling.
- **Cross-compile:** `--for windows|linux|macos|wasm`, or `--os` + optional `--arch`, or `--target TRIPLE`.
- **Windows cross:** `.exe` extension; `spawn`, TCP, and `async`/`await` supported (Winsock2 + Win32 threads/sync). Requires MinGW-w64 sysroot when cross-compiling from macOS/Linux (`NYRA_SYSROOT` or `--target x86_64-pc-windows-gnu` with clang + mingw).
- **Wasm:** `nyra build --for wasm app.ny -o app.wasm` → `target/wasm32-wasi/debug/app.wasm`.
- Add **`target/`** to `.gitignore` (like Rust).

Ship the executable from `target/release/` (or `target/{triple}/release/`) for production; run `./target/debug/main` while developing.

Release flags: `--release`, `--opt 0-3`, `--lto`, `--lto-full`, `--no-lto`, `--no-llvm-opt`, `--no-prelude`, `--native-cpu`, `--no-native-cpu` (host `--release` uses `-march=native` by default), `--pgo-generate`, `--pgo-use FILE`, `--race` (ThreadSanitizer for async/concurrency debug), `--for`, `--os`, `--arch`, `--target`.

Systems / freestanding: `--no-std` (skip `nyra_rt` link), `--freestanding` (`-ffreestanding -nostdlib`). Top-level `no_std` in source has the same effect as `--no-std`.

**Environment variables:**

| Variable | Purpose |
|----------|---------|
| `NYRA_SYSROOT` | MinGW/sysroot path for Windows cross-compile from macOS/Linux |
| `NYRA_HOME` / `~/.nyra/config` | NyraPkg registry URL and cache defaults |

## Syntax conventions

- **Easy syntax first:** no semicolons, minimal ceremony, inference by default (see [Design philosophy](#design-philosophy)).
- **Blocks:** `{` `}` with optional indentation (no significant whitespace requirement).
- **Statements:** one per line; no semicolons required.
- **Comments:** `//` line comments; `/* ... */` block comments (non-nested, multiline OK).

```ny
// line comment
let x = 1 /* inline block */ + 1
/*
 * multiline header
 */
```

Unclosed `/*` is a lexer error.  `CONF-COMMENT-*`.
- **Entry:** `fn main()` in `main.ny` for projects.
- **Naming:** `snake_case` for functions/variables; `PascalCase` for types/enums.

## Variables

A **variable** is a name for a value. Nyra has three main forms:

### `let` — immutable binding

```ny
let score = 10
// score = 20   // ERROR — cannot reassign without mut
```

`let` means: bind this name once. Reading the value is always OK; replacing it is not (unless you used `mut`).

### `mut` — mutable (changeable)

`mut` is short for **mutable**. The variable can be reassigned after creation.

```ny
let mut lives = 3
lives = lives - 1   // OK

mut counter = 0     // shorthand: mutable without repeating let (common in loops)
counter = counter + 1
```

Use `let mut` (or `mut`) for counters, loop indices, accumulators, and any value that changes over time.

### `const` — compile-time constant

```ny
const MAX_HP = 100
```

Fixed at compile time; shared fixed values across the program. Not the same as `let` — you cannot compute `const` from runtime input unless the expression is folded at compile time (see **comptime** below).

### `comptime` — compile-time evaluation (optional)

Three forms:

1. **File-level** — put `comptime` on the **first line** of a file. The **entire unit** is compile-time only; only `pub const` (and optional `pub struct`/`enum`) are exported to importers. Comptime modules cannot define `main`, `spawn`, `async`, `print`, or `extern`.

```ny
comptime

fn mix(n) {
    return n * 3
}

pub const SEED = mix(14)
```

2. **Function-level** — put `#[comptime]` on a **single function** in a normal file. Calls with known arguments fold at compile time; the function is stripped from the runtime binary.

```ny
#[comptime]
fn mix(n) {
    return n * 3
}

const SEED = mix(14)

fn main() {
    let seed = SEED   // 42 — folded at compile time
}
```

3. **Block expression** — `comptime { ... }` folds a compile-time block to a value (trailing expression or `return`).

```ny
const TOTAL = comptime {
    let mut acc = 0
    for i in 0..10 {
        acc = acc + i
    }
    acc
}
```

Import from a normal file (file-level comptime module):

```ny
import "tables.ny" as tables

fn main() {
    let seed = tables::SEED   // 42 — folded at compile time
}
```

Check a comptime module with `nyra check path/to/tables.ny` (no `main` required). Patterns: file-level `comptime`, `#[comptime]` fn, `comptime { }` block.

**Philosophy (Zig-like, optional):** Nyra does not force comptime — use normal runtime code by default. When you need lookup tables, string routing, or generated constants with **zero runtime cost**, opt in via `comptime` file, `#[comptime]`, or `comptime { }`. The interpreter folds values before codegen so hot paths stay lean (like Zig `comptime`, but always optional).

**Supported in comptime eval:** integers, bools, **strings** (literals, `+` concat, `==` / `!=`), fixed arrays (`[1, 2, 3]`, `[x; N]`, `[x; param]`, spreads), **`.len()`** on arrays/strings, **mutable array/struct updates** (`table[i] = v`, `s.field = v` with `let mut`), **structs** (literals, field access, spread, struct match), **enums** (unit + payload), **tuples**, `for i in 0..N`, `for x in arr`, `while` / `break` / `continue`, `comptime { }` blocks, **`match`** (enum, bool, **integer literals**, **string literals**, guards, struct, tuple), generic calls (monomorphized before eval), pure function calls, `if` / `return` / `let mut`.

**Match in comptime:** enum arms (`Status.Ok`, `Opt.Some(x)`), bool (`true` / `false`), integer literals (`0`, `7`, …) and `_ if guard`, **string literals** (`"GET" => …`), struct arms (`Point { x, y }`), tuple arms (`(a, b)`). Or-patterns (`A | B =>`) work via desugar.

**Comptime modules export:** `pub const` values plus **`pub struct` / `pub enum`** type definitions (private helpers and functions are stripped).

**Still forbidden in comptime:** `main`, `print`, `spawn`, `async`, `extern`, `unsafe`, `asm`, `parallel for`. Runtime calls to `#[comptime]` functions are a compile error. Reassigning `let` (non-`mut`) bindings is rejected.

Use `priv const` for internal comptime helpers (Nyra defaults to `pub` when visibility is omitted); export only `pub const` values importers need.

| | `let` | `let mut` | `const` | `comptime` file | `#[comptime]` fn |
|---|-------|-----------|---------|-----------------|------------------|
| Reassign? | No | Yes | No | N/A (no runtime) | N/A (stripped) |
| When set? | Runtime in code | Runtime; can change | Compile time | Compile time (whole file) | Compile time (per call site) |
| Example | `let name = "Ali"` | `let mut gold = 0` | `const MAX = 100` | `comptime` + `pub const TABLE = ...` | `#[comptime] fn hash(n) { ... }` |

- Immutable `let` of **Move** types (heap `string`) transfers ownership on use.
- `let mut` of Copy types (`i32`, `bool`, enums) is not moved on function call.

Integer separators: `1_000_000`

### Metaprogramming (compile-time code generation)

Nyra metaprogramming inspects or generates code **during compilation** — zero runtime reflection on hot paths:

| Mechanism | When it runs | Example |
|-----------|--------------|---------|
| **Comptime** | const fold before codegen | `comptime` file, `#[comptime] fn`, `comptime { }` |
| **Macros** | AST substitution before typecheck | `macro field_sum(a,b,c) { a + b + c }` |
| **Generics** | Monomorph before LLVM IR | `fn id<T>(x: T) -> T` → `id__i32`, `id__string` |
| **Struct JSON** | Compiler synthesis after typecheck | `{Struct}_json_encode` / `_json_decode` |

**Struct → JSON (automatic):** declare an eligible struct; the compiler emits encode/decode helpers — no runtime serde registry:

```ny
struct User { name: string, age: i32 }
let json = User_json_encode(User { name: "Ada", age: 30 })
```

See `examples/toolchain/metaprogramming.ny`, `examples/struct_serde.ny`, `stdlib/meta/mod.ny`, `stdlib/serde/mod.ny`.

## Language reference

Quick lookup for syntax the lexer and parser accept today. Types are optional unless inference fails.

### Keywords

| Keyword | Purpose |
|---------|---------|
| `fn` | Function definition |
| `let` / `let mut` | Immutable / mutable binding |
| `const` | Compile-time constant |
| `comptime` | Optional compile-time evaluation (file, `#[comptime]`, or `comptime { }` block) |
| `if` / `else` | Conditional (also expression) |
| `while` | Loop |
| `break` | Exit innermost `while` / `for` |
| `for` / `in` | `for i in 0..10` or `for x in arr` |
| `return` | Return from function |
| `match` | Pattern match on enums / values |
| `struct` / `enum` | User-defined types |
| `impl` | Methods; `impl Trait for Type` |
| `import` | Load another `.ny` file |
| `module` | Module declaration |
| `extern` / `export` | FFI declare / export C symbol |
| `test` | Test function |
| `print` | Built-in stdout; optional `color:` |
| `spawn` / `spawn:task` / `spawn:thread` | Concurrent block — task pool (default) or OS thread (Extended) |
| `allow_extended` | File directive — marks Extended-tier unit (see [spawn](#spawn--spawntask--spawnthread-extended--no-import-keyword)) |
| `parallel for` / `parallel:task` / `parallel:thread` | Parallel loop — task pool (default) or OS thread chunks (Extended) |
| `progress for` | Progress bar loop (Extended) |
| `benchmark` | Timed block with Time/Memory/CPU report (Extended) |
| `defer` | Scope-exit call (LIFO) — **Extended**; prefer auto-drop / `impl Drop` (see below) |
| `unsafe` | Raw memory block |
| `asm` | Inline assembly inside `unsafe` |
| `as` | Type cast (`expr as Type`) |
| `no_std` | Skip `nyra_rt` link |
| `move` / `clone` | Explicit move or clone at call site |
| `async` / `await` | Parsed; runtime evolving (Extended) |
| `trait` / `dyn` | Trait defs, static impl, trait objects `dyn Trait` (Extended) |
| `self` | Method receiver in `impl` |

### Operators

| Category | Operators |
|----------|-----------|
| Arithmetic | `+` `-` `*` `/` `%` |
| Comparison | `==` `!=` `<` `<=` `>` `>=` |
| Logical | `&&` `\|\|` `!` |
| Nullish / optional (v2.1+) | `??` `?.` `?.method()` |
| Reference | `&x` (shared), `&mut x` (exclusive) |
| Raw (inside `unsafe`) | `*ptr` load, `*ptr = v` store, `ptr + i32`, `ptr - i32` |
| Cast | `expr as Type` — raw casts need `unsafe` |
| Field / call | `obj.field`, `f(a, b)`, `obj.method(a)` |

### Literals

| Form | Example | Inferred type |
|------|---------|---------------|
| Integer | `42`, `1_000_000` | `i32` |
| Float | `3.14`, `1e-3` | `f64` |
| Character | `'a'`, `'\n'`, `'\u{1F600}'` | `char` |
| Boolean | `true`, `false` | `bool` |
| String | `"hello"` | `string` (static literal) |
| Template | `` `Hello ${name}!` `` | `string` (interpolates `i32`, `f64`, `string`, `bool`, `char`, and **any expression** including calls like `${fmt(x)}`) |
| Array | `[1, 2, 3]` | `[i32; 3]` when annotated |
| Tuple | `(1, "a")` | `(i32, string)` when annotated |
| Object | `{ name: "a", age: 1 }` | inferred struct (or matches declared struct) |

### Statements

```ny
let x = 10
let mut n = 0
n = n + 1
const MAX = 100

if x > 0 { print(x) } else { print(0) }

while n < 10 {
    if n == 5 { break }
    n = n + 1
}

for i in 0..5 { print(i) }       // half-open range
for v in [1, 2, 3] { print(v) } // array elements
for c in "hi" { print(c) }       // char codes per byte

return x
print("ok", color: green)
defer close_handle(h)          // Extended — prefer impl Drop RAII when possible
allow_extended                   // optional file directive when using Extended APIs
spawn { print(1) }               // Extended — default = task pool; use spawn:thread for OS thread
let h = spawn { work() }         // returns JoinHandle; h.join() blocks until done
unsafe { let p = &x as *i32; *p = 7 }
import "stdlib/fs.ny"
```

### How code becomes a binary

```text
Source (.ny)
  → Lexer → Parser → Macro expand
  → Monomorph (+ generic call inference at call sites)
  → Auto-borrow coercion (pass owned T as &T when callee expects ref)
  → Typecheck
  → Ownership (Copy inference, move tracking)
  → Borrow + lifetimes + Send/Sync
  → Escape analysis (NoEscape / ArgEscape / GlobalEscape)
  → Drop plan (auto-free at scope exit)
  → LLVM IR codegen
  → llvm opt (-O0 debug, -O3 release)
  → clang link + nyra_rt C modules
  → target/debug/main or target/release/main
```

Stop early without linking: `nyra check .` · JSON diagnostics: `nyra diag . --json`

## Types

> **Optional annotations:** Nyra has a full static type system (`i32`, `string`, structs, enums, generics), but **you do not have to write types** for most code. Add them only when you want clarity or the compiler requires them. See [Design philosophy](#design-philosophy).

| Type | Ownership | Notes |
|------|-----------|-------|
| i8–i128, u8–u128, isize, usize | Copy | Full integer families (optional annotations); literals default to `i32` |
| f32 | Copy | IEEE-754 single; literals `1.5f32` or annotate `f32` |
| f64 | Copy | IEEE-754 double; literals like `3.14`, `1e-3` (default for floats) |
| char | Copy | Unicode scalar; literals `'a'`, `'\n'`, `'\u{1F600}'` |
| bool | Copy | true / false |
| string | Move | UTF-8 pointer; literals are static |
| bytes | Move | Binary blob handle; **not** implicitly convertible to `string` |
| void | — | No return value (Rust `()` unit type) |
| struct Name { fields } | Copy or Move | Move if any field is Move; `repr(C)`, `align(N)`, `packed` |
| union Name { fields } | Copy | C-style union; field access only in `unsafe` |
| enum Name { A, B } | Copy | **Tag-only** by default — unit variants, no stored data |
| enum Name { Some(T) } | Copy or Move | **With payload** — heterogeneous payloads per variant supported |
| i32x4 / f32x4 / f64x2 | Copy | Portable SIMD vector types |
| option / Option | Copy | Built-in **tag names** for `??` / `?.` desugar; **payloads only after** `import "stdlib/option.ny"` |
| result / Result | Copy | Same split as `Option` — tags built-in; `Ok(v)` / `Err(e)` need stdlib import or monomorph enum |
| [T; N] | depends | Fixed array; type syntax `[i32; N]` or `[i32: N]` |
| [T] | depends | Slice (MVP) |
| (T, U, ...) | depends | Tuple; field access `.0`, `.1`; `let (a, b) = pair` |
| &T / &mut T | Borrow | References |
| &'a T | Borrow | Explicit lifetime |
| for<'a> fn(...) | — | HRTB function pointer type |
| ptr | Copy | Opaque FFI handle; Send |
| *T | Copy | Typed raw pointer; `*const T` / `*mut T` accepted (same semantics); !Send / !Sync |

Type annotations: `let x: i32 = 0`, `let b: u8 = 255`, `fn f(n: i32) -> bool` — all optional when inference suffices.

### Systems & stdlib types (vs Rust)

| Rust | Nyra | Status |
|------|------|--------|
| `fn(i32)->i32` | `fn(i32) -> i32` | Language |
| `\|x\| x+1` | `(x) => x + 1` | Extended closures |
| `struct` / `enum` | `struct` / `enum` | Language |
| `Option<T>` / `Result<T,E>` | `import "stdlib/option.ny"` | Stdlib + `?` / `??` / `?.` |
| `Vec<T>` | `Vec_i32`, `import "stdlib/vec.ny"` | Stdlib (monomorph) |
| `Vec<Vec<i32>>` | `Vec_Vec_i32_*`, `import "stdlib/collections/nested_vec.ny"` | Stdlib (nested MVP v1.25) |
| `Vec<MoveStruct>` | `Vec_{Struct}_*`, `import "stdlib/collections/vec_pod.ny"` | Reloc expand (string + scalars + nested; v1.27) |
| `HashMap<K,V>` | `HashMap_str_i32`, `HashMap_str_str` | Stdlib |
| `HashSet<T>` | `HashSet_str` | Stdlib |
| `Box<T>` / `Arc<T>` | `stdlib/box.ny`, `stdlib/arc.ny` | Partial / shipped |
| `Mutex<T>` / `RwLock<T>` | `stdlib/sync/mutex.ny`, `rwlock.ny` | Stdlib |
| `AtomicI32` / `AtomicBool` | `Atomic_i32`, `AtomicBool` in `stdlib/sync/atomic.ny` | Stdlib |
| `Rc`, `Cell`, `RefCell`, `Pin`, `PhantomData`, `Cow`, `!` | — | Not in Nyra MVP |
| `size_of` / `align_of` | `size_of<T>()`, `align_of<T>()` | Compiler intrinsics (`stdlib/mem/layout.ny`) |
| Stack buffer | `StackBuffer_i32_64` (`stdlib/buf/stack.ny`) | Stack-only; cannot be returned |
| Arena | `Arena_new` / `Arena_alloc` (`stdlib/alloc/arena.ny`) | Bump allocator, O(1) reset |
| SIMD | `simd_add_i32x4`, `stdlib/simd/mod.ny` | Portable + platform (`x86.ny`, `arm.ny`) |

 (zero types) and `.typed.ny`.

**Integer literals** default to `i32`, but bind to any integer type when the target is known — e.g. `let c = Color { r: 18, g: 52, b: 86, a: 255 }` with `r: u8` fields accepts `255` without `: u8` on each literal.

### Clone (strings & cloneable structs)

Two equivalent forms (both compile):

```ny
let b = clone a          // prefix at call site or in let initializer
let c = a.clone()        // method call (`.clone` is a keyword after `.`)
```

Use when a `string` (Move type) must be reused after a call that would move it (e.g. `strcat(a, b)`).

```ny
let prefix = "tab"
let key = strcat(clone prefix, "_name=")   // prefix still valid
// or: let key = strcat(prefix.clone(), "_name=")
```

## Functions

```ny
fn add(a: i32, b: i32) -> i32 {
    return a + b
}

fn greet(name: string) -> void {
    print(name)
}

// Generic (monomorphized at compile time)
fn id<T>(x: T) -> T {
    return x
}
```

- `return expr` or `return` for void.
- Struct parameters passed by pointer in LLVM (`%Struct*` ABI).
- `export fn` — unmangled C symbol for FFI out.
- `extern fn` — declare C/runtime symbol (not `extern export fn`).

### C Bindgen & `nyra pkg c`

**Recommended:** `nyra pkg c add NAME` — raylib, zlib, sqlite3, sdl2. Installs (Homebrew), full bindgen, `nyra.mod`, `vendor/bindings/c-libs.toml`.

```bash
nyra pkg c add raylib
nyra pkg c add zlib
nyra pkg c list
nyra pkg c remove raylib     # delete bindings + unlink nyra.mod
nyra pkg c add raylib --no-install --path ./myapp
```

**Manual bind** (any `.h`): `nyra pkg bind c HEADER --lib foo --update-mod`

Default: all bindable functions in `vendor/bindings/{stem}.ny`. C keyword params → `in_`, `type_`. Optional `--export` to shrink. `--shim` experimental.

Docs: [c-bindgen](https://nyra-lang.github.io/docs/c-bindgen.html)
### Template strings (v1.2 — Core)

Backtick strings with JS-style `${expr}` interpolation (static text + `i32` / `string` values):

```ny
let name = "hamdy"
let age = 25
print(`Hello, ${name}!`)
print(`Hello ${name}, age ${age}`)
```

 · [learn strings](https://nyra-lang.github.io/docs/learn-strings.html).

### Arrow functions (v2.1+ — Extended)

ES6-style lambdas; non-capturing arrows are hoisted to `__arrow_N` before typecheck. Capturing closures use **stack alloca** env structs for synchronous use; **heap promotion** (v2.2) when the closure escapes (returned, passed to `fn(...)`).

```ny
// Inferred param types (v2.1)
let add_one = (x) => x + 1
let inc = x => x + 1

// Explicit types (still supported)
let add_one_typed = (x: i32) => x + 1

// Tuple destructure param
let sum_pair = ((a, b)) => a + b

// Block body
let double = (x: i32) => {
    return x * 2
}

// Capturing closure — safe in loops when passed to sync callbacks (e.g. iter_filter)
let threshold = 2
let pred = (x) => if x > threshold { 1 } else { 0 }

// Escaping closure — heap env (v2.2)
fn make_adder(n: i32) -> fn(i32) -> i32 {
    return (x) => x + n
}
```

Pass as `fn(...)` parameters: `iter_filter(v, pred)` or `serve_handlers(host, port, max, router, health_slot)`.

### Nullish coalescing & optional chaining (v2.1+)

Desugared to `match` on the built-in `Option` **tag** names before typecheck.

**Important:** `??` and `?.` compile against `Option.None` / `Option.Some` patterns. To **store and read a real value** in `Some(v)`, import the generic enum:

```ny
import "stdlib/option.ny"

let x = Option.None
let y = x ?? 42              // y is 42 (None arm)

let z = Option.Some(99)      // stores i32 payload — requires import above
let w = z ?? 0               // w is 99 (Some(v) arm binds v)

let f = opt?.field           // optional field chain
let m = opt?.method()        // optional method chain (v2.2)
```

Without `import "stdlib/option.ny"`, `Option.Some(99)` is a **type error** (built-in `Some` expects zero args). Use the import for any code that constructs or matches payload values.

## Generics

Monomorph generics specialize at compile time — no runtime type info on hot paths.

```ny
fn id<T>(x: T) -> T {
    return x
}

fn main() {
    print(id(7))           // T = i32 → id__i32
    print(id("hi"))      // T = string → id__string
}
```

| Syntax | Meaning |
|--------|---------|
| `fn f<T>(x: T) -> T` | Type parameter `T` on function |
| `struct Box<T> { … }` / `enum Option<T>` | Generic types — monomorph to `Box__string`, `Option__i32`, … |
| `fn g<T: Trait>(x: T)` | Trait bound — requires `impl Trait for T` at monomorph site |
| `T: A + B` | Multiple bounds |
| `for<'a> fn(&'a T) -> i32` | HRTB function pointer type |

**Naming after monomorph:** `Foo__T` or `Foo_T` (underscore style in stdlib: `Vec_i32`, `HashMap_str_i32`). Prefer explicit monomorph names in Core CI; generic syntax `Vec<T>` is Extended but compiles.

**Inference:** `id(7)` infers `T` from the argument; ambiguous sites error with **E004** — add `: Type` manually.



## Control flow

```ny
// if / else
if x > 0 {
    print("positive")
} else {
    print("non-positive")
}
let sign = if x >= 0 { 1 } else { -1 }   // if expression

// while
let mut i = 0
while i < 10 {
    print(i)
    i = i + 1
}

// for range (half-open: start..end)
for j in 0..5 {
    print(j)    // 0, 1, 2, 3, 4
}

// break / continue
let mut i = 0
while i < 10 {
    i = i + 1
    if i == 5 { continue }   // skip rest of iteration
    if i == 8 { break }      // exit loop
    print(i)
}
```

## Match expressions

`match` is **exhaustive** — cover all variants or use `_` wildcard. Works on enums, `bool`, integer literals, **strings** (v1.17+), structs, tuples.

```ny
enum Color { Red, Green, Blue }
let n = match color {
    Color.Red => 1
    Color.Green => 2
    Color.Blue => 3
}

// if expression (not match)
let sign = if x >= 0 { 1 } else { -1 }
```

### Or-patterns, guards, wildcards

```ny
match c {
    Color.Red | Color.Blue => 1    // single | not ||
    Color.Green => 2
}

match method {
    "GET" | "HEAD" => 200
    "POST" | "PUT" => 201
    _ => 404
}

match r {
    Result.Ok(v) if v > 0 => v     // guard after pattern
    Result.Ok(_v) => 0
    Result.Err(e) => e
}
```

### Payload binds & nested enums (v1.33+)

```ny
enum Option_i32 { None, Some(i32) }
enum Result_Opt { Ok(Option_i32), Fail(Option_i32) }

match r {
    Result_Opt.Ok(Some(x)) => x
    Result_Opt.Ok(Option_i32.None) => 0
    Result_Opt.Fail(_) => -1
}
```

**Limit:** all payload-bearing variants in one enum must share the **same** payload type — no `Ok(Option)` + `Err(i32)` mix.

### Struct & tuple patterns (v1.37+)

```ny
match p {
    Point { x, y } => x + y      // field bind shorthand
}
match pair {
    (a, b) => a + b
}
```

Field-value patterns (`Point { x: 0, y }`) are **not** supported — bind fields only.

### Match rules summary

| Feature | Supported |
|---------|-----------|
| Enum unit / payload arms | ✅ |
| `bool`, integer literal arms | ✅ |
| String literal arms on `string` scrutinee | ✅ |
| `_` wildcard, `_ if guard` | ✅ |
| Or-patterns `A \| B` | ✅ |
| Struct / tuple destructure | ✅ |
| Trailing comma after arm body | ✅ |
| Field-value struct patterns | ❌ |

Further reading: [match](https://nyra-lang.github.io/docs/match.html)

## Structs (objects)

**Two styles — same compile-time struct layout, zero runtime cost:**

1. **Optional `struct` declaration** — name your shape for APIs, `impl`, and FFI.
2. **Anonymous object literal** — `{ field: value }` when you only need a grouped value; the compiler infers field types and synthesizes a struct (or reuses a declared struct with the same fields).

```ny
// Zero-types: no struct keyword required
fn main() {
    let family = {
        name: "hamdy",
        age: 20,
        city: "cairo"
    }
    print(family.name)
}

// Optional explicit struct (organization / public API)
struct Point {
    x: i32
    y: i32
}

fn demo() {
    let p = Point { x: 1, y: 2 }
    let q = { x: 3, y: 4 }   // same shape → uses Point when fields match
    print(p.x)
}
```

**Literal fields use commas** (`{ x: 1, y: 2 }`). **Struct definitions** use newlines between fields (no commas). Error `P006` if a literal omits a comma — common after `some_fn()`.

Methods via `impl`:

```ny
impl Calculator {
    fn add(self, n: i32) -> Calculator {
        Calculator { value: self.value + n }
    }
}
// call: c.add(10)  →  Calculator_add(c, 10)
```

**`impl` rules:**

| Form | Purpose |
|------|---------|
| `impl Type { fn method(self, …) }` | Instance methods — `self` is owned receiver |
| `impl Drop for Type { fn drop(self) }` | RAII cleanup at scope exit (preferred over `defer`) |
| `impl Trait for Type { fn trait_method(self, …) }` | Static trait dispatch (Extended) |
| `impl Type { fn new() -> Type }` | Constructor pattern (convention, not keyword) |

Method calls borrow or move `self` per signature. Chaining works when methods return `self` (e.g. `HashMap_str_i32.insert`).

**Tuples:** `(a, b)` — access `.0`, `.1`, …; destructure `let (x, y) = pair`; match arms `(a, b) => …`.

## Enums & payloads

Nyra has **two enum modes**. Do not mix them up — error handling depends on which you use.

### 1. Tag-only enums (default)

Unit variants — no data stored. LLVM layout: `i32` tag.

```ny
enum Color { Red, Green, Blue }

let c = Color.Red
let n = match c {
    Color.Red => 1
    Color.Green => 2
    Color.Blue => 3
}
// Or-patterns (v1.30+): shared body for multiple variants
let bucket = match c {
    Color.Red | Color.Blue => 1
    Color.Green => 2
}
// Nested binds (v1.33+): peel payload enums in one arm
// match res { Result.Ok(Some(x)) => x, Result.Ok(Option.None) => 0 }
// Struct / tuple patterns (v1.37+): Point { x, y }, (a, b)
// String match: `"GET" | "HEAD" => 1`
// Color.Red(42)  // ERROR — no payload declared
```

Built-in **`Option` / `Result` names** (no import): the compiler registers tag names `None`, `Some`, `Ok`, `Err` for `??` / `?.` desugar and pattern matching. These built-ins are **tag-only** — `Option.Some(42)` without import is invalid.

### 2. Enums with payloads (Extended — v2.2+)

Declare a payload type on variants, or import the stdlib generic enums.

**Monomorph enum (explicit):**

```ny
enum Option_i32 {
    None,
    Some(i32),
}

let x = Option_i32.Some(42)
let n = match x {
    Option_i32.Some(v) => v    // v is i32
    Option_i32.None => 0
}
```

**Generic stdlib (recommended for `Option` / `Result`):**

```ny
import "stdlib/option.ny"

enum Option<T> {
    None,
    Some(T),
}

enum Result<T, E> {
    Ok(T),
    Err(E),
}

let ok = Result.Ok(200)
let err = Result.Err("not found")
let maybe = Option.Some("hello")
```

Monomorphization produces instanced types (e.g. `Option__i32`) at compile time. Heap payloads (e.g. `Option<string>`) get automatic payload drop (v2.4).

**MVP payload rules (RFC 0002):**

| Rule | Detail |
|------|--------|
| Fields per variant | At most **one** payload field |
| Shared type | All payload-bearing variants in one enum share the **same** payload type |
| Unit variants | `None`-style variants have no field |
| Layout | `{ i32 tag, T payload }` when any variant has payload; else tag-only `i32` |

**Result / error handling — `?` operator (v1.3.2+):**

| Status | Detail |
|--------|--------|
| **Shipped** | `let x = fallible()?` / `const` / `return expr?` / expression statement `fallible()?`; `?` inside nested expressions (`print(step(1)?)`, call args, `return match … { Ok(x) => step(x)? }`, `let n = match … { Ok(v) => step(v)? }`); early `return` on `Err`/`None` when the enclosing function returns the same enum; in `void` / non-`Result` functions the final `match` uses the `Err` payload as the `i32` value |
| **Patterns** | `Result.Ok` / `Result.Err` and `Option.Some` / `Option.None` aliases match monomorph names (`Result_i32_i32`, `Result__i32_i32`, …) |
| **Requirement** | Enclosing function return type must be the same `Result`/`Option` enum (or monomorph) so `Err`/`None` can propagate |
| **Match arms** | Optional trailing comma after an arm body (`=> expr,`) |

```ny
enum Result_i32_i32 { Ok(i32), Err(i32) }

fn step(n: i32) -> Result_i32_i32 {
    return Result_i32_i32.Ok(n)
}

fn pipeline() -> Result_i32_i32 {
    let a = step(1)?
    let b = step(a + 1)?
    return Result_i32_i32.Ok(b * 2)
}
```

Verbose `match` per step still works (and is required when the function returns a plain `i32`):

```ny
fn pipeline_verbose() -> i32 {
    let v1 = match Result_i32_i32.Ok(1) {
        Result_i32_i32.Ok(x) => x
        Result_i32_i32.Err(_e) => 0
    }
    // ...
    return v1
}
```

| Approach | When |
|----------|------|
| `?` on `Result` / `Option` | Function returns the same enum — preferred for fallible pipelines |
| Explicit `match` per step | Unwrap to a scalar return type (`i32`, `string`, …) |
| `unwrap_*` helpers | `stdlib/result.ny` — e.g. `unwrap_i32_result(r, default)` |

 , Conformance: `CONF-ADT-004`

Generic `Result<T,E>` / `Option<T>` (auto-prelude or `import "stdlib/option.ny"`) supports `?` the same way after monomorphization.

**Official application errors (v1.40+):** import `stdlib/error.ny` for Nyra's batteries-included error path. Use `Result<T, Error>` plus `?`, `Error_context`, `Error_format`, and specialized helpers (`Result_string_context`, `Result_i32_context`) to compose I/O + JSON + validation without third-party packages.

```ny
import "stdlib/error.ny"
import "stdlib/json/mod.ny"

fn config_port(json_text) -> Result<i32, Error> {
    let port = Result_i32_context(json_i32(json_text, "port"), "reading config")?
    return Result.Ok(port)
}
```

Fallible stdlib wrappers: `read_text`, `write_text`, `append_text` (`stdlib/fs/result.ny`); `json_string`, `json_i32`, `json_bool` (`stdlib/json/mod.ny`). `Error_format(err)` prints kind/message, context/cause, and a runtime stack trace when available.

See: `stdlib/option.ny` · `stdlib/result.ny` · `stdlib/error.ny`.

## Imports & modules

```ny
module my.app

import "lib/helpers.ny"
import "types.ny"
import "lib/api.ny" as api

fn main() {
    print(APP_TITLE)        // const from imported file
    print(api::version())   // alias::name → api__version
}
```

- Project root: `main.ny` + optional `nyra.mod`.
- Paths relative to importing file: `import "src/engine.ny"`.
- Import brings **public** symbols into scope; `priv` hides from importers.
- `import "path" as alias` + `alias::symbol` qualified calls (v1.37+).

### Visibility (`pub` / `priv`)

| Modifier | Meaning |
|----------|---------|
| *(omit)* | **`pub` by default** — exported to importers |
| `priv fn` / `priv struct` / `priv const` | Hidden from other files that import this module |
| `pub const` | Explicit export (required in comptime modules) |

```ny
priv fn helper() { … }          // internal only
fn public_api() { … }           // visible to importers
```

### Workspace (`nyra.mod`)

Declare module identity, stdlib requires, and native link metadata:

```text
module my.api
version 1.0.0
require ny-sqlite ^0.1.0
require stdlib.net.http
link sqlite3
link-source vendor/shim.c
```

Typical layout:

```text
my_api/
  nyra.mod
  main.ny
  lib/common/
  handlers/
  workers/
```

Workspaces: [modules guide](https://nyra-lang.github.io/docs/modules.html)

## I/O & builtins

Further builtins reference: [https://nyra-lang.github.io/docs/methods.html](https://nyra-lang.github.io/docs/methods.html) · [https://nyra-lang.github.io/docs/stdlib.html#builtins](https://nyra-lang.github.io/docs/stdlib.html#builtins)

**Quick lookup — what needs an import?**

| Category | Import? | Receiver / type |
|----------|---------|-----------------|
| I/O, `date()`, timing, `spawn`, `JoinHandle.join()`, `parallel for`, `random()` / `random_f64()` | **No** | Global functions / methods |
| String `.split()` / `.trim()` / … | **No** | `string` — borrows receiver |
| Fixed array `.len()` / `.sort()` / `.sort_by()` | **No** | `[T; N]` |
| Split list `.len()` / `for s in parts` | **No** | result of `.split()` |
| `Vec_i32_*` / `vec_*` | auto-prelude or `import "stdlib/vec.ny"` | `ptr` handle |
| `StrVec` methods | auto-prelude or `import "stdlib/vec_str.ny"` | `StrVec` struct |
| `HashMap_str_*` methods | auto-prelude or `import "stdlib/map.ny"` | `HashMap_str_i32`, `HashMap_str_str` |
| `Array_*` / `String_*` / `Math_*` / `JSON_*` | `import "stdlib/builtins_*.ny"` | Function-style wrappers |

**User-defined methods:** declare with `impl TypeName { fn method(self, …) -> TypeName { … } }` — call as `obj.method(arg)` (lowers to `TypeName_method(obj, arg)`). `impl Drop for T` runs at scope exit. `impl Trait for T` for static dispatch; `dyn Trait` for trait objects (Extended).

### I/O (no import)

```ny
print("line")                    // stdout + newline (string, i32, bool, char, f64, fixed arrays)
print([1, 2, 3])                 // [1, 2, 3] — fixed arrays of printable scalars
print("OK", color: green)        // ANSI color — names, #RGB, #RRGGBB, rgb(r,g,b)
print("Err", color: "#FF0000")
write("buf")                     // buffered, no newline
println("line")                  // buffered + newline
flush()
let s = input()                  // read stdin line
let name = input("Name? ")       // prompt then read
```

**Color names:** `red`, `green`, `blue`, `yellow`, `cyan`, `magenta`, `white`, `black`, `bold`, `dim`, `bright_red`, …  
**String escapes:** `\n`, `\t`, `\\`, `\"`, `\033`, `\x1b`, `\u{1b}`.

### `date()` — local calendar (no import)

Returns a `Date` struct (fields, not methods). Month is **1–12**.

```ny
let d = date()
print(d.year)        // e.g. 2026
print(d.month)       // 1–12
print(d.day)
print(d.hour)
print(d.minute)      // alias: d.minutes
print(d.second)      // alias: d.seconds
print(d.week)        // 0=Sun … 6=Sat; alias: d.weekday
print(d.millisecond)
```

### String methods (no import)

Methods borrow the receiver (do not move). Heap copy: `clone s` or `s.clone()`.

| Method | Args | Returns | Notes |
|--------|------|---------|-------|
| `.length()` / `.len()` | — | `i32` | Byte length |
| `.split(sep)` | `string` | split list | `for part in parts` |
| `.trim()` | — | `string` | Strip whitespace |
| `.contains(s)` | `string` | `i32` | `1` / `0` |
| `.starts_with(s)` / `.ends_with(s)` | `string` | `i32` | Prefix / suffix test |
| `.replace(from, to)` | 2 × `string` | `string` | All matches (Rust-style) |
| `.replacen(from, to, count)` | 2 × `string`, `i32` | `string` | At most `count` matches (`1` = first only) |
| `.to_upper()` / `.to_lower()` | — | `string` | ASCII case |
| `.clone()` | — | `string` | Heap copy |

```ny
let parts = "a,b,c".split(",")
for p in parts { print(p) }
print("hello".trim().to_upper())
```

### Number & math intrinsics (no import)

Compiler builtins — lowered to LLVM, zero call overhead. Work with `--no-prelude`.

| Call | Args | Returns | Notes |
|------|------|---------|-------|
| `abs(x)` | `i32` or `f64` | same as arg | Type overload |
| `abs_i32(x)` / `abs_f64(x)` | 1 numeric | `i32` / `f64` | Explicit typed variants |
| `min_i32(a, b)` / `max_i32(a, b)` | 2 × `i32` | `i32` | Min / max |
| `min_f64(a, b)` / `max_f64(a, b)` | 2 × `f64` | `f64` | Min / max |
| `clamp_i32(x, lo, hi)` | 3 × `i32` | `i32` | Clamp to `[lo, hi]` |
| `sin(x)` / `cos(x)` / `tan(x)` | `f64` | `f64` | Trig (libm) |
| `atan2(y, x)` | 2 × `f64` | `f64` | Two-arg atan |
| `cpu_count()` | — | `i32` | Logical CPUs (for `parallel for`) |

 For `pow_i32`, `sqrt_i32`, etc. use `stdlib/math.ny` or `import "stdlib/builtins_math.ny"` (`Math_max`, `Math_random`, …).

### Fixed arrays & `for … in` (no import)

| Syntax / method | On | Returns |
|-----------------|-----|---------|
| `arr[i]` | fixed array | `T` — zero-based index |
| `for i in 0..n` | half-open range | — |
| `for x in arr` | `[T; N]` array | element `T` |
| `for c in str` | `string` | `char` per byte |
| `arr.length()` / `arr.len()` | fixed array | `i32` |
| `arr.sort()` | `i32` / `f32` / `f64` array | new sorted copy |
| `arr.sort_by(cmp)` | `fn(T, T) -> i32` | new sorted copy (any element type) |

```ny
let nums = [10, 1, 2, 8, 5]
let sorted = nums.sort()
let by_num = items.sort_by((a, b) => a.number - b.number)
for n in sorted { print(n) }
print(nums[0])   // original unchanged
```

### Split lists (`.split` result)

| Syntax | Returns |
|--------|---------|
| `parts.length()` / `parts.len()` | `i32` part count |
| `for s in parts` | each `string` part |

### `Vec_i32` — growable `i32` vector

`import "stdlib/vec.ny"` (or auto-prelude). Handle type is `ptr` — **no** `.push()` method sugar; use free functions.

| Function | Args | Returns | Notes |
|----------|------|---------|-------|
| `Vec_i32_new()` | — | `ptr` | Empty vector |
| `Vec_i32_push(v, x)` / `vec_push(v, x)` | `ptr`, `i32` | `void` / `ptr` | Append; `vec_push` returns handle for chaining |
| `Vec_i32_get(v, i)` / `vec_get(v, i)` | `ptr`, `i32` | `i32` | Index (0-based) |
| `Vec_i32_set(v, i, value)` | `ptr`, `i32`, `i32` | `void` | In-place update |
| `Vec_i32_len(v)` / `vec_len(v)` | `ptr` | `i32` | Element count |
| `Vec_i32_pop(v)` | `ptr` | `i32` | Pop last |
| `Vec_i32_from_range(start, end)` | 2 × `i32` | `ptr` | Half-open range fill |
| `Vec_i32_free(v)` | `ptr` | `void` | Manual free (or auto-drop if owned) |

```ny
import "stdlib/vec.ny"

let v = Vec_i32_new()
Vec_i32_push(v, 10)
Vec_i32_push(v, 20)
print(Vec_i32_len(v))       // 2
print(Vec_i32_get(v, 0))    // 10
Vec_i32_set(v, 1, 99)
```

### `StrVec` — string vector with method syntax

`import "stdlib/vec_str.ny"` (or auto-prelude). Struct with `impl` methods — **supports** `.push()` / `.get()` / `.len()`.

| Method / function | Args | Returns | Notes |
|-------------------|------|---------|-------|
| `StrVec_new()` | — | `StrVec` | Empty list |
| `.push(value)` | `string` | `StrVec` | Append (method chaining) |
| `.get(index)` | `i32` | `string` | Indexed access |
| `.len()` | — | `i32` | Element count |
| `StrVec_from_lines(text)` | `string` | `StrVec` | Split on `\n` |
| `StrVec_from_argv(start_index)` | `i32` | `StrVec` | CLI args from index |
| `argv()` | — | `StrVec` | Shorthand: `StrVec_from_argv(1)` |
| `StrVec_join_lines(vec)` | `StrVec` | `string` | Join with `\n` |
| `Vec_string_*` aliases | — | — | Generic `Vec<string>` syntax maps here |

```ny
let lines = StrVec_from_lines("a\nb\nc")
print(lines.get(0))
let mut v = StrVec_new()
v = v.push("hello").push("world")
for s in v { print(s) }   // iterate when used as iterable in loops
```

### `HashMap` — string-keyed maps with method syntax

`import "stdlib/map.ny"` (or auto-prelude). Two monomorph types ship today:

| Type | Value type | Constructor |
|------|------------|-------------|
| `HashMap_str_i32` | `i32` | `HashMap_str_i32_new()` |
| `HashMap_str_str` | `string` | `HashMap_str_str_new()` |

| Method | Args | Returns | Notes |
|--------|------|---------|-------|
| `.insert(key, value)` | `string`, value | same map type | **Chains** — returns `self` |
| `.get(key)` | `string` | `i32` / `string` | Lookup (0 / `""` if missing — check with `.contains`) |
| `.contains(key)` | `string` | `i32` | `1` if key exists, else `0` |
| `.keys()` | — | `StrVec` | All keys |
| `.remove(key)` | `string` | same map type | Remove key; chains |

Low-level `ptr` API (FFI style): `map_str_i32_new`, `map_str_i32_insert`, `map_str_i32_get`, `map_str_i32_contains`, `map_str_i32_keys`, `map_str_i32_remove`, `map_str_i32_free`. Struct wrappers auto-call `Drop`.

```ny
import "stdlib/map.ny"

let scores = HashMap_str_i32_new()
    .insert("alice", 95)
    .insert("bob", 87)

print(scores.get("alice"))       // 95
print(scores.contains("bob"))    // 1

let keys = scores.keys()
for k in keys { print(k) }

let updated = scores.remove("bob")
```

Generic syntax `HashMap<K, V>` is Extended tier — monomorph names above are Core-stable.

### Timing & memory (no import)

```ny
time_start("label")
// ... work ...
time_end("label")    // prints elapsed (colored terminal output)

mem_start("label")
mem_end("label")     // prints RSS delta (platform-dependent)
```

### `spawn { }` / `spawn:task` / `spawn:thread` (Extended — no import keyword)

**Platform:** native Linux, macOS, Windows only — **`spawn` is a compile error on `wasm32-wasi`**. Requires runtime link (`nyra_rt`); rejected in `no_std`.

#### File directive: `allow_extended`

Put on the **first line** of a file (before `fn` / imports) when the unit uses Extended-tier APIs (`spawn`, `parallel for`, `async`, `defer`, …):

```ny
allow_extended
```

| What | Detail |
|------|--------|
| **Purpose** | Documents that this file intentionally uses **Stable Extended** features |
| **Effect today (v1.40+)** | Extended ships **without `warning[W001]`** in default builds — `spawn` compiles with or without the directive |
| **When skipped** | If `extended_tier_warnings` runs, files **without** `allow_extended` may get W001 for Extended syntax; files **with** it suppress W001 in that unit |
| **CI** | Pair with `nyra check --deny-extended` for Core-only gates (converts W001 → error when preview warnings return) |
| **Scope** | One line per **compilation unit** — not per-function |

**Not a compile switch:** `allow_extended` does **not** enable `spawn`; it declares intent and integrates with stability warnings. Omit it in Core-only tutorials.

#### Task pool vs OS thread

| Syntax | Backend | When to use |
|--------|---------|-------------|
| `spawn { }` | **Task pool** (default) | Many concurrent jobs — queued on global workers (~`cpu_count()`); queue cap 65k; cheap (bytes–KB bookkeeping) |
| `spawn:task { }` | **Task pool** (alias) | Same as bare `spawn` |
| `spawn:thread { }` | **OS thread** | Blocking I/O, isolation, or true 1:1 thread (~MB stack); `pthread` / `CreateThread` |

Captures must be **Send**; no `&` / `&mut` captures.

#### `JoinHandle` and `.join()`

| Form | Syntax | Waits? |
|------|--------|--------|
| **Statement** | `spawn { … }` | **No** — fire-and-forget; runtime detaches immediately |
| **Expression** | `let h = spawn { … }` | Returns opaque **`JoinHandle`** (not printable) |
| **Join** | `h.join()` | **Yes** — blocks caller until work finishes; **consumes** `h` (move; no second `.join()`) |
| **Drop** | `h` goes out of scope unused | **No** — same as statement form (detach) |

`.join()` — method on `JoinHandle`; signature `h.join() -> void`; **no arguments**. Codegen calls `spawn_task_join` or `spawn_join` depending on whether the handle came from `spawn`/`spawn:task` or `spawn:thread`.

```ny
allow_extended

fn main() {
    // Task pool (default) — output order: 99, then 0
    let h = spawn {
        print(99)
    }
    h.join()
    print(0)

    // Fire-and-forget — main does not wait
    spawn {
        print("background")
    }

    // OS thread when you need real thread isolation
    let t = spawn:thread {
        blocking_syscall()
    }
    t.join()
}
```

**Async note:** `async fn` desugar runs the state machine body on **`spawn:thread`** internally (blocking `async_await` inside spawn remains possible).

Channels: `stdlib/sync/channel.ny`

### `parallel for` / `parallel:task` / `parallel:thread` (Extended)

Each entry: **name → explanation → example → output**. Runnable: `examples/builtins/parallel/` · gallery: [methods.html#ex-parallel](https://nyra-lang.github.io/docs/methods.html#ex-parallel).

#### `parallel for` (task pool — default)

**Explanation:** Independent iterations on the global task pool (same workers as `spawn`). Fork-join — code after the loop runs when all iterations finish. Requires `allow_extended`.

```ny
allow_extended
fn main() {
    parallel for i in 0..4 {
        print(i)
    }
    print(999)
}
```

**Output** (lines `0`–`3` may appear in any order; `999` always last):

```
0
1
3
2
999
```

#### `parallel:task(max = N)`

**Explanation:** Explicit task-pool alias; `max = N` caps worker chunks. Prefer `max` over `max_threads` to avoid confusion with `:thread`.

```ny
parallel:task(max = 4) for i in 0..1000 { work(i) }
```

**Output:** Depends on `work(i)`; loop is fork-join before the next statement runs.

#### `parallel:thread(max = N)`

**Explanation:** OS-thread fork-join per chunk. Backend is `:thread`, not the `max` key.

```ny
parallel:thread(max = 4) for i in 0..1000 { work(i) }
```

**Output** (with `print(i)` inside; order non-deterministic):

```
2
3
0
1
999
```

#### Worker options

| Option | Meaning |
|--------|---------|
| *(none)* | Task pool, `mode = auto`, workers from CPU count |
| `parallel:thread` / `backend = thread` | OS thread backend |
| `max = N` | At most N workers |
| `threads = N` | Exactly N workers |
| `cpu = P%` | P percent of logical CPUs |
| `mode` | `auto`, `balanced`, `max_performance`, `background` |

**Rules:** no `break`; no outer mutation; captures must be **Send**; range, fixed array, `string`, or `vec_str`. On `wasm32-wasi`, runs sequentially.

Gallery also covers: [`parallel(threads = N)`](methods.html#ex-parallel-exact) · [`parallel for n in array`](methods.html#ex-parallel-array) · [`progress for`](methods.html#ex-progress).



### `progress for` (Extended)

**Name:** `progress(label = "…") for x in items { … }`  
**Explanation:** Sequential progress bar; cannot combine with `parallel for`.  
**Example:** see [methods.html#ex-progress](https://nyra-lang.github.io/docs/methods.html#ex-progress)

```ny
allow_extended
progress(label = "demo") for i in 0..3 {
    print(i)
}
```

**Output:**

```
[#####--------] 33%
Running demo...
…
0
1
2
```



### `benchmark { }` (Extended)

**Name:** `benchmark { … }`  
**Explanation:** Wall time, RSS delta, and process CPU% for a block — no manual timers.  
**Example:** [methods.html#ex-benchmark](https://nyra-lang.github.io/docs/methods.html#ex-benchmark) · `nyra run examples/builtins/benchmark/benchmark.ny`

```ny
allow_extended

extern fn blackbox_i32(x: i32) -> i32

fn main() {
    benchmark {
        let mut acc = 0
        for i in 0..10000 {
            acc = blackbox_i32(acc + i)
        }
        blackbox_i32(acc)
    }
}
```

**Output:**

```
Time: 0.1 ms
Memory: 0.0 B
CPU: 98%
```

(Varies by machine.)



## Async & await (Stable Extended)

Gallery: [methods.html#ex-async-fn](https://nyra-lang.github.io/docs/methods.html#ex-async-fn) · Runnable: `examples/builtins/async/`

Prefer `import "stdlib/async/mod.ny"` for application code. It is Nyra's official in-tree runtime facade (`NyraRuntime_default`, `NyraRuntime_run_until`, `sleep_ms_async`, `await_i32`), so apps do not need a Tokio-like community executor for basic async tasks.

```ny
import "stdlib/async/mod.ny"

fn main() {
    let rt = NyraRuntime_default()
    let f = sleep_ms_async(20)
    let value = match NyraRuntime_run_until(rt, f.handle, 1000) {
        Result.Ok(v) => v
        Result.Err(err) => {
            Error_print(err)
            0
        }
    }
    print(value)
}
```

#### `async fn` + `await`

**Explanation:** Call returns handle immediately; body on `spawn:thread`. Import `stdlib/async_v1.ny` for executor.

```ny
allow_extended
import "stdlib/async_v1.ny"

async fn compute() -> i32 {
    return 42
}

fn main() {
    print(await compute())
}
```

**Output:** `42`

#### State machine (multiple `await`)

**Output:** `100` — see `#ex-await` · `async_state_machine.ny`

#### `Future<T>` (v1.26)

**Output:** `Nyra async v2` — see `#ex-async-future`

| Topic | Behavior |
|-------|----------|
| **`async fn` desugar (v1.5)** | Body runs on **`spawn:thread`**; call site gets promise handle immediately |
| **State machine (v1.6–v1.7)** | Top-level `await` in `async fn`; **`await` inside `if` / `while` / range `for`** |
| **`await` in `spawn` / `unsafe`** | Uses blocking `async_await` — not cooperative |
| **Futures (v1.26)** | `import "stdlib/async/future.ny"` — `Future_i32`, `Future_select2_i32(a, b)` |

Not on `wasm32-wasi`. Full guide: [async.html](https://nyra-lang.github.io/docs/async.html)



## Concurrency & sync primitives

Beyond `spawn { }` / `spawn:task` / `spawn:thread` and `parallel for` (see [I/O & builtins](#io--builtins)):

### Send / Sync (spawn captures)

| Rule | Detail |
|------|--------|
| **`spawn` captures** | Must be **Send**; **no `&` / `&mut` captures** |
| **`JoinHandle`** | **Send**, not **Sync** (move between threads; do not share by reference) |
| **Shared refs across threads** | Inner type must be **Sync** |
| **Active borrows** | Rejected in closure env |

Full rules: [Send/Sync](https://nyra-lang.github.io/docs/memory.html#send-sync) · [concurrency](https://nyra-lang.github.io/docs/concurrency.html)

### Channels — `import "stdlib/sync/channel.ny"`

| Type | Methods | Payload |
|------|---------|---------|
| `Channel_i32` | `.send(i32)`, `.recv() -> i32` | `i32` |
| `Channel_str` | `.send(string)`, `.recv() -> string` | `string` |

Low-level: `channel_new`, `channel_send`, `channel_recv`, `channel_free` (C runtime `stdlib/rt/rt_channel.c`).

### Mutex / RwLock / WaitGroup / Atomic

```ny
import "stdlib/sync/mutex.ny"
import "stdlib/sync/rwlock.ny"
import "stdlib/sync/waitgroup.ny"
import "stdlib/sync/atomic.ny"
```

| Type | API |
|------|-----|
| `Mutex` | `.lock()`, `.unlock()` |
| `Mutex_i32` | legacy wrapper around `Mutex` |
| `RwLock` / `RwLock_i32` | read/write lock wrappers |
| `WaitGroup` | `.add(delta)`, `.done()`, `.wait()` |
| `Atomic_i32` | `.load()`, `.store(v)`, `.fetch_add(n)` |
| `AtomicBool` | `.load()`, `.store(bool)` |

### Runtime concurrency symbols (C)

| Symbol | Role |
|--------|------|
| `spawn_capture` | OS thread (`spawn:thread`); returns `JoinHandle` |
| `spawn_join` / `spawn_handle_drop` | Join / detach OS thread handle |
| `spawn_task_capture` | Task pool (`spawn` / `spawn:task`); returns `JoinHandle` |
| `spawn_task_join` / `spawn_task_handle_drop` | Join / fire-and-forget task handle |
| `parallel_for_range` | Fork-join `parallel for` — task pool (default) or OS thread chunks |
| `progress_update` / `progress_finish` | Progress bar |

Not on `wasm32-wasi` (sequential stub).

## Stdlib-style helpers (import required)

Ergonomic **function-style** wrappers — use when you prefer JS-like names over `ptr` handles or method syntax.

### `stdlib/builtins_array.ny` — `Vec_i32` helpers

```ny
import "stdlib/builtins_array.ny"
```

| Function | Description |
|----------|-------------|
| `Array_push(v, x)` | Append `i32` |
| `Array_pop(v)` | Pop last `i32` |
| `Array_map(v, f)` | Map with `fn(i32) -> i32` |
| `Array_filter(v, pred)` | Filter (`pred` returns `1`/`0`) |
| `Array_reduce(v, init, f)` | Fold left |
| `Array_find(v, pred, fallback)` | First match or fallback |

### `stdlib/builtins_string.ny` — string helpers

```ny
import "stdlib/builtins_string.ny"
```

| Function | Description |
|----------|-------------|
| `String_toUpperCase(s)` / `String_toLowerCase(s)` | ASCII case |
| `String_includes(s, needle)` | Substring test → `i32` `1`/`0` |
| `String_split(s, sep)` | Split → `ptr` string vector |
| `String_replace(s, from, to)` | Replace all matches |
| `String_replacen(s, from, to, count)` | Replace at most `count` matches |
| `trim(s)` | Strip whitespace |

Prefer built-in `.split()` / `.trim()` on `string` when you do not need the import.

### Random — compiler builtins (no import)

`random()` / `random(min, max)` and `random_f64()` / `random_f64(min, max)` are **compiler builtins** (ChaCha20 CSPRNG in `stdlib/rt/rt_random.c`). **No import required.**

| Call | Returns | Notes |
|------|---------|-------|
| `random()` | `i32` by default | Full-range integer |
| `random(min, max)` | **Generic integer** | Return type follows bounds (`i32`, `i64`, `u64`, …); inclusive range; rejection sampling (no modulo bias) |
| `random<T>()` / `random<T>(min, max)` | `T` | Explicit type when inference is ambiguous |
| `random_f64()` | `f64` | Unit interval `[0, 1)` — 53-bit precision |
| `random_f64(min, max)` | `f64` | Half-open `[min, max)` |

**Removed (v1.39):** `Random()` alias and `random_range()` as public API — use `random()` / `random(min, max)` instead.

Seeding: OS/hardware entropy (`getentropy`, `arc4random`, `BCryptGenRandom`, `RDRAND` when available). Raw TRNG bytes: `stdlib/os/hw_crypto.ny` → `hw_random_bytes`.

### `stdlib/random.ny` — shuffle helper (import required)

```ny
import "stdlib/random.ny"
```

| Function | Description |
|----------|-------------|
| `shuffle_pick(vec)` | Random element from an `i32` vector handle |

The module re-exports the same ChaCha20 runtime; **`random()` itself is a builtin** — import only for `shuffle_pick`.

### `stdlib/builtins_math.ny` — JS-style math

```ny
import "stdlib/builtins_math.ny"
```

| Function | Description |
|----------|-------------|
| `Math_max(a, b)` / `Math_min(a, b)` | Min / max (`i32`) — wraps `max_i32` / `min_i32` |
| `Math_round(x)` / `Math_floor(x)` / `Math_ceil(x)` | Rounding (MVP on `i32`) |
| `Math_random()` | Random `f64` in `[0, 1)` — ChaCha20 via `rand_f64()` |

### `stdlib/builtins_json.ny` — MVP JSON helpers

```ny
import "stdlib/builtins_json.ny"
```

| Function | Description |
|----------|-------------|
| `JSON_stringify(key, value)` | Single-field JSON object string |
| `JSON_parse(json, key)` | Read string field from JSON |

For full JSON/serde use `stdlib/json/mod.ny`, `stdlib/serialize/mod.ny`, or NyraPkg `ny-serde`.



## Ownership (summary)

Nyra has **no GC**. The compiler builds a **DropPlan** per function and emits `free` / custom `Drop_*_drop` at scope exit. [Memory guide](https://nyra-lang.github.io/docs/memory.html) · [learn ownership](https://nyra-lang.github.io/docs/learn-ownership.html) · [learn borrowing](https://nyra-lang.github.io/docs/learn-borrowing.html).

### Copy vs Move

| Kind | Types | On assign / pass | Scope end |
|------|-------|------------------|-----------|
| **Copy** | all integer types, `f64`, `char`, `bool`, enum tags, `ptr`, fn ptr | Both bindings valid | Stack discard |
| **Move** | heap `string`, struct with move field or `impl Drop` | Source invalidated | Auto `free` or `Drop_*_drop` |

```ny
let a = "hello"
let b = a          // move — a invalid
print(b)
// print(a)       // ERROR: use of moved value
```

### Rules

1. **One owner** per heap value — cleanup follows ownership.
2. **Move by default** for `string` unless borrowing with `&` / `&mut`.
3. **No use after move** — borrowck tracks moves; moved bindings skipped in DropPlan (no double-free).
4. **Owned extern returns** — `read_file`, `strcat`, `sys_recv`, … → caller owns result; auto-dropped at scope end.
5. **NLL borrow** — `&x` / `&mut x` end at **last use** of the ref, not at `}`.
6. **Cannot return `&local`** — return owned `string` or borrow from a parameter lifetime.
7. **Auto-borrow at calls** — `f(user)` → `f(&user)` when callee expects `&T`.
8. **Auto-Copy** — structs with only Copy fields are Copy automatically (RFC 0008); `#[derive(Copy)]` documents/validates.
9. **Clone** — explicit (`clone user` / `.clone()`); synthesized for `string` and cloneable structs.
10. **`defer`** — optional scope-exit hook (Extended). **For memory cleanup, use auto-drop or `impl Drop` instead** — see [defer vs Drop](#defer-vs-drop--when-to-use-which).
11. **`spawn` / closures** — no `&` captures; move types must be **Send**; parent binding marked moved.

### Borrowing examples

```ny
fn read_len(s: &string) -> i32 {
    return s.length()
}

fn main() {
    let name = "Ada"
    print(read_len(name))     // auto-borrow &name
    print(name.length())    // method borrows — name still valid

    let mut count = 0
    let r = &mut count
    print(*r)               // read through mut ref
    count = count + 1       // mutate binding directly

    let a = "hello"
    let b = a                 // move — a invalid after
    print(b)
    // print(a)              // E012 use after move
}
```

**String concat:** runtime uses `strcat(a, b)` (moves args unless borrowed). Comptime allows `+` on string literals. Before `strcat(a, …)` reuse `a` with `clone a` or `a.clone()`.

**Float literals:** `3.14` → `f64`; `1.5f32` or `: f32` annotation for `f32`.

### defer vs Drop — when to use which

Gallery: [methods.html#ex-defer](https://nyra-lang.github.io/docs/methods.html#ex-defer) · `examples/builtins/defer/`

**Name:** `defer cleanup()`  
**Example:**

```ny
allow_extended
fn cleanup() { print(1) }
fn main() {
    defer cleanup()
    return
}
```

**Output:** `1`

**Short answer:** **`Drop` (auto-drop + `impl Drop`) covers almost every cleanup case.** Keep `defer` in **Extended** tier — niche escape hatch, not the default path.

| Goal | Preferred (Core / ownership) | `defer` (Extended) |
|------|------------------------------|---------------------|
| Free heap `string` at `}` | **Auto-drop** — nothing to write | ❌ **Do not** `defer free(x)` — compiler warns (double-free risk) |
| Struct with heap fields | **Composite auto-drop** at scope end | ❌ Redundant |
| Socket / file / FFI handle | **`impl Drop for Wrapper { ... }`** (RAII) | ⚠️ One-off `defer extern_close(h)` only if you skip a wrapper struct |
| Log / metric at scope exit | Normal code before `}` or inside `Drop` | `defer log_done()` — side effect only; not memory |
| LIFO order of several cleanups | Declare wrappers; drops run in reverse binding order | Multiple `defer` lines (LIFO) — same idea as Go |

**Why `defer` stays Extended (not Core):**

1. **Overlap with Drop** — if `impl Drop` + auto-drop cover your cleanup, `defer` adds no capability Core needs.
2. **Discouraged pattern** — `defer free(x)` duplicates auto-drop; typechecker emits manual-free warnings.
3. **Semantics still evolving** — general `defer call(...)` lowering is not fully on par with `defer free(ptr)` in codegen; treat as preview.
4. **Core-only CI** — `nyra check --deny-extended` assumes you rely on auto-drop, not scope-exit hooks.

**When `defer` still makes sense (Extended only):**

- One-shot **FFI teardown** (`defer gzclose(f)`) in a short function where a RAII wrapper feels heavy.
- **Non-cleanup side effects** at block exit (logging, counters) — rare; often clearer to write before `return` / at end of block.

**When to use Drop instead (recommended):**

```ny
struct GzFile { handle: ptr }

impl Drop for GzFile {
    fn drop(self) {
        unsafe { gzclose(self.handle) }
    }
}

fn read_gz(path: string) -> string {
    let f = GzFile { handle: gzopen(path, "rb") }
    // auto gzclose at `}` — no defer
    return slurp(f.handle)
}
```

Reusable resources, predictable order, no `warning[W001]` from `defer` — **prefer this over `defer`**.

**Roadmap note:** If Core users need FFI teardown without Extended `impl Drop`, promoting **`defer` to Core** could be reconsidered. Today both are Extended; **Drop-first documentation avoids needing `defer` in Core-only codebases.**

See [defer vs Drop](https://nyra-lang.github.io/docs/memory.html#defer) · [custom Drop](https://nyra-lang.github.io/docs/memory.html#custom-drop)

### Copy vs Move (RFC 0008)

Scalars and all-Copy structs (`Point`, `Rect`, `Color`) copy on assign — both bindings stay valid. Structs with `string` or custom `Drop` move. No annotation required for auto-Copy; use `#[derive(Copy)]` only to document or validate.

### Type inference (RFC 0006)

| Site | Rule |
|------|------|
| `let x = expr` | Infer from RHS |
| `fn f() { ... }` without `->` | `void` if no `return`; else unify all `return` types |
| `fn f(mgr) { return mgr.active_id }` | Return field type from receiver struct when body is a direct field access |
| `dir == SplitDirection.Horizontal` | Enum parameter from variant comparison or call site (`SplitDirection.Vertical`) |
| `line.split(" ")` | String receiver wins over `*_split` struct methods (e.g. `LayoutManager_split`) |
| `id(x)` on generic `fn id<T>(...)` | Infer `T` from argument; monomorph to `id__T` |
| Ambiguous | Error asks for explicit annotation, e.g. `let x: User = ...` |

### Struct constructor sugar

When `Name` is a struct (not a function), `Name(a, b)` desugars to a struct literal with positional fields; missing trailing fields use zero defaults (`User()` → `age: 0`, `name: ""`).

```ny
struct User {
    name: string
    age: i32
}
let u = User("Ada")   // User { name: "Ada", age: 0 }
```

### Spread operator `...` (Extended)

JS-style spread with **three dots** (`...`). Rust-style **two dots** (`..`) still works in struct literals.

**Array literals** — copy elements from fixed-size arrays, or **field values** from objects (like `Object.values` in JS):

```ny
let nums = [1, 2, 3]
let more = [...nums, 4, 5]   // [1, 2, 3, 4, 5]

let row = { x: 10, y: 20 }
let flat = [...row, 30]      // [10, 20, 30] — struct fields in declaration order
```

Structs cannot be inserted as array elements directly (`[obj]` is an error). Use spread: `[...obj]`.

**Object / struct literals** — copy fields; later spreads and explicit fields override earlier ones:

```ny
let user = { name: "Alex", role: "Admin" }
let updated = { ...user, role: "Editor" }

struct Profile { name: string, role: string, theme: string }
let merged = Profile { ...user, theme: "dark" }
```

 (zero-types) and `spread_operator.typed.ny`. Object spread into arrays requires **compatible scalar field types** (same element type as the rest of the array). Object spread in `{ ...obj }` requires a **struct** value.

### Struct spread (Extended)

Copy fields from one or more struct values into a **named** target struct. Later spreads and explicit fields override earlier ones (like JS object spread). Prefer `...expr` or `..expr` interchangeably.

```ny
struct User {
    name: string
    role: string
}
struct Settings {
    theme: string
    notifications: bool
}
struct Profile {
    name: string
    role: string
    theme: string
    notifications: bool
}

let user = User { name: "Alex", role: "Admin" }
let settings = Settings { theme: "dark", notifications: true }
let merged = Profile { ...user, ...settings }

// Update one field on the same struct type:
let p = Pair { a: 1, b: 2 }
let q = Pair { ...p, b: 9 }
```

 Named struct targets use `Type { ...spread }`. Anonymous `{ ...spread, key: value }` also works when fields are inferred (see above).

### Auto-borrow example

```ny
fn save(u: &User) -> void { print(u.name) }
fn main() {
    let user = User { name: "Ahmed", age: 25 }
    save(user)       // → save(&user)
    print(user.name) // OK
}
```

### Prefix syntax (RFC 0007)

| Call | Meaning |
|------|---------|
| `save(user)` | auto-borrow when callee expects `&T` |
| `save(clone user)` | duplicate then pass (`user` stays valid) |
| `save(move user)` | explicit move; skips auto-borrow |

Use-after-move errors name the callee and line, show the function signature, and suggest `&`, `clone`, or `move` fixes.

```ny
// error: `user` was moved into `save()` at line 10
// note: keep using `user`: save(clone user)
```

### Leak prevention (normal code)

- Every owned `let` still in scope gets dropped on all paths (`return`, block end, branch merge).
- **Composite structs** (v2.3): field-wise `free` for `string` fields without manual `impl Drop`.
- **`extern fn ... -> string`** (v2.3): auto-detected as owned returns — no whitelist needed.
- Moving to a function transfers cleanup to the callee.
- Escaping closures (v2.2 heap env) register `heap_owned` — freed when the `let` binding ends.
- **Not automatic:** intentional FFI leaks, manual `free` on live bindings, raw-pointer cycles.

### Common errors

| Error | Fix |
|-------|-----|
| `Use of moved value` | Borrow with `&`, use `.clone()` if `Clone`, or take `&T` in callee signature (auto-borrow applies at call) |
| `Cannot borrow as mutable` | End first borrow (NLL) before second use |
| `cannot return reference to local` | Return owned value or `&'a` from parameter |
| `cannot capture reference in closure` | Capture owned Copy/Move value |

## Stdlib (modular — see https://nyra-lang.github.io/docs/stdlib.html)

> **Batteries-included by design:** Nyra’s stdlib is **strong** — crypto, databases, serialization, WebSocket, compression, and encoding belong **in-tree** with the compiler. Some modules are still **stubs or MVP** while native implementations land in `stdlib/rt/`; import paths stay stable. **NyraPkg** complements stdlib for community and optional packages — it does not replace core domains. Status inventory: [stdlib](https://nyra-lang.github.io/docs/stdlib.html) · [status](https://nyra-lang.github.io/docs/stdlib.html#status).

### What ships vs what is in progress

| Status | Modules | Notes |
|--------|---------|-------|
| **Shipped** | `vec.ny`, `vec_str.ny`, `map.ny`, `collections/*`, `strings/ops.ny`, `fs/mod.ny`, `path.ny`, `crypto/mod.ny`, `encoding/base64.ny`, `net/tcp.ny`, `net/http/mod.ny`, `net/udp.ny`, `net/websocket.ny`, `compress/mod.ny`, `serialize/mod.ny`, `json/mod.ny`, `db/sqlite.ny`, `tls.ny`, `time/*`, `strconv/mod.ny`, `flag/mod.ny`, `bufio/mod.ny`, `context/mod.ny`, `sync/mod.ny`, `process.ny` (POSIX), `bridge/mod.ny`, `terminal/*`, `encoding/csv.ny`, `archive/zip.ny`, `mime/mod.ny`, `random_bytes`, `embed/mod.ny`, `slog/mod.ny`, `testing/fstest.ny`, `testing/quick.ny` | Collections, FS, crypto, HTTP/TCP/UDP/WS, CLI, DB (SQLite), sync |
| **MVP / partial** | `json/mod.ny` / `serialize/mod.ny` (multi-key encode; not full schema serde), `uuid/mod.ny`, `url` helpers, `async.ny`, `reflect/mod.ny` | Use NyraPkg (`ny-serde`) for full schema serde |
| **Native when linked** | `db/postgres.ny` (`link pq`), `db/mysql.ny` (`link mysqlclient`), `compress/bzip2.ny` (`link bz2`) |
| **Shipped (v1.1)** | `env_set`, `process` (POSIX + Windows), Windows prebuilt releases |
| **Stub → in progress** | `compress/bzip2.ny` (link `bz2`) | Native driver when linked |

Tell users Nyra **targets** production crypto, SQLite, WebSocket, and full serde **in stdlib**. Where a module is still a stub, say so honestly — do not redirect to NyraPkg as the primary path. See [Standard library](https://nyra-lang.github.io/docs/stdlib.html).

### Naming: current style vs legacy (read this)

Nyra uses **monomorph names** in Core stdlib and **generic syntax** in Extended tiers. Both compile; prefer the **Current** column for new code and `--deny-extended` CI.

| API | Current (use this) | Legacy / alternate | Notes |
|-----|-------------------|-------------------|-------|
| Growable `i32` vector | `Vec_i32_new()`, `Vec_i32_push(v, x)`, `Vec_i32_len(v)` | `Vec<T>` generic syntax (Extended) | Handle type is `ptr`; free with `Vec_i32_free(v)` or scope end if owned |
| String-key map | `HashMap_str_i32_*`, `HashMap_str_str_*`, `Map_str_i32_*` in `map.ny` | `HashMap<K,V>` (Extended) | **Method chaining:** `.insert().insert()` · `.get` · `.contains` · `.keys()` · `.remove()` |
| String vector | `StrVec`, `StrVec_from_argv`, `StrVec_from_lines` in `vec_str.ny` | `Vec_str_*` low-level `ptr` API | CLI args, JSON keys, line lists |
| Heap single owner | `import "stdlib/box.ny"` → `Box<string>`, `Box_new(value)` | `Box_string` (v2.3 changelog name) | `Box<T>` monomorph; today `Box_new` ships for `string` |
| Shared ownership | `import "stdlib/arc.ny"` → `Arc<i32>`, `Arc<string>`, `Arc_from_i32`, `Arc_from_string`, `Arc_get_applied_i32` | `Arc_i32`, `Arc_new_i32`, `Arc_clone_i32` (v2.3 struct + manual `impl Drop`) | Legacy `Arc_i32` API remains in `arc.ny` for backward compat |
| Optional / errors | `import "stdlib/option.ny"` → `Option<T>`, `Result<T,E>` | `Option_i32`, `Result_i32_i32` in `stdlib/result.ny` | Prefer generic `option.ny`; `result.ny` is older explicit monomorph helpers |
| Option tags only | built-in `Option.None` / `Option.Some` (no args) | — | For `??` / `?.` desugar only; not storage |

**Rule of thumb:** If you see `Foo_bar_baz` (underscore monomorph), that is the **stable Core stdlib surface**. If you see `Foo<T>` in source, it is **generic Extended** — compiler emits `Foo__T` (or similar) at compile time.

### Vec example (current Core idiom)

```ny
import "stdlib/vec.ny"

fn main() {
    let v = Vec_i32_new()
    Vec_i32_push(v, 1)
    Vec_i32_push(v, 2)
    print(Vec_i32_len(v))
    print(Vec_i32_get(v, 0))
    Vec_i32_free(v)   // or rely on auto-drop if ownership tracks the handle
}
```

Do **not** write `v.push(1)` on `Vec_i32` — `ptr` handle, not method-chaining object. Use `Vec_i32_push(v, x)` or `import "stdlib/builtins_array.ny"` helpers (`Array_push`, `Array_map`, …). For string lists use `StrVec` which **does** support `.push()`.

### HashMap example (method chaining)

```ny
import "stdlib/map.ny"

fn main() {
    let cache = HashMap_str_str_new()
        .insert("theme", "dark")
        .insert("lang", "en")

    print(cache.get("theme"))
    print(cache.contains("lang"))

    let keys = cache.keys()
    for k in keys { print(k) }

    cache = cache.remove("lang")
}
```

### Arc / Box examples (Extended — generic syntax)

```ny
import "stdlib/arc.ny"
import "stdlib/box.ny"

fn main() {
    let b = Box_new("hello")           // Box<string>
    let a = Arc_from_i32(42)           // Arc<i32> — preferred
    print(Arc_get_applied_i32(a))

    // Legacy v2.3 (still compiles — avoid in new code):
    // let old = Arc_new_i32(42)
}
```

See [stdlib](https://nyra-lang.github.io/docs/stdlib.html)

```ny
import "stdlib/vec.ny"
import "stdlib/map.ny"
import "stdlib/strings/ops.ny"
```

**Stdlib auto-prelude (lazy):** Referenced stdlib symbols resolve on demand via a virtual symbol table — use `read_file`, `Vec_i32_new`, `StrVec`, `HashMap_str_i32_new`, `os_arg_count`, `os_arg_at`, `list_dir`, `is_dir`, `env_get`, etc. without imports; only used modules are merged into the build. Opt out with `# no_std` or `--no-prelude`. Explicit `import "stdlib/vec.ny"` still works.

**Common auto-prelude symbols (no import when prelude enabled):**

| Domain | Functions |
|--------|-----------|
| **FS** | `read_file`, `read_file_limit`, `write_file`, `append_file`, `file_exists`, `exists`, `is_dir`, `list_dir`, `list_dir_entries`, `create_dir`, `create_dir_all`, `remove_file`, `remove_dir`, `copy_file`, `file_size` |
| **CLI / env** | `os_arg_count`, `os_arg_at`, `argv`, `env_get`, `env_set`, `env_has` |
| **Collections** | `Vec_i32_*`, `vec_*`, `StrVec_*`, `HashMap_str_i32_*`, `HashMap_str_str_*` |
| **Strings** | `strcat`, `strlen`, `substring`, `strstr_pos` (via `stdlib/strings.ny` chain) |
| **Crypto** | `sha256`, `hmac_sha256`, … (`stdlib/crypto/mod.ny`) |
| **Net** | `tcp_listen`, `tcp_accept`, … (`stdlib/net/tcp.ny`) |

**Compiler math intrinsics (always on):** `abs`, `abs_i32`, `abs_f64`, `min_i32`, `max_i32`, `clamp_i32`, `min_f64`, `max_f64`, `sin`, `cos`, `tan`, `atan2`, and typed `abs(x)` lower to LLVM — no stdlib merge required.  with `--no-prelude`.

**Core modules (usable):** `vec.ny`, `vec_str.ny`, `map.ny`, `collections/*`, `strings/ops.ny`, `strings/regex.ny`, `fs/mod.ny`, `path.ny`, `crypto/mod.ny`, `encoding/base64.ny`, `time/instant.ny`, `time/date.ny`, `json/mod.ny`, `serialize/mod.ny`, `iter/mod.ny`, `env/mod.ny`, `config/mod.ny`, **`net/http/mod.ny`**, `net/tcp.ny`, `net/udp.ny`, `net/websocket.ny`, `tls.ny`, `strconv/mod.ny`, `flag/mod.ny`, `bufio/mod.ny`, `context/mod.ny`, `sync/mod.ny`, `process.ny`, `bridge/mod.ny`, `db/sqlite.ny`, `db/lsm.ny`, `db/sql_parse.ny`, `db/sstable.ny`, `collections/btree_pages.ny`, `bench/mod.ny`, `profile/mod.ny`, `testing.ny`, `async.ny` (Extended). [Stdlib reference](https://nyra-lang.github.io/docs/stdlib.html) (`#cli-parsing`, `#database`, `#process`, `#crypto`).

### Database quick start (v1.21)

```ny
import "stdlib/db/sqlite.ny"
import "stdlib/db/lsm.ny"
import "stdlib/db/sql_parse.ny"
import "stdlib/collections/btree_pages.ny"

fn main() {
    let db = Sqlite_open(":memory:")
    db.exec("CREATE TABLE kv (k TEXT, v TEXT)")
    let stmt = db.prepare("SELECT v FROM kv WHERE k = 'a'")
    while stmt.step() == 1 { print(stmt.col(0)) }
    stmt.finalize()
    db.close()

    let mut tree = LsmTree_new("data")
    tree = LsmTree_put(tree, "key", "value")
    let hit = LsmTree_lookup(tree, "key")
    tree = hit.tree
    print(hit.value)

    let mut btree = BTreePaged_new()
    btree = BTreePaged_insert(btree, "a", "1")

    let ast = SqlParse_parse("SELECT name FROM users WHERE id = 1")
    print(SqlParse_format(ast))
    let upd = SqlParse_parse("UPDATE users SET active = 1 WHERE id = 1")
    print(SqlParse_format(upd))

    let range = BTreePaged_range(btree, "a", "z")
    print(range.keys.len())
}
```

Requires `link sqlite3` in `nyra.mod` for SQLite. LSM/B-tree/SQL parser are pure Nyra stdlib.

**Shipped (v1.1):** `env_set`, `process` on Windows, postgres/mysql native when linked. **NyraPkg** for full serde: `ny-serde`, `ny-toml`. [Stdlib reference](https://nyra-lang.github.io/docs/stdlib.html).

### net/http API reference (v1.2+)

`import "stdlib/net/http/mod.ny"` — **canonical names:** `HttpRouter`, `HttpRouter_*`, `serve_handlers` (older docs may say `Router_*` / `listen_and_serve_*` — prefer `HttpRouter_*` in new code).

**Method constants:** `METHOD_GET`, `METHOD_POST`, `METHOD_PUT`, `METHOD_DELETE`, `METHOD_PATCH`, `METHOD_HEAD`, `METHOD_OPTIONS`.

#### Router & server

| Function | Description |
|----------|-------------|
| `HttpRouter_new()` | Empty router |
| `HttpRouter_register(router, method, path, tag)` | Static response tag string |
| `HttpRouter_register_slot(router, method, path, slot)` | Handler slot id |
| `HttpRouter_match_slot(router, ctx)` | Resolve slot (`-1` missing) |
| `serve_handlers(host, port, max_requests, router, handler)` | `handler(slot, ctx) -> HttpResponse` |
| `serve_loop(host, port, max_requests)` | Builtin loop |
| `serve_once(host, port, body)` | Single request |

#### Responses & client

| Function | Role |
|----------|------|
| `response_ok_json(body)` | 200 JSON |
| `response_created_json`, `response_not_found`, … | Status helpers |
| `get(url)` / `fetch(url)` | HTTP GET |
| `post`, `put`, `patch`, `delete` | Verbs → `HttpResponse` |

```ny
import "stdlib/net/http/mod.ny"

fn health_slot(slot: i32, ctx: RequestContext) -> HttpResponse {
    return response_ok_json("{\"status\":\"ok\"}")
}

fn main() {
    let mut router = HttpRouter_new()
    router = HttpRouter_register_slot(router, METHOD_GET, "/health", 0)
    serve_handlers("127.0.0.1", 8080, 100, router, health_slot)
}
```

[net/http reference](https://nyra-lang.github.io/docs/net-http.html)

### TCP, WebSocket, crypto, serde (quick API)

| Module | Key APIs |
|--------|----------|
| `stdlib/net/tcp.ny` | `tcp_listen`, `tcp_accept`, `tcp_connect`, `tcp_read`, `tcp_write` |
| `stdlib/net/websocket.ny` | `WebSocket_connect`, `ws_listen_on`, `.send`, `.recv` |
| `stdlib/crypto/mod.ny` | `sha256`, `hmac_sha256`, `sha512` (submodules) |
| `stdlib/serde/mod.ny` | `trait Serialize` / `Deserialize`; `{Struct}_json_encode/decode` |
| `stdlib/flag/mod.ny` | `FlagSet_new`, `Flag_parse`, `.verbose()`, `.help()` |
| `stdlib/strconv/mod.ny` | `atoi`, `itoa`, `parse_f64`, `format_f64` |
| `stdlib/bufio/mod.ny` | `Scanner_new`, `Scanner_scan`, `ReadLine` |
| `stdlib/iter/mod.ny` | `iter_filter`, `iter_map`, `vec_reduce_sum` |
| `stdlib/process.ny` | `exec(program, args)`, `Command` |
| `stdlib/collections/set.ny` | `HashSet_str` — `.insert`, `.contains` |

**Low-level runtime** (still valid): `read_file`, `vec_i32_*`, `map_str_i32_*`, `channel_*`, `bridge_exec`, `spawn { }`, `spawn:thread { }`, `h.join()`.

Crypto, SQLite, WebSocket, gzip, and full serde are **stdlib domains** — native implementations in `stdlib/rt/`; NyraPkg remains for community extensions.

## NyraPkg (packages)

Install third-party Nyra code + native link metadata into `.nyra/cache/`:

```bash
nyra pkg init
nyra pkg install ny-sqlite@^0.1.0
import "pkg/ny-sqlite"
```

**`nyra.mod` example:**

```text
module myapp.local
version 1.0.0
require ny-sqlite ^0.1.0
link sqlite3
link-source vendor/shim.c
```

| Source | How |
|--------|-----|
| Registry name | `nyra pkg install ny-sqlite@^0.1.0` — default `http://127.0.0.1:9470` (`~/.nyra/config`) |
| Git URL | `require https://github.com/you/ny-lib` |
| Bundled dev copy | NyraPkg registry / `nyra pkg install` |

- **`link`** / **`link-arg`** merge into project `nyra.mod` on install.
- **`link-source`** compiles package `.c` files at `nyra build` (no manual `clang`).
- Lock: `nyra.lock` + `nyra.sum` pin exact versions; `nyra pkg verify` checks constraints.
- **`nyra pkg prune`** — auto-fix unused code (like `cargo fix` for lint warnings). See [NyraPkg prune](https://nyra-lang.github.io/docs/packages.html#prune).
- Native C libraries (e.g. `-lsqlite3`) must exist on the system; NyraPkg ships bindings + shims, not OS packages.

### `nyra pkg prune` (unused code cleanup)

Removes dead imports and prefixes unused locals. Similar to **`cargo fix`** for Nyra lint warnings.

```bash
nyra pkg prune              # apply fixes in current project
nyra pkg prune --check      # dry run — report only, exit 1 if fixes needed
nyra pkg prune --path ./myapp
```

| Lint | Action |
|------|--------|
| **W002** unused import | Removes the entire `import "…"` line |
| **W003** unused variable | Prefixes the name with `_` (e.g. `let dead` → `let _dead`) |

Prefixing is safer than deleting `let` statements when the initializer might have side effects.

**Before:**

```ny
import "src/unused.ny"
fn main() {
    let dead = 99
    print("ok")
}
```

**After `nyra pkg prune`:**

```ny
fn main() {
    let _dead = 99
    print("ok")
}
```

Implementation: `nyra pkg prune` / `nyra pkg prune --check` (see [NyraPkg](https://nyra-lang.github.io/docs/packages.html#prune)).

## Native code & C interop

Nyra **compiles to native LLVM code** — it is not interpreted. C appears in three deliberate layers:

| Layer | Role | Example |
|-------|------|---------|
| **Nyra runtime** | Bootstrap I/O, strings, spawn, channels | `stdlib/rt/*.c` → stable C ABI |
| **FFI shims** | Thin wrappers around existing C APIs | `link-source` `.c` files in your package |
| **Your app logic** | Business code, routing, validation | `.ny` files — **preferred** |

Nyra is **not** “too weak” for these tasks — C is used for mature libraries (OpenSSL, libpq, hiredis) and low-level runtime, same pattern as Rust + libc. Application code stays in Nyra; do not rewrite Redis/Postgres wire protocols in Nyra.

## Foreign libraries & other languages

Nyra does **not** require libraries to be written in Nyra. Pick the pattern:

| Need | Pattern | Example |
|------|---------|---------|
| C API (raylib, zlib, sqlite3) | `nyra pkg c add NAME` — one command | [c-bindgen](https://nyra-lang.github.io/docs/c-bindgen.html#pkg-c) |
| pip / npm / Maven ecosystem | **Language bridge** — subprocess JSON workers | `stdlib/bridge/mod.ny` |
| Run system command (exit code) | **Command** — fork/exec MVP | `stdlib/process.ny` |
| Host calls Nyra | `export fn` + `--cdylib` | NyraPkg registry / `nyra pkg install` |

### Subprocess — `Command` (stdlib/process.ny)

Like Rust `std::process::Command`. Auto-prelude — no import required.

```ny
fn main() {
    let code = Command_new("ls").arg("-la").run()   // exit code; stdout → terminal
    print(code)

    // Shell one-liner
    Command_new("/bin/sh").arg("-c").arg("uname -a").run()
}
```

- POSIX only today (macOS/Linux); Windows returns `-1`.
- Blocks until child exits; up to 30 args; no `cwd`/env/piped `output()` on `Command` yet.
- **Capture stdout:** `bridge_exec` / `bridge_exec_arg` in `stdlib/bridge/mod.ny`.
- **Interactive PTY shell:** `stdlib/terminal/pty.ny` (terminal apps).
- Docs: [stdlib → process](https://nyra-lang.github.io/docs/stdlib.html#process)

### Language bridge (Nyra → Python / Node / Java)

```ny
import "stdlib/bridge/mod.ny"

fn main() {
    let req = bridge_op_add(10, 32)
    let out = bridge_exec("workers/run_python.sh", req)
    print(bridge_result(out))
}
```

- Protocol: one JSON line stdin → one JSON line stdout (`{"ok":true,"result":"42"}`).
- Extend workers to `pip install numpy`, `npm install lodash`, Maven jars.
- POSIX only today (macOS/Linux); not Wasm/Windows subprocess bridge yet.
- Docs: [stdlib bridge](https://nyra-lang.github.io/docs/stdlib.html) · `examples/bridge/`.

### Host → Nyra (cdylib)

```bash
nyra build lib.ny -o mylib --cdylib
python3 host/call.py    # ctypes + free on returned strings
node host/call.mjs      # koffi (npm install)
```

See [https://nyra-lang.github.io/docs/ffi-abi.html](https://nyra-lang.github.io/docs/ffi-abi.html) · [https://nyra-lang.github.io/docs/bindings.html](https://nyra-lang.github.io/docs/bindings.html).

## Tests

```ny
import "stdlib/testing.ny"

test fn adds() {
    assert_eq(1 + 2, 3)
    assert_bool(true)
}
// Legacy: *_test.ny files run main as test
```

**Helpers** (`stdlib/testing.ny`): `assert_eq`, `assert_ne`, `assert_true`, `assert`, `assert_bool`.

**IDE discovery (v1.32+):** `nyra test . --list-json` prints `[{ "file", "name", "line" }, …]`. Filter: `nyra test . --filter substring`. VS Code extension Test Explorer uses these flags.

**Language conformance (CONF-LANG):** Nyra compiler ships pass/fail conformance fixtures for language features. Run `nyra test` / `nyra check` in your project; see [tooling → conformance](https://nyra-lang.github.io/docs/tooling.html#conformance).

| Suite | Purpose |
|-------|---------|
| CONF-LANG | Nyra-source pass + fail fixtures |
| CONF-* (compiler) | Compile-time IR/ownership contracts |
| `nyra test` | User `test fn` blocks + `stdlib/testing.ny` |

Spec: [tooling → conformance](https://nyra-lang.github.io/docs/tooling.html#conformance).

## Project layout

```
myapp/
  main.ny
  nyra.mod          # optional: module, require, link, link-source
  nyra.lock         # pinned deps (after nyra pkg install)
  nyra.sum          # checksums
  .nyra/cache/      # installed packages
  src/
    helpers.ny
  target/
    debug/main
```

Run: `nyra run .` from project directory.

## Unsafe & no_std (v0.5.0)

```ny
mut x = 42
unsafe {
    let p = &x as *i32
    *p = 99
}
```

Inside `unsafe`: `*ptr` load, `*ptr = v` store, `ptr + i32` / `ptr - i32`, casts involving `*T` or `ptr as i32`.
Outside `unsafe`: only `&T` / `&mut T` references.

`no_std` at top of file (or `--no-std`): no automatic `nyra_rt` link; `print`/`spawn` rejected.
`import "stdlib/core/mem.ny"` for `malloc`, `free`, `memcpy`, `memset`, `volatile_*`.

`ptr` = opaque FFI. `*T` = typed raw pointer for MMIO/drivers — not `Send`.

## OS APIs & asm (v0.5.0)

```ny
import "stdlib/os.ny"

fn main() {
    print(platform_name())       // linux | darwin | windows
    print(battery_percent())     // 0-100 or -1
    print(os_getenv("HOME"))     // NOT getenv — collides with libc
    print(os_getpid())
    unsafe { asm "nop" }
}
```

- `os_syscall6(num, a0..a5)` — raw syscall; constants in `stdlib/os/syscall_linux.ny` / `syscall_darwin.ny`
- `cpu_nop()` / `cpu_pause()` via `stdlib/os/asm.ny`
- Docs: [OS & hardware](https://nyra-lang.github.io/docs/os-hardware.html)
## Performance & optimization

### Release builds

```bash
nyra build --release .          # -O3, thin LTO, target/release/main
nyra build --release --lto-full .
nyra build --release --native-cpu .   # host CPU tuning
```

Flags: `--opt 0-3`, `--lto`, `--lto-full`, `--no-lto`, `--no-llvm-opt`, `--native-cpu` (host only).

Compare debug vs release when benchmarking CPU-bound code.

### Monomorphization & dead code elimination

Nyra targets **batteries-included APIs** with **pay-for-what-you-use** binaries. LLVM optimizes static dispatch and strips uncalled symbols when the stdlib is split into small units.

**Static dispatch (monomorphization):** Generics are specialized **before** LLVM IR (`monomorphize_program` after `expand`). `fn id<T>(x: T) -> T` called with `i32` and `string` becomes `id__i32` and `id__string` — direct calls, no runtime type info. Prefer generics and `impl Trait for Type` on hot paths; use `dyn Trait` only when you need runtime polymorphism (vtable + indirect call). Math intrinsics (`abs_i32`, `min_f64`, …) lower to LLVM intrinsics and are not codegen'd as Nyra functions.

**Four DCE layers:**

| Layer | Mechanism | Effect |
|-------|-----------|--------|
| Lazy prelude | `StdlibVirtualIndex` + `collect_program_uses` | Only referenced `.ny` stdlib modules merge into the program |
| Micro-modules | Small `stdlib/*.ny` + `stdlib/rt/*.c` files | `str_trim` does not pull `regex.ny` or `rt_net.c` |
| Runtime profile | `used_runtime` in codegen → `runtime_map.rs` | Linker gets only needed C runtime translation units |
| LLVM + Thin LTO | `opt -O3`, `clang -flto=thin` on `--release` | Cross-module inlining and dead function elimination |

**Authoring rules:** one focused file per concern in stdlib; `extern fn` per C runtime entry so `runtime_map` can link granularly; `--no-prelude` / `# no_std` for freestanding builds.

Full page: [performance](https://nyra-lang.github.io/docs/performance.html).

### Profile-Guided Optimization (PGO)

PGO records **real execution counts** from a training run, then rebuilds with LLVM profile data so inlining, branch layout, and hot-path ordering match your workload.

**One command (host executables only):**

```bash
nyra build --pgo .
# → target/release/main (or your -o name)
```

**Five phases** (Nyra prints `PGO: phase N/5`):

| Phase | What happens |
|-------|----------------|
| 1 — Instrument | Build temp binary with `-fprofile-instr-generate` |
| 2 — Train | Run `main` + every discovered `test fn` / `test_*` harness |
| 3 — Merge | `llvm-profdata merge` → `target/release/pgo/nyra.profdata` |
| 4 — Optimize | Rebuild with `-fprofile-instr-use` + thin LTO |
| 5 — Cache | Fingerprint sources; unchanged → skip instrument/train/merge |

**Training tips:**

- Training must **exit cleanly** so LLVM flushes `.profraw`.
- Add workload args in `nyra.mod`: `pgo-run --iterations 1000000`
- Or CLI: `nyra build --pgo . --pgo-arg --benchmark --pgo-timeout 600`
- Inside training binaries, `NYRA_PGO=1` is set.

**When to use:** CPU-bound CLI, parsers, game logic, stable server hot paths.  
**Skip when:** I/O-bound, cross-compile (`--for`), wasm, `--cdylib`.

**Prerequisites:** full LLVM toolchain (`opt`, `llvm-profdata`) — `brew install llvm` on macOS.

**Manual workflow:**

```bash
nyra build . --release --pgo-generate -o train_bin
LLVM_PROFILE_FILE=default.profraw ./train_bin
llvm-profdata merge -output=nyra.profdata default.profraw
nyra build . --release --pgo-use nyra.profdata
```

`nyra run --pgo` is rejected — build first, then run `target/release/main`.

Full docs: [PGO](https://nyra-lang.github.io/docs/pgo.html)

### Escape analysis

After borrow checking, Nyra classifies each binding:

| State | Meaning | Codegen effect |
|-------|---------|----------------|
| **NoEscape** | Created and consumed in same function | Stack promotion, SROA, skip redundant clone/free |
| **ArgEscape** | Passed as `&T` to callee, not returned/spawned | Stays on caller stack |
| **GlobalEscape** | `return`, `spawn`, or channel send | Heap / runtime channel |

**Stack promotion & SROA:** NoEscape struct literals skip unnecessary `str_clone`; all-Copy scalar structs (`Point { x: i32 y: i32 }`) decompose into SSA values instead of struct `alloca`.

**LocalChannel:** NoEscape `Channel<T>` never shared with `spawn` → inline ring buffer (capacity 16), no `pthread_mutex` / `rt_channel.c`.

**`#[no_escape]` on parameters:** promise reference never escapes callee:

```ny
fn process(#[no_escape] data: &string) {
    print(data)
}

fn bad(#[no_escape] data: &string) {
    return data   // E0602 — would escape
}
```

- **E0601** — `#[no_escape]` only on `&T` parameters.
- **E0602** — parameter would escape (return, spawn, channel).

**Verbose report:**

```bash
nyra build --verbose .
# escape: main::user → NoEscape
# local channel: main::chan → LocalChannel
```

**FFI boundary:** values passed to / returned from `extern fn` / `export fn` are treated as **GlobalEscape** — stack promotion and LocalChannel do not apply across C ABI.

**Limitations:** SROA for all-Copy structs without spread; LocalChannel sequential only; dynamic heap strings still allocate when they escape.

Full docs: [escape analysis](https://nyra-lang.github.io/docs/escape-analysis.html)

### C FFI out-parameters (`&mut` + `as ptr`)

`let mut` scalars use SSA registers. For C APIs that write through a pointer (e.g. zlib `compress` `destLen`):

```ny
import "vendor/bindings/zlib.ny"

fn main() {
    let data = read_file("content.txt")
    let len: u64 = data.len()
    let dest = valloc(compressBound(len))
    let mut dest_len = compressBound(len)

    unsafe {
        // Compiler spills mut SSA to stack — valid address for C
        compress(dest, (&mut dest_len) as ptr, &data as ptr, len)
    }
}
```

- **`string` at FFI:** pass as `&content as ptr` when callee expects `ptr` (not auto-coerced).
- **`extern fn` with `string` param:** Nyra passes C string pointer automatically.
- **`unsafe` required** for `*ptr` deref, raw casts, pointer arithmetic.

[C bindgen](https://nyra-lang.github.io/docs/c-bindgen.html) · [FFI & ABI](https://nyra-lang.github.io/docs/ffi-abi.html)

## Macros (Extended)

Compile-time **hygienic text substitution** — expanded before typecheck.

```ny
macro double(x) {
    $x + $x
}

fn main() {
    print(double(3))    // → 3 + 3 → 6
}
```

| Rule | Detail |
|------|--------|
| Syntax | `macro name(param, …) { body }` |
| Param refs | `$param` in body |
| Expansion | `name(expr)` → body with args substituted |
| Tier | Extended — `--deny-extended` rejects |



## Traits & dynamic dispatch (Stable Extended)

Nyra supports **trait definitions**, **`impl Trait for Type`**, and **trait objects** via `dyn Trait`. Shipped on **Stable Extended** — multi-method vtables, `dyn Trait + Send + Sync` bounds, and trait-object `Drop`. Remaining gate: multi-trait `dyn A + B` objects.

### Static dispatch

```ny
trait Add {
    fn add(self, other: i32) -> i32
}

struct Counter { value: i32 }

impl Add for Counter {
    fn add(self, other: i32) -> i32 {
        return self.value + other
    }
}

fn main() {
    let c = Counter { value: 5 }
    print(c.add(3))   // static: resolves to Add_Counter_add
}
```

Trait method signatures in the trait block may omit `{ }` or use `;` after the signature.

### Dynamic dispatch (`dyn Trait`)

Box a concrete value as a trait object and call through the vtable:

```ny
fn call_add(g: dyn Add) -> i32 {
    return g.add(1)
}

fn main() {
    let c = Counter { value: 10 }
    print(call_add(c as dyn Add))
}
```

- Cast: `value as dyn TraitName` requires `impl TraitName for Type` for the concrete struct.
- **Auto-trait bounds (v1.5+):** `value as dyn Trait + Send` / `+ Sync` — parsed and **validated** (non-Send/Sync types rejected at cast).
- Fat pointer layout: `{ data: ptr, vtable: ptr }` (synthesized as `Dyn_TraitName`).
- **Multi-method traits (v1.29+):** each method has its own vtable slot; `__dyn_{Trait}_{method}` dispatches correctly.
- **Trait object drop (v1.29+):** vtable drop thunk + `__dyn_{Trait}_drop` frees heap-boxed concrete data.
- **`Drop` / `Clone`** built-in traits use dedicated compiler paths; user traits use the generic vtable.

### Trait bounds on generics (v1.3+)

Constrain type parameters so generic code can call trait methods:

```ny
trait Greet {
    fn hello(self) -> i32
}

fn call_greet<T: Greet>(x: T) -> i32 {
    return x.hello()
}
```

- Syntax: `T: Trait` or `T: A + B` on `fn` / generic declarations
- Checked at monomorph: missing `impl Trait for Type` is a compile error
- Works with inferred call sites (`call_greet(u)` without explicit type args)

See [generics](https://nyra-lang.github.io/docs/generics.html).

### Limitations (MVP)

- Copy-sized structs only (heap box via `malloc` + `memcpy`).
- `dyn Trait + Send + Sync` bounds validated on **casts**; fn-parameter bound checking is partial.
- No `dyn A + B` multi-trait objects yet.
- Explicit **`return`** required in impl bodies (no implicit tail return).
- Extended tier: `nyra check --deny-extended` rejects `trait` / `dyn` in Core-only CI.

See [traits & macros](https://nyra-lang.github.io/docs/traits-macros.html).

## Real-world pitfalls (systems apps)

Nyra is strong for **domain logic** (structs, enums, `match`, modules, FFI). Full terminals, GPU, PTY, and subprocesses need **C shims + vendor bindings** — same pattern as Rust + libc, not pure Nyra stdlib.

| Pitfall | What happens | Fix |
|---------|----------------|-----|
| **String move** | `` `x` was moved into `strcat()` `` | `clone x` or `x.clone()` before the call |
| **Import paths** | `import "vendor/foo.ny"` fails from `src/gpu/` | Paths are relative to the **importing file**: `import "../../vendor/foo.ny"` |
| **HashMap wrappers** | Chained `.insert()` on `HashMap_str_*` structs | Supported in v1.2.x+; or use `ptr` + `map_str_*` externs |
| **FFI `u8` fields** | `255` inferred as `i32` in some contexts | Annotate field type `u8` on struct; literals in struct literals coerce |
| **REPL vs shell** | `input()` is line-based, not a PTY | Use `forkpty` / C shim (`link-source`) for real terminals |
| **`nyra run .` showcase** | Default may be demo, not interactive shell | Document env flags (e.g. `GHOSTTERM_REPL=1`) in your app README |

**Systems app pattern (recommended):**

```text
Nyra (domain logic) + rt/*.c (PTY/FFI) + vendor/bindings (raylib, etc.) + nyra.mod link-source
```

in repo.

## Do NOT hallucinate

- No garbage collector.
- **Stdlib is batteries-included:** `stdlib/crypto`, `db/sqlite`, `net/websocket`, `compress`, full `serialize` are **core stdlib** — some are still stubs while native code lands; do not treat NyraPkg as the primary path for these domains.
- **Enum payloads — precise rules (not “never”):**
  - Tag-only user enums (`enum Color { Red }`) → **no** `Color.Red(x)` unless you declare a payload field.
  - Built-in `Option` / `Result` **without import** → tag names only; **`Option.Some(42)` is wrong** without `import "stdlib/option.ny"` or a monomorph enum like `Option_i32.Some(42)`.
  - **With** `stdlib/option.ny` → `Option.Some(v)`, `Result.Ok(v)`, `Result.Err(e)` **do** store values (monomorphized `T` / `E`).
  - No multi-field variants (`Some(a, b)`) or mixed payload types in one enum (MVP limit).
- **`?` operator** — `Result`/`Option` propagate on `let`/`const`/`return`/expr stmt, nested expressions (`print(f()?)`, call args), `return match` arm bodies, and `let n = match { Ok(v) => f(v)?, … }`. Enclosing function must return the same enum for propagation; in `void` test fns the inner `Err` payload becomes the `i32` binding. `??` nullish coalesce and `?.` optional chain are separate.
- No **`defer free(x)`** for owned `string` — auto-drop handles it; use **`impl Drop` RAII** for handles, not `defer`, when possible (`defer` is Extended).
- No `extern export fn` — use `extern fn` or `export fn` separately.
- Async/`await`: promise handles + **executor v1.4** + **state-machine v1.6** + **v1.7 CFG** (`await` in `if`/`while`/range `for`). `async fn` body runs on **`spawn:thread`**. `spawn`/`unsafe` with `await` still blocking. **`JoinHandle.join()`** blocks on task/thread completion. **`nyra build --race`** enables TSan. See [async guide](https://nyra-lang.github.io/docs/async.html) · [concurrency](https://nyra-lang.github.io/docs/concurrency.html).
- **Struct JSON** — `{Struct}_json_encode/decode` after monomorph; fields: `string`/`i32`/`bool`/nested struct/**`ptr` Vec_i32/fixed `[T; N]`**.
- **`Serialize` trait (v1.38+)** — `u.to_json()` / `u.to_bytes()`; import `stdlib/serde/mod.ny` for trait defs; decode via `{Struct}_json_decode`.
- Arrow functions are **Extended** tier — use `nyra check --deny-extended` in Core-only CI if you avoid them.

## Diagnostics

Stable codes — explain with `nyra explain E003` or `nyra explain --list`. JSON: `nyra diag . --json`.

### Errors (E00x)

| Code | Title | Meaning |
|------|-------|---------|
| **E001** | import not found | `import "path"` does not resolve |
| **E002** | undefined name | Variable/function/type not in scope |
| **E003** | type mismatch | Expression type ≠ expected context |
| **E004** | cannot infer type | Add explicit `: Type` annotation |
| **E005** | unknown struct | Struct name/literal not defined |
| **E006** | immutable assignment | Reassign `let` without `mut` |
| **E007** | wrong arity | Call argument count mismatch |
| **E008** | wrong argument type | Arg position type mismatch |
| **E009** | invalid assignment target | LHS not an l-value |
| **E010** | borrow while assigned | `&mut` conflicts with assignment |
| **E011** | use while borrowed | Value used during active borrow |
| **E012** | use after move | Move-type used after transfer |
| **E0601** | no_escape violation | `#[no_escape]` param escaped callee |

### Warnings (W00x)

| Code | Title | Fix |
|------|-------|-----|
| **W001** | extended tier feature | Add `allow_extended` at file top, remove Extended syntax, or drop `--deny-extended` |
| **W002** | unused import | Remove import or `nyra pkg prune` |
| **W003** | unused variable | Prefix `_` or remove |

### Parser (P00x)

| Code | Title |
|------|-------|
| **P001** | anonymous object literal (old) — use struct or `{ field: value }` literal |
| **P006** | missing comma in object literal fields |

Page: [diagnostics](https://nyra-lang.github.io/docs/diagnostics.html)

## Common errors

| Message | Meaning |
|---------|---------|
| Use of moved value | Move-type used after transfer — use `&`, `clone x`, or `x.clone()` |
| `break` is only valid inside `while` or `for` | `break` outside a loop |
| Expected ')' after arguments ... `.clone()` | Old parser bug — use newlines between statements; ensure compiler ≥ break/clone fix |
| Field expected `u8`, found `i32` | Add struct field type `u8` or use `integer_assignable` context (struct literal) |
| Undefined function `@insert` / `@get` | HashMap method chaining codegen bug (fixed) — update compiler |
| Cannot borrow as mutable | `&mut` aliasing conflict |
| cannot return reference to local | Dangling reference return |
| Expected 'fn' after extern | Invalid extern syntax |

## Online documentation map

**Home:** [https://nyra-lang.github.io/docs/](https://nyra-lang.github.io/docs/)

| Topic | URL |
|-------|-----|
| Learn Nyra (tutorial track) | [https://nyra-lang.github.io/docs/learn-intro.html](https://nyra-lang.github.io/docs/learn-intro.html) |
| Get started | [https://nyra-lang.github.io/docs/learn-get-started.html](https://nyra-lang.github.io/docs/learn-get-started.html) |
| Language reference | [https://nyra-lang.github.io/docs/reference.html](https://nyra-lang.github.io/docs/reference.html) |
| Built-in methods | [https://nyra-lang.github.io/docs/methods.html](https://nyra-lang.github.io/docs/methods.html) |
| Standard library | [https://nyra-lang.github.io/docs/stdlib.html](https://nyra-lang.github.io/docs/stdlib.html) |
| Data structures (learn) | [https://nyra-lang.github.io/docs/learn-data-structures.html](https://nyra-lang.github.io/docs/learn-data-structures.html) |
| Match | [https://nyra-lang.github.io/docs/match.html](https://nyra-lang.github.io/docs/match.html) |
| Async | [https://nyra-lang.github.io/docs/async.html](https://nyra-lang.github.io/docs/async.html) |
| Traits & macros | [https://nyra-lang.github.io/docs/traits-macros.html](https://nyra-lang.github.io/docs/traits-macros.html) |
| Concurrency | [https://nyra-lang.github.io/docs/concurrency.html](https://nyra-lang.github.io/docs/concurrency.html) |
| Memory & ownership | [https://nyra-lang.github.io/docs/memory.html](https://nyra-lang.github.io/docs/memory.html) |
| Ownership (learn) | [https://nyra-lang.github.io/docs/learn-ownership.html](https://nyra-lang.github.io/docs/learn-ownership.html) |
| Borrowing (learn) | [https://nyra-lang.github.io/docs/learn-borrowing.html](https://nyra-lang.github.io/docs/learn-borrowing.html) |
| PGO | [https://nyra-lang.github.io/docs/pgo.html](https://nyra-lang.github.io/docs/pgo.html) |
| Escape analysis | [https://nyra-lang.github.io/docs/escape-analysis.html](https://nyra-lang.github.io/docs/escape-analysis.html) |
| Performance | [https://nyra-lang.github.io/docs/performance.html](https://nyra-lang.github.io/docs/performance.html) |
| net/http | [https://nyra-lang.github.io/docs/net-http.html](https://nyra-lang.github.io/docs/net-http.html) |
| C bindgen | [https://nyra-lang.github.io/docs/c-bindgen.html](https://nyra-lang.github.io/docs/c-bindgen.html) |
| FFI & ABI | [https://nyra-lang.github.io/docs/ffi-abi.html](https://nyra-lang.github.io/docs/ffi-abi.html) |
| NyraPkg | [https://nyra-lang.github.io/docs/packages.html](https://nyra-lang.github.io/docs/packages.html) |
| Diagnostics | [https://nyra-lang.github.io/docs/diagnostics.html](https://nyra-lang.github.io/docs/diagnostics.html) |
| Stdlib bridge | [https://nyra-lang.github.io/docs/stdlib.html](https://nyra-lang.github.io/docs/stdlib.html) |
| AI skill download page | [https://nyra-lang.github.io/docs/ai-skill.html](https://nyra-lang.github.io/docs/ai-skill.html) |
| Roadmap & status | [https://nyra-lang.github.io/docs/roadmap.html](https://nyra-lang.github.io/docs/roadmap.html) |
| Changelog | [https://nyra-lang.github.io/docs/changelog.html](https://nyra-lang.github.io/docs/changelog.html) |

