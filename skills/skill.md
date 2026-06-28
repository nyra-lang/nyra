# Nyra Programming Language

> Canonical copy: `webDocs/nyra-skill.md`. Regenerate with `node webDocs/scripts/build-nyra-skill.mjs`.

# Nyra Programming Language

> Canonical copy: `webDocs/nyra-skill.md`. Regenerate with `node webDocs/scripts/build-nyra-skill.mjs`.

# Nyra Programming Language

> Canonical copy: `webDocs/nyra-skill.md`. Regenerate with `node webDocs/scripts/build-nyra-skill.mjs`.

Use this file as the **sole authoritative reference** for Nyra syntax, semantics, stdlib, toolchain, PGO, and escape analysis.
Do not invent features not listed here. Full docs: `webDocs/` in the Nyra repository.

> **Project status — v1.36 production-ready tier:** **Core** and **Stable Extended** (async, traits, macros, lifetimes, defer, serde, `?`, spawn, enum payloads) ship **without W001**. Prebuilt Linux, macOS, and Windows releases. See [Stability](#stability-v10) · `docs/status.md`.

## Table of contents

1. [Identity & compiler pipeline](#identity)
2. [Design philosophy — easy syntax & optional types](#design-philosophy)
3. [Toolchain & CLI](#toolchain)
4. [Syntax conventions & variables](#syntax-conventions)
5. [Language reference — keywords, operators, statements](#language-reference)
6. [Types & functions](#types)
7. [Control flow, structs, enums & payloads, imports](#control-flow)
8. [Built-in API & I/O (no import)](#io--builtins)
9. [Ownership & memory](#ownership-summary)
10. [Performance — monomorph, DCE, release, PGO, escape analysis](#performance--optimization)
11. [Stdlib, NyraPkg, FFI & C interop](#stdlib-modular--see-stdlibreadmemd)
12. [Unsafe, OS, tests, layout](#unsafe--no_std-v050)
13. [Do NOT hallucinate](#do-not-hallucinate)

> **Enum payloads (read first):** Default enums are **tag-only** (`Color.Red`). **`Option.Some(42)` stores a real value** only when you `import "stdlib/option.ny"` (or define `enum Option_i32 { None, Some(i32) }`). Built-in `Option`/`Result` without import are tag **names** for `??`/`?.` — not storage. See [Enums & payloads](#enums--payloads).

> **Collections naming:** Core stdlib uses **monomorph names** (`Vec_i32_push`, `HashMap_str_i32`). **Generic syntax** (`Vec<T>`, `Arc<T>`, `Box<T>`) is Extended — current for smart pointers, but vectors/maps in docs/examples use `Vec_i32` style. See [Naming: current vs legacy](#naming-current-style-vs-legacy-read-this).

## Identity

- **Nyra** — systems language: Go-like syntax, Rust-like ownership, LLVM backend.
- Source: `.ny` / `.nyra` files → lexer → parser → expand → monomorph (+ generic call inference) → auto-borrow coercion → typecheck → ownership (Copy inference) → borrow + lifetimes + Send/Sync → **escape analysis** → drop plan → LLVM IR → `opt` → clang + runtime C modules.
- CLI: `nyra` (Rust). Package manager: `nyra pkg` (NyraPkg).
- Version baseline: **v1.36.x** — **Core tier semver-stable**; **Stable Extended** shipped ([`docs/stability-v1.md`](../docs/stability-v1.md) · [`docs/status.md`](../docs/status.md)).
- **v1.2:** template strings, arrow functions, `net/http` handler dispatch, language bridge (Python/Node/Java workers), NyraPkg semver + registry, `link-source` auto-link, bindings reference, native C interop pattern.
- **v2.1:** stack closures (loop-safe), arrow param inference, tuple destructure in arrow params, `??` nullish coalescing, `?.` optional chaining.
- **v2.2:** heap closure promotion; `?.method()`; **`Option.Some(T)` payloads** when using `import "stdlib/option.ny"` (replaces tag-only built-in `Option` for that module).
- **v2.3:** composite struct field drop, auto-owned `extern fn -> string`, `Box_string` (superseded by `Box<string>` in v2.4), `OptionStr`.
- **v2.4:** generic `enum Option<T>` / `enum Result<T,E>` monomorph; enum payload drop; `struct Box<T>` + `Box_new(string)` (replaces `Box_string`).
- **v2.5:** generic `struct Arc<T>` (`Arc<i32>`, `Arc<string>`); auto Drop for monomorph instances; `Arc_i32` kept as legacy alias in `stdlib/arc.ny`.
- **v2.6:** async bootstrap patterns, HTTP health via stdlib `net/http`.
- **v2.7:** `nyra.mod` workspaces, `CONF-WS-*` conformance (`conformance/workspace.rs`), `webDocs/enterprise.html`. Tracing/service mesh = external.
- **v2.8:** return type inference (`void` default), generic call-site inference (`id(7)`), auto-borrow at calls (`T` → `&T`), `string.clone()` + synthesized struct `Clone`, struct ctor sugar `User("Ada")` / `Point()`.
- **v2.9:** Swift-style use-after-move diagnostics (`was moved into save()` + fix-it notes), `move` / `clone` prefix at call sites (`save(move user)`, `save(clone user)`).
- **v3.0:** auto Copy inference for all-Copy structs (`Point`, `Rect`); `#[derive(Copy)]` validation; no annotation needed for value types.
- **v3.1:** `f64` IEEE-754 double — float literals (`3.14`), mixed `i32`/`f64` promotion, LLVM `double` codegen.
- **v3.2:** `char` Unicode scalar — `'a'`, `'\n'`, `'\u{...}'`; LLVM `i32`; `print` via `%c`.
- **v1.3:** **CONF-LANG** Nyra-source conformance (`tests/conformance/` — pass + fail + fixtures), `stdlib/testing.ny` assertions, `scripts/conformance-tests.sh` in `test-all.sh`.

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

**Multi-file projects:** in `main.ny`, import every module the app needs (e.g. `import "src/cli.ny"` **and** `import "src/cat.ny"`). Shared helpers like `cli.ny` should be imported from `main.ny`, not only from a sibling file — see `Apps/FileSystem/cat/`.

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

**For AI assistants:** when generating Nyra for beginners or app code, **omit types unless the user asks for them** or inference would fail. Do not “Rust-ify” Nyra by sprinkling `: i32` everywhere. See `webDocs/stdlib.html#optional-types` · learn track (`learn-get-started.html`).

## Stability (v1.0)

- **Core (stable):** types, control flow, modules, **enum tags** (unit variants), `match`, `impl Type { }`, ownership, FFI, `unsafe`/`no_std` (documented MVP).
- **Extended (experimental):** **enum payloads** (`Some(i32)`, `Ok(T)` / `Err(E)`), `async`/`await`, traits, macros, **`defer`** (see [defer vs Drop](#defer-vs-drop--when-to-use-which)), explicit lifetimes, `spawn { }`.
- **Core-stable:** monomorph **generics** (`fn id<T>`, `Option<T>`, `Result<T,E>`, `Arc<T>`, `Box<T>`), optional type annotations, `module` declarations.
- Compiler emits **`warning[W001]`** on Extended features.
- **`nyra check --deny-extended`** — fail on Extended tier (Core-only CI).

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

Release flags: `--release`, `--opt 0-3`, `--lto`, `--lto-full`, `--no-lto`, `--no-llvm-opt`, `--no-prelude`, `--native-cpu`, `--no-native-cpu` (host `--release` uses `-march=native` by default), `--pgo-generate`, `--pgo-use FILE`, `--for`, `--os`, `--arch`, `--target`.

Systems / freestanding: `--no-std` (skip `nyra_rt` link), `--freestanding` (`-ffreestanding -nostdlib`). Top-level `no_std` in source has the same effect as `--no-std`.

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

Unclosed `/*` is a lexer error. Tests: `tests/nyra/block_comments_test.ny` · `CONF-COMMENT-*`.
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

Fixed at compile time; shared fixed values across the program. Not the same as `let` — you cannot compute `const` from runtime input.

| | `let` | `let mut` | `const` |
|---|-------|-----------|---------|
| Reassign? | No | Yes | No |
| When set? | Runtime in code | Runtime; can change | Compile time |
| Example | `let name = "Ali"` | `let mut gold = 0` | `const MAX = 100` |

- Immutable `let` of **Move** types (heap `string`) transfers ownership on use.
- `let mut` of Copy types (`i32`, `bool`, enums) is not moved on function call.

Integer separators: `1_000_000`

## Language reference

Quick lookup for syntax the lexer and parser accept today. Types are optional unless inference fails.

### Keywords

| Keyword | Purpose |
|---------|---------|
| `fn` | Function definition |
| `let` / `let mut` | Immutable / mutable binding |
| `const` | Compile-time constant |
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
| `spawn` | Concurrent block (Extended) |
| `parallel for` | Parallel loop over range or array (Extended) |
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
spawn { print(1) }               // Extended
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
| void | — | No return value (Rust `()` unit type) |
| struct Name { fields } | Copy or Move | Move if any field is Move |
| enum Name { A, B } | Copy | **Tag-only** by default — unit variants, no stored data |
| enum Name { Some(T) } | Copy or Move | **With payload** (Extended) — one field per variant; see [Enums & payloads](#enums--payloads) |
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

Example: `examples/syntax/systems_types.ny` (zero types) and `.typed.ny`.

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

Docs: `webDocs/c-bindgen.html` · `examples/c_raylib/` · `examples/c_bindgen/`

### Template strings (v1.2 — Core)

Backtick strings with JS-style `${expr}` interpolation (static text + `i32` / `string` values):

```ny
let name = "hamdy"
let age = 25
print(`Hello, ${name}!`)
print(`Hello ${name}, age ${age}`)
```

See `examples/syntax/template_strings.ny` · learn track: `learn-strings.html`.

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

Pass as `fn(...)` parameters: `iter_filter(v, pred)` or `listen_and_serve_handlers(host, port, router, health_slot)`.

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
```

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

Tests: `tests/nyra/result_propagate_test.ny` · Examples: `examples/result_propagate_question.ny`, `examples/try_operator_smoke.ny` · Conformance: `CONF-ADT-004`

Generic `Result<T,E>` / `Option<T>` (auto-prelude or `import "stdlib/option.ny"`) supports `?` the same way after monomorphization.

See: `stdlib/option.ny` · `stdlib/result.ny` · `examples/option_payload_smoke.ny` · `CONF-ADT-*` conformance tests.

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
- Import brings **public** functions, structs, enums, consts into scope (`pub` default; `priv` hides from importers).
- `import "path" as alias` + `alias::symbol` qualified calls (v1.37+).

## I/O & builtins

See full runnable gallery: `examples/builtins/` · `webDocs/methods.html` · `webDocs/stdlib.html#builtins`

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

### Fixed arrays & `for … in` (no import)

| Syntax / method | On | Returns |
|-----------------|-----|---------|
| `for i in 0..n` | half-open range | — |
| `for x in arr` | `[T; N]` array | element `T` |
| `for c in str` | `string` | `char` per byte |
| `arr.length()` / `arr.len()` | fixed array | `i32` |
| `arr.sort()` | `i32` / `f64` array | new sorted copy |
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

### Timing & memory (no import)

```ny
time_start("label")
// ... work ...
time_end("label")    // prints elapsed (colored terminal output)

mem_start("label")
mem_end("label")     // prints RSS delta (platform-dependent)
```

### `spawn { }` (Extended — no import keyword)

Runs block on new thread. Captures must be **Send**; no `&` captures.

```ny
spawn {
    print(42)
}
```

Channels: `stdlib/sync/channel.ny` · see `examples/syntax/spawn_channel.ny`

### `parallel for` (Extended)

Independent iterations across worker threads — no manual thread pool. Compiler lowers to `parallel_for_range` (`stdlib/rt/rt_parallel.c`).

```ny
parallel for i in 0..1000 { work(i) }
parallel(max_threads = 4) for i in 0..1000 { work(i) }
parallel(threads = 4) for i in 0..1000 { work(i) }
parallel(cpu = 80%) for i in 0..n { work(i) }
parallel(threads = cpu_count() - 1) for i in 0..n { work(i) }
parallel(mode = balanced) for i in 0..n { work(i) }
```

| Option | Meaning |
|--------|---------|
| *(none)* | `mode = auto`, workers from CPU count |
| `max_threads = N` | At most N workers |
| `threads = N` | Exactly N workers |
| `cpu = P%` | `P` percent of logical CPUs |
| `mode` | `auto`, `balanced`, `max_performance`, `background` |

`cpu_count()` — built-in logical CPU count.

Rules: no `break`; no mutation of outer variables; captures must be **Send**; iterable must be range, fixed array, `string`, or `vec_str`. On `wasm32-wasi`, runs sequentially.

See `examples/parallel_for.ny`.

### `progress for` (Extended)

Built-in progress bar for sequential loops (`stdlib/rt/rt_progress.c`).

```ny
progress(label = "parser tests") for item in tests {
    run(item)
}

progress for i in 0..100 {
    step(i)
}
```

Output each iteration: `[#####-------] 43%` plus `Running parser tests...`. Optional `label = "..."`; default derives from iterable name. Cannot combine with `parallel for`.

See `examples/progress_for.ny`.

### `benchmark { }` (Extended)

Measure wall time, RSS delta, and process CPU usage — no manual timers.

```ny
benchmark {
    run()
}
```

Prints:

```
Time: 14.2 ms
Memory: 1.8 MB
CPU: 38%
```

Lowers to `benchmark_begin()` / `benchmark_end()` in `stdlib/rt/rt_bench.c`. For iteration loops use `stdlib/bench/mod.ny`; for labeled timers use `time_start` / `mem_start`.

See `examples/benchmark_block.ny`.

## Stdlib-style helpers (import required)

These are **not** built-ins — import when needed:

```ny
import "stdlib/builtins_array.ny"
import "stdlib/vec.ny"
```

| Function | Description |
|----------|-------------|
| `Array_push(v, x)` | Append `i32` to `Vec_i32` |
| `Array_pop(v)` | Pop last `i32` |
| `Array_map(v, f)` | Map with `fn(i32) -> i32` |
| `Array_filter(v, pred)` | Filter (`pred` returns 1/0) |
| `Array_reduce(v, init, f)` | Fold left |
| `Array_find(v, pred, fallback)` | First match or fallback |

Example: `examples/builtins/array/main.ny`

## Ownership (summary)

Nyra has **no GC**. The compiler builds a **DropPlan** per function and emits `free` / custom `Drop_*_drop` at scope exit. Docs: `webDocs/memory.html`, lessons `learn-ownership.html` / `learn-borrowing.html`.

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

### defer vs Drop — when to use which

**Short answer:** **`Drop` (auto-drop + `impl Drop`) covers almost every cleanup case.** Keep `defer` in **Extended** tier — it is a niche escape hatch, not the default path. Do **not** move it to Core while RAII remains the recommended model.

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

See: `webDocs/memory.html#defer` · `webDocs/memory.html#custom-drop` · `CONF-*` ownership tests

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

See `examples/syntax/spread_operator.ny` (zero-types) and `spread_operator.typed.ny`. Object spread into arrays requires **compatible scalar field types** (same element type as the rest of the array). Object spread in `{ ...obj }` requires a **struct** value.

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

See `examples/syntax/struct_spread_merge.ny`. Named struct targets use `Type { ...spread }`. Anonymous `{ ...spread, key: value }` also works when fields are inferred (see above).

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

## Stdlib (modular — see stdlib/README.md)

> **Batteries-included by design:** Nyra’s stdlib is **strong** — crypto, databases, serialization, WebSocket, compression, and encoding belong **in-tree** with the compiler. Some modules are still **stubs or MVP** while native implementations land in `stdlib/rt/`; import paths stay stable. **NyraPkg** complements stdlib for community and optional packages — it does not replace core domains. Status inventory: `stdlib/README.md` · `webDocs/stdlib.html#status`.

### What ships vs what is in progress

| Status | Modules | Notes |
|--------|---------|-------|
| **Shipped** | `vec.ny`, `vec_str.ny`, `map.ny`, `collections/*`, `strings/ops.ny`, `fs/mod.ny`, `path.ny`, `crypto/mod.ny`, `encoding/base64.ny`, `net/tcp.ny`, `net/http/mod.ny`, `net/udp.ny`, `net/websocket.ny`, `compress/mod.ny`, `serialize/mod.ny`, `json/mod.ny`, `db/sqlite.ny`, `tls.ny`, `time/*`, `strconv/mod.ny`, `flag/mod.ny`, `bufio/mod.ny`, `context/mod.ny`, `sync/mod.ny`, `process.ny` (POSIX), `bridge/mod.ny`, `terminal/*`, `encoding/csv.ny`, `archive/zip.ny`, `mime/mod.ny`, `random_bytes`, `embed/mod.ny`, `slog/mod.ny`, `testing/fstest.ny`, `testing/quick.ny` | Collections, FS, crypto, HTTP/TCP/UDP/WS, CLI, DB (SQLite), sync |
| **MVP / partial** | `json/mod.ny` / `serialize/mod.ny` (multi-key encode; not full schema serde), `uuid/mod.ny`, `url` helpers, `async.ny`, `reflect/mod.ny` | Use NyraPkg (`ny-serde`) for full schema serde |
| **Native when linked** | `db/postgres.ny` (`link pq`), `db/mysql.ny` (`link mysqlclient`), `compress/bzip2.ny` (`link bz2`) |
| **Shipped (v1.1)** | `env_set`, `process` (POSIX + Windows), Windows prebuilt releases |
| **Stub → in progress** | `compress/bzip2.ny` (link `bz2`) | Native driver when linked |

Tell users Nyra **targets** production crypto, SQLite, WebSocket, and full serde **in stdlib**. Where a module is still a stub, say so honestly — do not redirect to NyraPkg as the primary path. See [Language vs Ecosystem](../webDocs/language-vs-ecosystem.html).

### Naming: current style vs legacy (read this)

Nyra uses **monomorph names** in Core stdlib and **generic syntax** in Extended tiers. Both compile; prefer the **Current** column for new code and `--deny-extended` CI.

| API | Current (use this) | Legacy / alternate | Notes |
|-----|-------------------|-------------------|-------|
| Growable `i32` vector | `Vec_i32_new()`, `Vec_i32_push(v, x)`, `Vec_i32_len(v)` | `Vec<T>` generic syntax (Extended) | Handle type is `ptr`; free with `Vec_i32_free(v)` or scope end if owned |
| String-key map | `HashMap_str_i32_*`, `HashMap_str_str_*`, `Map_str_i32_*` in `map.ny` | `HashMap<K,V>` (Extended) | **Method chaining works:** `HashMap_str_i32_new().insert("a", 1).insert("b", 2)` · or low-level `ptr` + `nyra_map_*` externs |
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

Do **not** write `v.push(1)` — `Vec_i32` is a `ptr` handle, not a method-chaining object. Use `Vec_i32_push(v, x)` or `import "stdlib/builtins_array.ny"` helpers (`Array_push`, `Array_map`, …).

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

See: `examples/graph_arc_smoke.ny` · `examples/monolith_struct_smoke.ny` · `stdlib/README.md`

```ny
import "stdlib/vec.ny"
import "stdlib/map.ny"
import "stdlib/strings/ops.ny"
```

**Stdlib auto-prelude (lazy):** Referenced stdlib symbols resolve on demand via a virtual symbol table — use `read_file`, `Vec_i32_new`, `StrVec`, `os_arg_count`, `os_arg_at`, `list_dir`, `is_dir`, etc. without imports; only used modules are merged into the build. Opt out with `# no_std` or `--no-prelude`. Explicit `import "stdlib/vec.ny"` still works.

**Compiler math intrinsics (always on):** `abs_i32`, `abs_f64`, `min_i32`, `max_i32`, `clamp_i32`, `min_f64`, `max_f64`, and typed `abs(x)` lower to LLVM intrinsics — no stdlib merge required. See `examples/builtins/math_intrinsics.ny` with `--no-prelude`.

**Core modules (usable):** `vec.ny`, `vec_str.ny`, `map.ny`, `collections/*`, `strings/ops.ny`, `strings/regex.ny`, `fs/mod.ny`, `path.ny`, `crypto/mod.ny`, `encoding/base64.ny`, `time/instant.ny`, `time/date.ny`, `json/mod.ny`, `serialize/mod.ny`, `iter/mod.ny`, `env/mod.ny`, `config/mod.ny`, **`net/http/mod.ny`**, `net/tcp.ny`, `net/udp.ny`, `net/websocket.ny`, `tls.ny`, `strconv/mod.ny`, `flag/mod.ny`, `bufio/mod.ny`, `context/mod.ny`, `sync/mod.ny`, `process.ny`, `bridge/mod.ny`, `db/sqlite.ny`, `db/lsm.ny`, `db/sql_parse.ny`, `db/sstable.ny`, `collections/btree_pages.ny`, `bench/mod.ny`, `profile/mod.ny`, `testing.ny`, `async.ny` (Extended). Docs: `webDocs/stdlib.html` (`#cli-parsing`, `#database`, `#process`, `#crypto`).

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

**Shipped (v1.1):** `env_set`, `process` on Windows, postgres/mysql native when linked. **NyraPkg** for full serde: `ny-serde`, `ny-toml`. See `stdlib/README.md`.

### net/http quick start (v1.2)

Static response bodies:

```ny
import "stdlib/net/http/mod.ny"

fn main() {
    let router = Router_new()
    let r = Router_add_get(router, "/health", "{\"status\":\"ok\"}")
    listen_and_serve("127.0.0.1", 8080, r)
}
```

Handler dispatch (Nyra functions per route slot):

```ny
import "stdlib/net/http/mod.ny"

fn health_slot(slot: i32, ctx: RequestContext) -> HttpResponse {
    return response_ok_json("{\"status\":\"ok\"}")
}

fn main() {
    let router = Router_new()
    let r = Router_add_slot_get(router, "/health", 0)
    listen_and_serve_handlers("127.0.0.1", 8080, r, health_slot)
}
```

Full API: `webDocs/net-http.html` · `examples/net_http_smoke.ny`. Compose with `stdlib/db/*` and NyraPkg drivers for production services.

**Low-level runtime** (still valid): `read_file`, `vec_i32_*`, `map_str_i32_*`, `channel_*`, `bridge_exec`, `spawn { }`.

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
| Bundled dev copy | `examples/packages/ny-sqlite`, `ny-serde`, `ny-toml` when in Nyra repo |

- **`link`** / **`link-arg`** merge into project `nyra.mod` on install.
- **`link-source`** compiles package `.c` files at `nyra build` (no manual `clang`).
- Lock: `nyra.lock` + `nyra.sum` pin exact versions; `nyra pkg verify` checks constraints.
- **`nyra pkg prune`** — auto-fix unused code (like `cargo fix` for lint warnings). See [packages.html](packages.html#prune).
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

Implementation: `compiler/lint/src/prune.rs` · driver: `Compiler::prune_project()` · tests: `cargo test -p lint`, `cargo test -p compiler --test pkg_prune`.

## Native code & C interop

Nyra **compiles to native LLVM code** — it is not interpreted. C appears in three deliberate layers:

| Layer | Role | Example |
|-------|------|---------|
| **Nyra runtime** | Bootstrap I/O, strings, spawn, channels | `stdlib/rt/*.c` → stable C ABI |
| **FFI shims** | Thin wrappers around existing C APIs | `link-source rt/hiredis_shim.c`, `examples/packages/ny-redis/rt/` |
| **Your app logic** | Business code, routing, validation | `.ny` files — **preferred** |

Nyra is **not** “too weak” for these tasks — C is used for mature libraries (OpenSSL, libpq, hiredis) and low-level runtime, same pattern as Rust + libc. Application code stays in Nyra; do not rewrite Redis/Postgres wire protocols in Nyra.

## Foreign libraries & other languages

Nyra does **not** require libraries to be written in Nyra. Pick the pattern:

| Need | Pattern | Example |
|------|---------|---------|
| C API (raylib, zlib, sqlite3) | `nyra pkg c add NAME` — one command | `examples/c_raylib/` · `webDocs/c-bindgen.html#pkg-c` |
| pip / npm / Maven ecosystem | **Language bridge** — subprocess JSON workers | `stdlib/bridge/mod.ny` |
| Run system command (exit code) | **Command** — fork/exec MVP | `stdlib/process.ny` |
| Host calls Nyra | `export fn` + `--cdylib` | `examples/ffi/export_greet/` |

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
- **Interactive PTY shell:** `stdlib/terminal/pty.ny` · GhostTerm.
- Docs: `webDocs/stdlib.html#process` · example: `examples/process_command.ny`.

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
- Demo: `examples/bridge/` · docs: [`docs/bridge.md`](../docs/bridge.md).

### Host → Nyra (cdylib)

```bash
nyra build lib.ny -o mylib --cdylib
python3 host/call.py    # ctypes + free on returned strings
node host/call.mjs      # koffi (npm install)
```

See [`docs/abi-policy.md`](../docs/abi-policy.md). Runtime symbol map: [`docs/bindings.md`](../docs/bindings.md) / `webDocs/bindings.html`.

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

**Language conformance (CONF-LANG):** `tests/conformance/` — `pass/` (`nyra test`), `fail/` (`nyra check` must reject), `fixtures/` (import smoke). Run: `bash scripts/conformance-tests.sh` or `./target/debug/nyra test tests/conformance/pass`. Use `./target/debug/nyra` (not stale `~/.cargo/bin/nyra`) so `import "stdlib/testing.ny"` resolves.

| Suite | Purpose |
|-------|---------|
| `tests/conformance/` | Feature-by-feature Nyra pass + fail (CONF-LANG) |
| `compiler/driver/tests/conformance/` | Rust `CONF-*` compile/IR contracts |
| `tests/suite/` | File-based compiletest (~1.6k fast; `--profile full` ~10k) |
| `tests/nyra/` | Legacy native smoke |

Spec: `tests/conformance/README.md` · docs: `webDocs/tooling.html#conformance`.

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
- Docs: `libraries/os/README.md`

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

Examples: `examples/toolchain/lazy_prelude.ny`, `examples/toolchain/monomorph_static_dispatch.ny`. Full page: `webDocs/performance.html`.

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

Full docs: `webDocs/pgo.html`

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

Full docs: `webDocs/escape-analysis.html` · design: `Escape_Analysis.md`

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

See `webDocs/c-bindgen.html` · `webDocs/ffi-abi.html`

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

Example: `examples/trait_bounds.ny` · tests: `tests/nyra/trait_bounds_test.ny`

### Limitations (MVP)

- Copy-sized structs only (heap box via `malloc` + `memcpy`).
- `dyn Trait + Send + Sync` bounds validated on **casts**; fn-parameter bound checking is partial.
- No `dyn A + B` multi-trait objects yet.
- Explicit **`return`** required in impl bodies (no implicit tail return).
- Extended tier: `nyra check --deny-extended` rejects `trait` / `dyn` in Core-only CI.

Example: `examples/trait_dyn.ny` · `examples/trait_dyn_multi.ny` · `examples/trait_dyn_send.ny` · tests: `tests/nyra/trait_dispatch_test.ny`, `trait_dyn_multi_test.ny`, `dyn_send_test.ny`

## Real-world pitfalls (apps like GhostTerm)

Nyra is strong for **domain logic** (structs, enums, `match`, modules, FFI). Full terminals, GPU, PTY, and subprocesses need **C shims + vendor bindings** — same pattern as Rust + libc, not pure Nyra stdlib.

| Pitfall | What happens | Fix |
|---------|----------------|-----|
| **String move** | `` `x` was moved into `strcat()` `` | `clone x` or `x.clone()` before the call |
| **Import paths** | `import "vendor/foo.ny"` fails from `src/gpu/` | Paths are relative to the **importing file**: `import "../../vendor/foo.ny"` |
| **HashMap wrappers** | Chained `.insert()` on `HashMap_str_*` structs | Supported in v1.2.x+; or use `ptr` + `nyra_map_*` externs (GhostTerm style) |
| **FFI `u8` fields** | `255` inferred as `i32` in some contexts | Annotate field type `u8` on struct; literals in struct literals coerce |
| **REPL vs shell** | `input()` is line-based, not a PTY | Use `forkpty` / C shim (`link-source`) for real terminals |
| **`nyra run .` showcase** | Default may be demo, not interactive shell | Document env flags (e.g. `GHOSTTERM_REPL=1`) in your app README |

**GhostTerm pattern (recommended for systems apps):**

```text
Nyra (tabs, sessions, CLI dispatch) + rt/*.c (PTY) + vendor/bindings (raylib) + nyra.mod link-source
```

Tests: `tests/nyra/break_clone_test.ny` · `tests/nyra/hashmap_chain_test.ny` · `Apps/GhostTerm/` in repo.

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
- Async/`await`: promise handles + **executor v1.4** + **state-machine v1.6** + **v1.7 CFG** (`await` in `if`/`while`/range `for`). `spawn`/`unsafe` with `await` still blocking. **`nyra build --race`** enables TSan. See `async.html`.
- **Struct JSON** — `{Struct}_json_encode/decode` after monomorph; fields: `string`/`i32`/`bool`/nested struct/**`ptr` Vec_i32/fixed `[T; N]`**.
- **`Serialize` trait (v1.38+)** — `u.to_json()` / `u.to_bytes()`; import `stdlib/serde/mod.ny` for trait defs; decode via `{Struct}_json_decode`.
- Arrow functions are **Extended** tier — use `nyra check --deny-extended` in Core-only CI if you avoid them.

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

## Doc map (webDocs)

| Topic | Page |
|-------|------|
| Learn Nyra (W3Schools-style track) | learn-intro.html |
| Language reference (keywords, operators) | reference.html |
| Data structures (arrays, vec, tuples, hashmap) | learn-data-structures.html |
| Language basics + examples | language-basics.html |
| Syntax cheat sheet | language.html |
| Types detail | types.html |
| Imports | imports.html |
| Memory & ownership (full guide) | memory.html |
| Learn ownership / borrowing | learn-ownership.html · learn-borrowing.html |
| Unsafe / no_std | memory.html#unsafe |
| **PGO** | pgo.html |
| **Escape analysis** | escape-analysis.html |
| Performance toolchain | performance.html |
| OS APIs (battery, syscalls) | stdlib.html#os |
| Stdlib API & all builtins | stdlib.html#builtins |
| Backend (TCP/HTTP/JSON) | backend.html |
| net/http API reference | net-http.html |
| C Bindgen & `nyra pkg c` | c-bindgen.html |
| FFI & ABI policy | ffi-abi.html |
| NyraPkg (semver, registry, link-source) | packages.html |
| Runtime bindings (C ↔ stdlib) | bindings.html |
| Integration (bridge, sidecar, FFI) | integration.html |
| Example apps | examples.html |
| **This AI skill file** | ai-skill.html · nyra-skill.md |

Repo docs: [`docs/bindings.md`](../docs/bindings.md) · [`stdlib/README.md`](../stdlib/README.md) · [`webDocs/c-bindgen.html`](../webDocs/c-bindgen.html)
