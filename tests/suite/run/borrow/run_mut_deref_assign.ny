// run-stdout: 1
// run-stdout: 2
fn main() {
    let mut x = 1
    let r = &mut x
    print(*r)
    x = 2
    print(x)
}
