// Typed variant of comptime_tables.ny — same compile-time evaluation.

comptime

fn hash_step(n: i32) -> i32 {
    return (n * 2654435761) % 2147483647
}

fn sum_array(values: [i32; 4]) -> i32 {
    let mut acc: i32 = 0
    for x in values {
        acc = acc + hash_step(x)
    }
    return acc
}

pub const SEED: i32 = hash_step(42)
pub const SUM_FOUR: i32 = sum_array([0, 1, 2, 3])
