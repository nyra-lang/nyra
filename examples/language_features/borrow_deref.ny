fn main() {
    let mut x = 1
    let r = &mut x
    print(*r)
    x = 2
    print(x)
}
