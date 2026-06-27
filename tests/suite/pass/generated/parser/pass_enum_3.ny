enum Tag3 {
    A
    B
    C
}
fn main() {
    let t = Tag3.B
    let n = match t {
        Tag3.A => 1
        Tag3.B => 3
        Tag3.C => 3
    }
    print(n)
}
