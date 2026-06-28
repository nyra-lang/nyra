# Comptime table benchmark

Compares **runtime table build + sum** (all languages) vs **Nyra comptime** where the
same work is folded at compile time (`bench_comptime.ny`).

- `bench.ny` / `bench_typed.ny` — runtime baseline (Nyra zero-types / explicit types)
- `bench_comptime.ny` / `bench_comptime_typed.ny` — full fold via `comptime { }`
- `bench.c`, `bench.cpp`, `bench.go`, `bench.rs` — same algorithm in other languages

Run via `make bench` (suite `comptime_table`) or quick subset: `BENCH_QUICK=1 make bench`.

Expect **Nyra (Comptime)** to be orders of magnitude faster than runtime Nyra/C/Go/Rust on
this suite — the hot loops disappear from the binary.
