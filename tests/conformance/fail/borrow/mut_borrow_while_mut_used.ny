fn main() {
    let mut v = 1
    let r = &mut v
    let x = r
    v = 2
    print(x)
}
