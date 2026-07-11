// Document JSON via stdlib (formerly NyraPkg ny-serde).
// nyra run examples/serde_json_pkg/main.ny
import "stdlib/json/mod.ny"

fn main() {
    let parsed = parse_json("{\"lang\":\"nyra\",\"version\":1}")
    let out = stringify_json(parsed)
    print(out)
}
