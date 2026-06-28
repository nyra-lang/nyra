#[comptime]
fn mix(n) {
    return n * 3
}

const SEED = mix(14)

fn test_comptime_fn_attr_fold() {
    if SEED != 42 {
        print("fail fn_attr fold", SEED)
    }
}
