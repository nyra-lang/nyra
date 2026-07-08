fn main() {
    let x = 42
    let p = &x as *i32
    let v = *p
    print(v)
}
