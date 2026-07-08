// Single-function compile-time evaluation with #[comptime] (normal file, not `comptime` module).
// `mix` is folded away; only `SEED == 42` remains in the binary.

#[comptime]
fn mix(n) {
    return n * 3
}

const SEED = mix(14)

fn main() {
    print("SEED", SEED)
}
