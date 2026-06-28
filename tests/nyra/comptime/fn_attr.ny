#[comptime]
fn mix(n) {
    return n * 3
}

const SEED = mix(14)

fn main() {
    if SEED != 42 {
        print("fail fn_attr seed", SEED)
    }
}
