fn main() -> void {
    let mut x: i32 = 1
    let r = &mut x
    print(*r)
    x = 2
    print(x)
}
