// ny-serde — compatibility shim over stdlib JSON document API.
// Prefer: import "stdlib/json/mod.ny" (parse_json / stringify_json).
import "stdlib/json/mod.ny"

fn from_json(input: string) -> string {
    return parse_json(input)
}

fn to_json(value: string) -> string {
    return stringify_json(value)
}
