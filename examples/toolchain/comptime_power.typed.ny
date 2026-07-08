// Optional Zig-style comptime power (explicit types variant).
// Check: nyra check examples/toolchain/comptime_power.typed.ny

comptime

fn square(n: i32) -> i32 {
    return n * n
}

fn build_lookup(size: i32) -> [i32; 8] {
    let mut table: [i32; 8] = [0; 8]
    let mut i: i32 = 0
    while i < size {
        table[i] = square(i)
        i = i + 1
    }
    return table
}

pub const LOOKUP: [i32; 8] = build_lookup(8)
pub const GET_CODE: i32 = comptime {
    match "GET" {
        "GET" => 1
        "POST" => 2
        "PUT" => 3
        _ => 0
    }
}
pub const LABEL: string = comptime {
    let prefix: string = "nyra"
    let suffix: string = "-comptime"
    prefix + suffix
}
pub const LOOKUP_AT_3: i32 = comptime {
    let t: [i32; 8] = build_lookup(8)
    t[3]
}
