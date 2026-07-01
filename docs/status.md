# Nyra project status

> Canonical matrix for README and roadmap. Updated with v1.7 race detector + async CFG + collection JSON.

## Summary

| Area | Status |
|------|--------|
| **Overall** | **Production-ready** — Core + Stable Extended (v1.7 TSan + async CFG + collection JSON) |
| **Core tier** | **Stable** (semver 1.0+) |
| **Stable Extended** | **Stable** — async CFG desugar, traits + `dyn Send/Sync`, struct JSON (nested + collections), macros, defer |
| **Extended preview** | **None** |
| **Toolchain** | CLI **done**; **`nyra build --race` (TSan)**; LSP depth (semantic tokens, inlay hints, code actions, signature help) |
| **Releases** | Linux, macOS, **Windows prebuilt** (GitHub Releases) |

## Language

| Feature | Status |
|---------|--------|
| LLVM codegen + release/LTO/PGO | Shipped |
| Zero-types + explicit types | Shipped — no runtime overhead |
| Ownership / borrow / NLL | Shipped |
| Generics (monomorph) | Core stable |
| Enum tags | Core stable |
| Enum payloads | **Stable Extended** |
| `?` propagation | **Stable Extended** — `tests/nyra/result_propagate_test.ny` |
| `spawn` + channels | **Stable Extended** |
| `impl Drop` | **Stable Extended** |
| `async` / `await` | **Stable Extended** — executor v1.4 + state-machine v1.6–v1.7 + **v1.26 `Future<T>` + select** |
| Traits / `dyn` | **Stable Extended** — multi-method vtables, **`dyn Trait + Send + Sync`** bounds with **Send/Sync validation**, trait-object **`Drop`** (heap free) |
| Macros | **Stable Extended** |
| Lifetimes / defer | **Stable Extended** |
| JSON nested + bool | **Stable Extended** |
| Struct `{Name}_json_encode/decode` | **Stable Extended** — nested structs + **`ptr`/`Vec`/fixed `[T; N]`/`StrVec`** (post-monomorph) |
| **`Serialize` / `Deserialize` traits** | **Stable Extended** — `to_json`/`to_bytes`/`from_json`; NBF v1 binary for scalar/nested structs |
| `Vec<string>` generic syntax | **Stable Extended** — aliases to `StrVec` |
| `Matrix2D` / `RowVec` | **Shipped** — dynamic 2D grid + Move-safe string rows |
| C-style `union` | **Shipped** — `union U repr(C) { ... }`, field access in `unsafe` |
| Layout / alignment | **Shipped** — `repr(C)`, `align(N)`, `packed`, `size_of<T>()` / `align_of<T>()` |
| Heterogeneous enum payloads | **Shipped** — per-variant payload union slot + tag-discriminated drop |
| `bytes` type | **Shipped** — distinct from `string`; `.len()`, `[i32]`, `.to_string()` |
| `StackBuffer[T; N]` | **Shipped** — stack-only wrapper (`stdlib/buf/stack.ny`); return rejected |
| Portable SIMD | **Shipped** — `i32x4` / `f32x4` / `f64x2` + `simd_*` intrinsics |
| Platform SIMD | **Shipped** — `stdlib/simd/x86.ny`, `arm.ny` behind `unsafe` + CPU checks |
| Arena allocator | **Shipped** — `stdlib/alloc/arena.ny` + `rt_arena.c` bump allocator |

## Conformance

| Suite | Status |
|-------|--------|
| `tests/conformance/` (CONF-LANG) | Pass + fail runtime/check gates |
| `scripts/conformance-tests.sh` | Wired in `test-all.sh` |
| Rust `CONF-*` driver tests | Ownership, ADT, coercion |

## Stdlib

| Domain | Status |
|--------|--------|
| Collections, fs, strings, time | Shipped |
| crypto (SHA/HMAC/AES) | Shipped |
| net/http, tcp, websocket, udp | Shipped |
| db/sqlite | Shipped (`link sqlite3`) |
| db/postgres, db/mysql | Native when libpq/mysqlclient linked |
| env_get / **env_set** | Shipped (POSIX + Windows) |
| process / Command | Shipped (POSIX + Windows) |
| json/serialize | **Stable Extended** — nested JSON + struct synthesis; NyraPkg for advanced schemas |

## Platforms

| Platform | Prebuilt release | Notes |
|----------|-------------------|--------|
| Linux x86_64 / arm64 | Yes | `install.sh` |
| macOS x86_64 / arm64 | Yes | `install.sh` |
| Windows x86_64 | Yes | `install.ps1` + `nyra-x86_64-windows.zip` |
| Cross-compile | Yes | `--for windows|linux|macos` |

## Ecosystem

| Component | Status |
|-----------|--------|
| NyraPkg | Semver registry, `link` lines |
| Editor grammar | TextMate `grammar/nyra.tmLanguage.json` |

## Not yet production gates

- `dyn A + B` multi-trait objects; full auto-trait checking on fn params
- Generic struct fields beyond monomorph instances (manual serde for exotic `T`)

Native race runtime ships as **`--race-native`**; TSan remains **`--race`**.

See [`docs/stability-v1.md`](stability-v1.md) and [`webDocs/roadmap.html`](../webDocs/roadmap.html).
