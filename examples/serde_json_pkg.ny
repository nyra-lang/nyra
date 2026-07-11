// Document JSON via stdlib — same API as former NyraPkg ny-serde.
// Project demo: examples/serde_json_pkg/main.ny
import "stdlib/json/mod.ny"

fn main() {
    let raw = stringify_json("{\"name\":\"nyra\",\"n\":1}")
    print(raw)
}
