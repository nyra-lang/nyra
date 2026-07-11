# Language Conformance Tests (CONF-LANG)

Nyra source tests that verify **each language feature behaves as specified**.
Organized by feature area; run via `make/lib/conformance-tests.sh` (included in `make test-all`).

## Layout

| Path | Mode | Expectation |
|------|------|-------------|
| `pass/` | `nyra test` | Compile, link, run; assertions pass |
| `fail/` | `nyra check` | **Must not** compile (type/borrow errors) |
| `fixtures/` | `nyra run` / `nyra test` / `nyra check` | Multi-file import, TLS backends, comptime, no_std |

### Pass areas (`pass/`)

| Directory | Feature |
|-----------|---------|
| `variables/` | `let`, `mut`, arithmetic |
| `control/` | `if`, `while`, `for`, `break`, `continue` |
| `match/` | `match`, `if` expressions |
| `types/` | structs, booleans, i64 literals |
| `functions/` | params, return, inference |
| `arrays/` | literals, indexing, push |
| `enums/` | ADT payloads |
| `strings/` | len, concat, ops |
| `generics/` | monomorph / typed fn |
| `borrow/` | i32 copy, NLL happy paths |
| `edge/` | empty loops, nested expr |
| `imports/` | local const |
| `option/` / `result/` | `Option`, `Result`, `?` propagation; `string_nullish.ny` for `Option<string>` + `??` |
| `hashmap/` | insert/get |
| `async/` | `async`/`await`, executor, promises |
| `spawn/` | `spawn`, `spawn:thread`, join |
| `traits/` | static `impl`, `dyn Trait` dispatch |
| `macros/` | syntactic macro expansion |
| `defer/` | scope-exit hooks on `return` |
| `comptime/` | `#[comptime]` const folding |
| `unsafe/` | `unsafe` blocks, unions, raw pointers |
| `tls/` | rustls availability, HTTP body decode, **deterministic** HTTPS error paths (no soft-skip) |
| `contrib_automation/` | **Python** — `make contribute` / batch tooling (CONF-CONTRIB-PY); see README there |

### Fail areas (`fail/`)

| Directory | Feature |
|-----------|------|
| `assign/` | immutable variable reassignment |
| `borrow/` | use-after-move, double move, conflicting borrows, mut+imm conflicts |
| `types/` | type mismatch in expr / assign |
| `generics/` | monomorph mismatch |
| `option/` | wrong optional payload type |
| `unsafe/` | raw pointer deref outside `unsafe` |
| `no_std/` | `print` rejected in `no_std` programs (`no_std` file directive) |

## Writing a pass test

```ny
import "stdlib/testing.ny"

test fn conf_let_binding() {
    let x = 5
    assert_eq(x, 5)
}
```

Helpers: `assert`, `assert_eq`, `assert_ne`, `assert_true`, `assert_bool`, `assert_str_eq`.

## Writing a fail test

Single-file `fn main()` — no `test fn`. The runner expects `nyra check` to exit non-zero:

```ny
fn main() {
    let x = 1
    x = 2
}
```

## TLS testing strategy

| Suite | What it verifies |
|-------|------------------|
| `pass/tls/offline.ny` | Chunked/plain body decode, status parse, **localhost refused port → JSON error** (always runs) |
| `fixtures/tls_native/` | OS native TLS backend + deterministic error path |
| `fixtures/tls_openssl/` | OpenSSL backend availability or descriptive `tls_last_error` |
| `fixtures/tls_live/` | **Optional** hard live HTTPS (`NYRA_CONF_TLS_LIVE=1`) — no soft-skip |

Live public-Internet HTTPS is **off by default** so CI stays deterministic. Enable explicitly:

```bash
NYRA_CONF_TLS_LIVE=1 bash make/lib/conformance-tests.sh
```

## Run locally

```bash
cargo build -p cli
bash make/lib/conformance-tests.sh
# pass only (nyra test tests/conformance also works — skips fail/ and fixtures/):
./target/debug/nyra test tests/conformance/pass
```

Rebuild the CLI after pulling changes (`cargo build -p cli` or `cargo install --path cli`). Stdlib resolves from the test file path, not only the current directory.

## Relation to other suites

| Suite | Purpose |
|-------|---------|
| `tests/conformance/` (here) | Feature-by-feature pass **and** fail runtime/check |
| `compiler/driver/tests/conformance/` | Rust `CONF-*` compile/IR contracts |
| `tests/suite/` | File-based compiletest at scale (~10k) |
| `tests/nyra/` | Legacy native syntax/ownership smoke |
| `make test-contrib-conformance` | Python hub + batch automation (CONF-CONTRIB-PY) |

When adding a language feature: **≥2 pass** tests under `pass/<area>/` and **≥2 fail** tests under `fail/<area>/`.
