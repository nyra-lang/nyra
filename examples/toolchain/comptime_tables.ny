// Comptime lookup table — evaluated at compile time, zero runtime cost for SEED.
// Check module: nyra check examples/toolchain/comptime_tables.ny
// Use from runtime: import this file and read `tables.SEED`.

comptime

fn hash_step(n) {
    return (n * 2654435761) % 2147483647
}

fn sum_array(values) {
    let mut acc = 0
    for x in values {
        acc = acc + hash_step(x)
    }
    return acc
}

pub const SEED = hash_step(42)
pub const SUM_FOUR = sum_array([0, 1, 2, 3])
