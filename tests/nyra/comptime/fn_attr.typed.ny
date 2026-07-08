#[comptime]
fn mix(n: i64) -> i64 {
    return n * 3
}

const SEED: i64 = mix(14)

fn main() {
    if SEED != 42 {
        print("fail fn_attr seed", SEED)
    }
}
