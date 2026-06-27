fn main() {
    let mut v = 2
    let r = &v
    v = v + 1 //~ ERROR because it is borrowed
    print(r)
}
