comptime

fn mix(n) {
    return n * 3
}

pub const SEED = mix(14)
