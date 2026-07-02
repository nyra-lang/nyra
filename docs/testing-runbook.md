# Nyra Testing Runbook

Operational guide for CI failures, regressions, and tier promotion.

## CI pipeline stages

Staged workflow in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml): **build → tier1 (fast) → tier2 (medium) → tier3 (heavy) → native** on each OS, then Windows-only package/DAP extras. Weekly schedule runs extended fuzz on Linux.

| Stage | Gates (matrix per OS) |
|-------|------------------------|
| **0 build** | `make test-platform-ci-build` |
| **1 fast** | `test-optional-types`, `test-conformance`, `test-cargo-workspace` |
| **2 medium** | `test-nyra-lang`, `smoke-stdlib-priority` |
| **3 heavy** | `smoke-stdlib`, `smoke-stdlib-runtime`, `test-runtime-smoke` |
| **4 native** | `make test-all-{linux,macos,windows}-native` |

Local full mirror: `make test-all` (optional `TEST_PERF=1`, `TEST_FUZZ=1`, `NYRA_SUITE_PROFILE=fast` for quicker iteration).

**Gate order:** `test-all` runs **fast → slow** so simple failures surface before heavy compiletest (~3k CI files), fuzz smoke (5×60s), cross-compile, and optional sanitizer/perf/nightly-fuzz gates. Subsets: `make test-all-core-fast`, `make test-all-core-slow`.

**CI platforms (every push/PR):**

| OS | Workflow jobs | Native smoke | Extras |
|----|---------------|--------------|--------|
| **Linux** | `build-linux`, `tier1-linux`, `tier2-linux`, `tier3-linux`, `native-linux` | `make test-all-linux-native` | Weekly `fuzz` job (full compiletest + nightly fuzz) |
| **macOS** | `build-macos`, `tier1-macos`, … | `make test-all-macos-native` | — |
| **Windows** | `build-windows`, `tier1-windows`, … | `make test-all-windows-native` | `windows-package`, `windows-dap` |

Monolithic local targets: `make test-all-linux`, `make test-all-macos`, `make test-all-windows` (platform core + native smoke).

## Detection principles

| Layer | Catches |
|-------|---------|
| Per-crate unit tests | Single-pass bugs (lexer, parser, borrowck, …) |
| Driver integration | End-to-end compile + IR substrings |
| Example corpus | All `examples/` entry points |
| insta snapshots | Unintended IR/diagnostic text changes |
| Conformance (`CONF-*`) | Spec-like behavior contracts |
| Fuzz targets | Parser/compiler panics on random input |
| Perf baseline | Release runtime regressions |

## When CI fails

### `clang-sys` / `llvm-config` / `libclang.so` (Linux CI or local build)

`cargo build --workspace` compiles `nyra-c-bindgen`, which links against **libclang**. On Ubuntu/Debian install development headers, not only the `clang` compiler:

```bash
sudo apt-get install -y clang lld libclang-dev llvm-dev libsqlite3-dev zlib1g-dev libssl-dev
```

CI uses the same packages in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml).

### Snapshot mismatch (`insta`)

- **Intentional change**: run `INSTA_UPDATE=1 cargo test -p compiler --test codegen_snapshots --test diagnostics_snapshots`, review diff, commit `.snap` files.
- **Unintentional**: revert the compiler change or fix the bug; do not update snapshots blindly.

### Perf regression

- Investigate with `scripts/bench.sh`
- Update `benchmarks/ci-baseline.json` only with justification in the PR description

### Example corpus failure

- Check `tests/corpus/manifest.toml` expectations
- If example is broken, fix example or set `expect_compile = false` with a comment

### Conformance failure

- Map failing `CONF-*` test to `docs/conformance/*.md`
- Either fix regression or update spec + test together (never silently delete rules)

## Rollback policy

1. **Core tier regression on `main`** — revert PR immediately or hotfix within hours
2. **Extended tier regression** — may disable via `FeatureSet` flag while fixing
3. **Large LLVM IR shift** — use snapshot review; pin LLVM version if backend-related

## Tier promotion (Extended → Core)

Requirements:

1. All `CONF-*` tests pass without warnings
2. Snapshot review completed
3. RFC updated to Accepted
4. `FeatureSet` flag removed or hard-coded `true`
5. `--deny-extended` tests still pass for remaining Extended features

## Feature flag emergency disable

```rust
CompileOptions {
    features: FeatureSet {
        async_fns: false,
        spawn: false,
        ..FeatureSet::core_only()
    },
    ..Default::default()
}
```

Document the disable in PR and open a follow-up to re-enable.

## Fuzzing (type 7 — libFuzzer + stress)

Two layers:

1. **`fuzz_stress`** — runs on every CI via `cargo test` (2000 borrow + 500 codegen programs).
2. **libFuzzer targets** — in `test-all.sh` and weekly `fuzz-nightly.sh`.

Targets: `fuzz_lexer`, `fuzz_parser`, `fuzz_compile`, `fuzz_gen`, **`fuzz_codegen`**.

```bash
bash scripts/sync-fuzz-corpus.sh
cd fuzz && cargo fuzz run fuzz_codegen --sanitizer none -- \
  -dict=dictionaries/nyra.dict -max_total_time=60
./scripts/test-all.sh
TEST_FUZZ=1 ./scripts/test-all.sh   # + 5min/target nightly gate
```

Dictionary: `fuzz/dictionaries/nyra.dict`. Crashes → minimize → `tests/suite/fail/regression/fuzz/`.

## Contacts / ownership

- Compiler driver tests: `compiler/driver/tests/`
- Corpus manifest: `tests/corpus/manifest.toml`
- ABI manifest: `docs/abi-manifest.toml`
