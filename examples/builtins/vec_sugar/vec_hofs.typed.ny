fn is_even(x: i32) -> i32 {
    if x % 2 == 0 { return 1 }
    return 0
}
fn times2(x: i32) -> i32 { return x * 2 }
fn add(a: i32, b: i32) -> i32 { return a + b }

fn main() -> void {
    let xs = vec().push(1).push(2).push(3).push(4)
    print(xs.len())
    print(xs.contains(3))
    print(xs.filter(is_even).len())
    print(xs.map(times2).get(0))
    print(xs.reduce(0, add))
    print(vec_range(0, 3).len())
}
