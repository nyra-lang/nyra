comptime

fn mix(n: i32) -> i32 {
    return n * 3
}

pub const SEED: i32 = mix(14)
