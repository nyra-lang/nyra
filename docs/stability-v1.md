# Nyra stability policy (v1.2)

Nyra splits the language into **Core** and **Stable Extended**. As of v1.2, all former Extended preview features are **Stable Extended** — no `warning[W001]`.

## Core (stable)

Types, inference, control flow, modules, structs, enum tags, ownership, borrow/NLL, generics (monomorph), FFI, `unsafe`/`no_std`, CLI toolchain.

## Stable Extended (v1.2 — all ship without W001)

| Feature | Notes |
|---------|--------|
| **`?` operator** | Result/Option propagation |
| **Enum payloads** | `Some(T)`, `Ok(T)` / `Err(E)` |
| **`spawn { }` + channels** | pthread / Win32 |
| **`impl Drop for T`** | Custom RAII |
| **`async` / `await`** | i32 promise handles + **executor v1.4** (`runtime_executor_tick`, `async_sleep_ms`, `Executor_run_until`) |
| **`trait` / `impl` / `dyn Trait`** | Static dispatch + vtable MVP (struct-by-value) |
| **`macro`** | Syntactic substitution; multi-param; expands in blocks/impls |
| **Explicit lifetimes / HRTB** | Borrowck + lifetime pass |
| **`defer`** | LIFO on block fall-through and `return` |
| **Struct spread** | `..base` in struct literals |
| **Stdlib JSON/serialize** | Flat + nested object encode/decode + document `parse_json`/`stringify_json` in `rt_json.c` |

`nyra check --deny-extended` — reserved for **future** preview features (none in v1.2).

## Stdlib serde tiers

| Tier | API |
|------|-----|
| **Document JSON** | `json/mod.ny` — `parse_json` / `stringify_json` (validate + compact) |
| **Field / object** | `json/mod.ny` — `decode_*`, `encode_object`, `JSON_parse_object`, nested objects |
| **Schema traits** | `serde/mod.ny` — `Serialize` / `Deserialize` + compiler `{Struct}_json_encode/decode` |

## Releases

Linux, macOS, Windows prebuilts — `install.sh` / `install.ps1`.

## Breaking changes

Core + Stable Extended: RFC + semver minor bump. See [`docs/status.md`](status.md).
