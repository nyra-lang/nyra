enum Opt3 {
    None
    Some(i32)
}
fn main() {
    let o = Opt3.Some(3)
    let v = match o {
        Opt3.None => 0
        Opt3.Some(x) => x
    }
    print(v)
}
