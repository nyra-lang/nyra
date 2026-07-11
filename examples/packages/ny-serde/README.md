# ny-serde (compatibility shim)

Document JSON (`parse_json` / `stringify_json`) now ships in **stdlib**.

Prefer:

```ny
import "stdlib/json/mod.ny"

fn main() {
    let obj = parse_json("{\"ok\":true}")
    print(stringify_json(obj))
}
```

This package re-exports the same API (plus `from_json` / `to_json` aliases) for older `nyrapkg install ny-serde` projects. No `nyra bind` or `link-crate` is required.

## Test

```bash
nyra test examples/packages/ny-serde/serde_test.ny
```
