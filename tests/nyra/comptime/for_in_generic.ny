comptime

fn id<T>(x: T) -> T {
    return x
}

fn sum_array(values) {
    let mut acc = 0
    for x in values {
        acc = acc + id(x)
    }
    return acc
}

priv const VALUES = [1, 2, 3, 4]
pub const TOTAL = sum_array(VALUES)
