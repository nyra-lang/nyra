# Nyra file-based test suite

Small `.ny` files exercised by the `compiletest` harness (`compiler/compiletest/`).

## Layout

| Directory | Expectation |
|-----------|-------------|
| `pass/` | Clean compile (no errors) |
| `fail/` | Compile failure with expected diagnostics |
| `run/` | Compile, link, run; stdout matches `// run-stdout:` |

## Writing a pass test

```ny
fn main() {
    let x = 5
    let y = x + 1
}
```

## Writing a fail test

Use inline directives (substring match against formatted diagnostics):

```ny
fn main() {
    let x = 1
    x = 2 //~ ERROR cannot assign to immutable
}
```

Or add a golden file `my_test.stderr` next to `my_test.ny` for full message snapshots.

## Writing a run test

```ny
// run-stdout: 42
fn main() {
    print(42)
}
```

| `fail/regression/` | Curated regression guards (must never start compiling) |
| `run/generated/` | Generated runtime stdout tests |

## Regenerating combinatorial tests

```bash
make gen-suite-tests                  # ci profile (~3k generated, default)
make gen-suite-tests GEN_SUITE_ARGS="--profile fast"   # fast profile (~1.7k)
make gen-suite-tests GEN_SUITE_ARGS="--profile full"   # full combinatorial (~10k, weekly CI)
make gen-suite-tests GEN_SUITE_ARGS="--dry-run"
```

Generated tests live under `pass/generated/`, `fail/generated/`, `run/generated/`, and `fail/regression/`. Commit them after regeneration.

**Scale:** default **ci** profile (~3k total suite files) — see `tests/suite/.count-baseline`. Use `--profile fast` for quicker local iteration or `--profile full` for exhaustive combinatorial coverage (weekly CI).

Multi-file import tests live under `projects/` (entry point is always `main.ny`).

## Running locally

```bash
# All suite tests (via cargo)
cargo test -p compiler suite_

# Per-test progress on stderr (default). Quiet summary only:
NYRA_SUITE_QUIET=1 cargo test -p compiler suite_

# Filter + live output (recommended for long run shards)
NYRA_SUITE_FILTER=print cargo test -p compiler suite_run_generated_print -- --nocapture

# Standalone CLI
cargo run -p compiletest -- --pass
cargo run -p compiletest -- --fail --filter borrow
cargo run -p compiletest -- --run

# Update golden .stderr files
NYRA_SUITE_UPDATE=1 cargo test -p compiler suite_fail
```

## CI

Included in `make test-all` via `cargo test --workspace` and counted by `make test-count`.

## Contribution rule

Every language feature change should add **≥3 pass** and **≥2 fail** tests in the matching subdirectory.
