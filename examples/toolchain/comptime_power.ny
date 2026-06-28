// Optional Zig-style comptime power: tables, strings, and match — zero runtime cost.
// Check: nyra check examples/toolchain/comptime_power.ny

comptime

fn square(n) {
    return n * n
}

fn build_lookup(size) {
    let mut table = [0; size]
    let mut i = 0
    while i < size {
        table[i] = square(i)
        i = i + 1
    }
    return table
}

pub const LOOKUP = build_lookup(8)
pub const GET_CODE = comptime {
    match "GET" {
        "GET" => 1
        "POST" => 2
        "PUT" => 3
        _ => 0
    }
}
pub const LABEL = comptime {
    let prefix = "nyra"
    let suffix = "-comptime"
    prefix + suffix
}
pub const LOOKUP_AT_3 = comptime {
    let t = build_lookup(8)
    t[3]
}
