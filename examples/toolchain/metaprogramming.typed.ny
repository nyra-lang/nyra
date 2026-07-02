// Metaprogramming (typed) — compile-time code generation without runtime cost.
// nyra run examples/toolchain/metaprogramming.typed.ny

import "stdlib/json/mod.ny"

struct Product {
    id: i32
    name: string
    active: bool
}

macro field_sum(a, b, c) {
    a + b + c
}

const ROUTE_GET = comptime {
    match "GET" {
        "GET" => 1
        "POST" => 2
        _ => 0
    }
}

fn main() {
    let p = Product { id: 42, name: "Nyra", active: true }
    let json = Product_json_encode(p)
    let back = Product_json_decode(json)
    print(back.name)
    print(back.id)

    let total = field_sum(10, 20, 12)
    print(total)
    print(ROUTE_GET)
}
