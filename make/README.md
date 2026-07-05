# Nyra Makefile layout

The root [`Makefile`](../Makefile) is the **only** contributor entry point for tests, builds, installs, and generators.

## Structure

| Path | Role |
|------|------|
| `Makefile` | `make help`, aliases (`make test`, `make build`) |
| `make/common.mk` | Shared variables (`ROOT`, `NYRA_BIN`, optional gates) |
| `make/nyra.mk` | Build `target/debug/nyra` once (`make build-cli`) |
| `make/test.mk` | Unit/integration/conformance/fuzz targets |
| `make/smoke.mk` | Stdlib, CLI, apps, cross-compile smokes |
| `make/build.mk` | Bench, webDocs, packaging |
| `make/install.mk` | Dev install, release installer, LLVM toolchain |
| `make/generators.mk` | Python generators (`make gen-abi-header`, …) |
| `make/test-all.mk` | Full suite orchestration (`make test-all`) |
| `make/test-platform.mk` | macOS/Windows CI core (same `run_gate` aggregation) |
| `make/lib/*.sh` | Internal recipe scripts (invoked by Make targets) |
| `make/py/*.py` | Generator implementations (invoked only via `make`) |
| `scripts/install.sh` | `curl \| sh` installer shim → `make/lib/install.sh` |
| `scripts/install.ps1` | Windows installer |

## Common commands

```bash
make help              # list targets
make test-preflight    # fast pre-check (~1–3 min)
make test-triage       # common CI failures (~5–15 min); see target/.nyra-test-all-failures
make test-all          # full suite (Linux CI; fast gates first)
make test-all-macos    # macOS CI core
make test-all-windows  # Windows CI core
make test-all-core-fast   # quick subset before slow compiletest/fuzz
make test-all-core-slow   # compiletest + fuzz smoke only
make test-conformance  # CONF-LANG only
make smoke-stdlib      # nyra check every stdlib module
make install-dev       # cargo install + stdlib sync
make bench             # cross-language benchmarks
make gen-abi-header    # regenerate stdlib/nyra_rt.h
make gen-bindings-doc  # regenerate bindings docs
make build-webdocs     # webDocs skill + search index
```

## Failure aggregation (`make test-all`, `make test-all-macos`, `make test-all-windows`)

- Gates run **quietly** during the suite (progress bar + gate name only); full output is captured, not streamed.
- Every gate and per-test failure is appended to `target/.nyra-test-all-failures`.
- Failed gate logs are kept under `target/.nyra-test-all-gate-logs/` until the suite ends.
- A complete failure dump is printed **once at the end** in `test-all-summary` / `test-platform-summary`.
- Multi-file smoke scripts (`stdlib-smoke`, `corpus-smoke`, `example-smoke`, `apps-smoke`, `runtime-smoke`) **continue on failure** under `NYRA_TEST_ALL=1` so no error is lost to an early exit.

## GitHub Actions CI (`.github/workflows/ci.yml`)

Staged pipeline — fastest gates first, **parallel within each tier**:

| Stage | Jobs (matrix) | Typical time |
|-------|----------------|--------------|
| 0 build | macOS ∥ Windows | ~3–8 min |
| 1 fast | `test-optional-types`, `test-conformance`, `test-cargo-workspace` | ~1–3 min each |
| 2 medium | `test-nyra-lang`, `smoke-stdlib-priority` | ~3–10 min each |
| 3 heavy | `smoke-stdlib`, `smoke-stdlib-runtime`, `test-runtime-smoke` | ~5–20 min each |
| 4 native | `test-all-*-native`, Windows package + DAP | varies |

Later tiers still run when an earlier tier fails (`if: always`) so all breakages surface in one run. The **CI summary** job lists pass/fail per stage.

Local monolithic targets (`make test-all-macos`, `make test-all-windows`) are unchanged. CI gate env: `NYRA_CI_GATE=<target> make test-platform-ci-tierN`.

## Generator targets (`make/py/`)

| Make target | Purpose |
|-------------|---------|
| `gen-abi-header` | `stdlib/nyra_rt.h` from ABI manifest |
| `gen-bindings-doc` | `docs/bindings.md` + `webDocs/bindings.html` |
| `gen-suite-tests` | `tests/suite/` compiletest corpus |
| `gen-typed-examples` | `.typed.ny` siblings for examples |
| `sync-webdocs-code-tabs` | Sync Without/With types tabs in webDocs |
| `test-webdocs-snippets` | Run self-contained doc snippets (`tests/webdocs/pass-manifest.txt`) |
| `test-webdocs-snippets-full` | Audit all runnable webDocs snippets (may report known failures) |
| `gen-comparison-extended` | Extended comparison benchmark suites |
| `sync-comparison-typed` | Typed comparison mirrors |
| `bench-comparison-html` | Benchmark HTML report |
| `update-readme-bench` | README benchmark section |

## Adding a new gate

1. Add a phony target in the appropriate `make/*.mk` file.
2. If non-trivial, add `make/lib/my-check.sh` and call it from the target.
3. Wire into `make/test-all.mk` when it belongs in CI.
4. Document in `make help` (root `Makefile`).

## Install exception

`scripts/install.sh` remains for the public one-liner:

```bash
curl -fsSL …/scripts/install.sh | sh
```

Use `make install-release` when Make is available locally.
