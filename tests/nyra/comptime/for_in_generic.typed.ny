comptime

fn id<T>(x: T) -> T {
    return x
}

fn sum_array(values: [i32; 4]) -> i32 {
    let mut acc: i32 = 0
    for x in values {
        acc = acc + id(x)
    }
    return acc
}

priv const VALUES: [i32; 4] = [1, 2, 3, 4]
pub const TOTAL: i32 = sum_array(VALUES)
