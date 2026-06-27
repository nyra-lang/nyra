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
| `make/lib/*.sh` | Internal recipe scripts (invoked by Make targets) |
| `make/py/*.py` | Generator implementations (invoked only via `make`) |
| `scripts/install.sh` | `curl \| sh` installer shim → `make/lib/install.sh` |
| `scripts/install.ps1` | Windows installer |

## Common commands

```bash
make help              # list targets
make test-preflight    # fast pre-check (~1–3 min)
make test-all          # full suite (CI core; fast gates first; failures collected at end)
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

## Generator targets (`make/py/`)

| Make target | Purpose |
|-------------|---------|
| `gen-abi-header` | `stdlib/nyra_rt.h` from ABI manifest |
| `gen-bindings-doc` | `docs/bindings.md` + `webDocs/bindings.html` |
| `gen-suite-tests` | `tests/suite/` compiletest corpus |
| `gen-typed-examples` | `.typed.ny` siblings for examples |
| `sync-webdocs-code-tabs` | Sync Without/With types tabs in webDocs |
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
