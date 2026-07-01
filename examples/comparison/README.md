# Fair comparison benchmarks

Same algorithm per subfolder across **Nyra (zero types)**, **Nyra (typed)**, **C**, **C++**, **Go**, **Rust**, **Node**, **Python**, and **Java**.

Heavy CPU suites use **modular arithmetic** so every language (including Nyra `i32`) produces the same checksum without overflow.

| Folder | What it measures | Expected output |
|--------|------------------|-----------------|
| [hello/](hello/) | Minimal I/O | `Hello Nyra` |
| [arithmetic/](arithmetic/) | Two adds + one print | `30` |
| [loop/](loop/) | Modular sum 0..N-1 (N = 375M) | `320312507` |
| [loop_nofold/](loop_nofold/) | Same loop, anti-constant-fold | `320312507` |
| [fib/](fib/) | Fibonacci swaps (375M steps, mod 1e9+7) | `751659594` |
| [nested/](nested/) | 2D nested loop (4000 × 4000, mod 1e9+7) | `3552224` |
| [struct_sum/](struct_sum/) | Copy struct fields in hot loop (80M) | `240000000` |
| [cpu_bound/](cpu_bound/) | Mod mul-add chain (180M, mod 997) | `415` |
| *(bench only)* `cpu_bound_pgo` | Same as cpu_bound, Nyra `--release --pgo` | `415` |
| [mix/](mix/) | Chained mod mix (270M, mod 1e9+7) | `473067162` |
| [escape/](escape/) | **Escape analysis** — LocalChannel vs spawn, SROA | see [escape/README.md](escape/README.md) |
| [dungeon/](dungeon/) | **Dungeon Steps** app — zero types (`dungeon/`) | see [dungeon/README.md](dungeon/README.md) |
| [dungeon_typed/](dungeon_typed/) | Same app with explicit types | same output as `dungeon/` |

### Extended suites (memory, strings, collections, algorithms, concurrency)

23 additional suites under [`memory/`](memory/), [`strings/`](strings/), [`collections/`](collections/), [`algorithms/`](algorithms/), [`concurrency/`](concurrency/). Regenerate with `python3 scripts/gen-comparison-extended.py`. Full list and expected checksums: [`extended/README.md`](extended/README.md).

| Category | Suites |
|----------|--------|
| Memory | `alloc_struct`, `free_struct`, `arena`, `ownership` |
| Strings | `concat`, `substring`, `replace`, `split`, `utf8` |
| Collections | `hashmap`, `hashset`, `vec_push`, `vec_pop`, `sort` |
| Algorithms | `qsort`, `mergesort`, `binary_search`, `json_parse`, `regex` |
| Concurrency | `spawn_tasks`, `channel_pingpong`, `worker_pool`, `parallel_map` |

Scale iteration counts: `BENCH_SCALE=20 python3 scripts/gen-comparison-extended.py` (then re-sync typed). Skip extended benches at runtime: `BENCH_EXTENDED=0 ./scripts/bench.sh`.

### Nyra dual entries

| Entry | Source | Role |
|-------|--------|------|
| **Nyra (Zero Types)** | `*.ny`, `dungeon/` | Zero-types style (inference; structs only where required) |
| **Nyra (Explicit Types)** | `*_typed.ny`, `dungeon_typed/` | Same algorithm with explicit type annotations |

The HTML report includes a dedicated **Nyra zero types vs typed** table (Δ time per benchmark) — proof that annotations do not change performance.

Regenerate typed mirrors: `python3 scripts/sync-comparison-typed.py`

### Nyra v0.2+ coverage

| Suite | Language features exercised |
|-------|----------------------------|
| `loop` | `mut`, `while`, `%` modular arithmetic |
| `fib` | `while`, mutable swap, mod arithmetic |
| `nested` | nested `for`, multiply-add inner loop |
| `struct_sum` | `struct` literals, field access, Copy semantics |
| `cpu_bound` | `for`, mul/add/mod hot path, blackbox sink |
| `mix` | chained mul-add-mod (LCG-style mixing) |
| `escape` | LocalChannel vs runtime channel, SROA structs, NoEscape strings |
| `dungeon` | multi-file `import`, `match`, enums, tests |

## Run (smoke test)

```bash
# Nyra zero-types — start with nested/fib (seconds); loop/fib/mix take minutes at full N
cargo run --bin nyra -- run examples/comparison/nested/nested.ny
cargo run --bin nyra -- run examples/comparison/fib/fib.ny
cargo run --bin nyra -- run examples/comparison/cpu_bound/bench.ny

# Nyra typed (same checksums)
cargo run --bin nyra -- run examples/comparison/nested/nested_typed.ny
cargo run --bin nyra -- run examples/comparison/fib/fib_typed.ny
cargo run --bin nyra -- run examples/comparison/cpu_bound/bench_typed.ny
cargo run --bin nyra -- run examples/comparison/dungeon_typed

# Parity gate (all languages, quick subset)
bash scripts/check-comparison-parity.sh

# Go / Rust / Node / Python / Java — same folders, matching constants
go run examples/comparison/nested/nested.go
rustc -O examples/comparison/nested/nested.rs -o /tmp/nested_rust && /tmp/nested_rust
```

## Runtime benchmark

```bash
./scripts/bench.sh
```

Opens **`results/latest.txt`** and **`results/latest.html`** (Nyra, Nyra-typed, C, C++, Go, Rust, Node, Python, Java).

Requires on `PATH`: `clang`/`clang++` (or `CC`/`CXX`), `rustc`, `go`, `node`, `python3`, `javac`, `java` (missing tools are skipped with a warning).

Release Nyra builds are the default. Set `BENCH_RELEASE=0` for debug builds (less RAM/CPU while compiling).

**Report columns:** Language · **Time** (median ms by default) · **Memory** (peak RSS) · **Binary size** (hello-world release / stripped / UPX).

**Nyra Zero vs Explicit (clean parity):** each suite alternates Zero Types and Explicit Types every round; reported time is the **median** across timed runs (`BENCH_STAT=median`, override with `mean`). Hot-path parity in the HTML report excludes `hello` / `arithmetic` (spawn-dominated; LLVM is identical). Micro and concurrency suites use extra runs: `BENCH_MICRO_RUNS=9`, `BENCH_CONCURRENCY_RUNS=11`.

Optional: `BENCH_PGO=1` builds **every** Nyra suite with `--pgo` (slow). By default, `./scripts/bench.sh` always runs **`cpu_bound_pgo`** for both **Nyra** and **Nyra-typed** (same hot path, `--release --pgo`). Skip with `BENCH_SKIP_PGO=1`. Requires LLVM `opt` + `llvm-profdata` on `PATH`.

Skip hello binary-size table: `BENCH_BINARY_SIZE=0`. Install [UPX](https://upx.github.io/) for the UPX column (often `—` on macOS arm64 Mach-O).

See also [`PERFORMANCE_ROADMAP.md`](../../PERFORMANCE_ROADMAP.md).

By default **Nyra variants run per suite** (interleaved Zero/Explicit pairs), then **other languages run in isolation** (all suites for C, cooldown, then C++, …) so cross-language comparison stays fair. Override: `BENCH_LANG_COOLDOWN=3` (seconds between languages, default `2`), or `BENCH_NO_ISOLATE=1` for every language per suite back-to-back.

Full benchmark run can take **tens of minutes** (375M-iteration suites). Tune down for quick checks: `BENCH_RUNS=1 BENCH_WARMUP=0`.

Release profile uses LLVM `opt`, clang `-O3`, and thin LTO — see [`docs/performance.md`](../../docs/performance.md).
