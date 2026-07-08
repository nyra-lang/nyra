# Language Conformance Tests (CONF-LANG)

Nyra source tests that verify **each language feature behaves as specified**.
Organized by feature area; run via `scripts/conformance-tests.sh` (included in `test-all.sh`).

## Layout

| Path | Mode | Expectation |
|------|------|-------------|
| `pass/` | `nyra test` | Compile, link, run; assertions pass |
| `fail/` | `nyra check` | **Must not** compile (type/borrow errors) |
| `fixtures/` | `nyra run` | Multi-file import smoke |

### Pass areas (`pass/`)

| Directory | Feature |
|-----------|---------|
| `variables/` | `let`, `mut`, arithmetic |
| `control/` | `if`, `while`, `for` |
| `match/` | `match`, `if` expressions |
| `types/` | structs, booleans |
| `functions/` | params, return, inference |
| `arrays/` | literals, indexing |
| `enums/` | ADT payloads |
| `strings/` | len, concat |
| `generics/` | monomorph / typed fn |
| `borrow/` | i32 copy (happy path) |
| `edge/` | empty loops, nested expr |
| `imports/` | local const (project import via fixture) |
| `tls/` | rustls availability, chunked HTTPS body decode, live HTTPS soft-skip (CONF-TLS-*); fixtures `tls_native` / `tls_openssl` cover the other backends |

### Fail areas (`fail/`)

| Directory | Feature |
|-----------|---------|
| `assign/` | immutable variable reassignment |
| `borrow/` | use-after-move (string + struct) |
| `types/` | type mismatch in expr / assign |

## Writing a pass test

```ny
import "stdlib/testing.ny"

test fn conf_let_binding() {
    let x = 5
    assert_eq(x, 5)
}
```

Helpers: `assert`, `assert_eq`, `assert_ne`, `assert_true`, `assert_bool`.

## Writing a fail test

Single-file `fn main()` — no `test fn`. The runner expects `nyra check` to exit non-zero:

```ny
fn main() {
    let x = 1
    x = 2
}
```

## Run locally

```bash
cargo build -p cli
bash scripts/conformance-tests.sh
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

When adding a language feature: **≥2 pass** tests under `pass/<area>/` and **≥2 fail** tests under `fail/<area>/`.
