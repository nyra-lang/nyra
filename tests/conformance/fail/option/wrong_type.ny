fn main() {
    let v: Option_i32 = Option_i32.Some(1)
    let _: i32 = v
}

enum Option_i32 {
    None,
    Some(i32),
}
