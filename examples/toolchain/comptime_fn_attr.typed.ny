// Typed variant — same behavior as comptime_fn_attr.ny.

#[comptime]
fn mix(n: i64) -> i64 {
    return n * 3
}

const SEED: i64 = mix(14)

fn main() {
    print("SEED", SEED)
}
