fn use(x: string) -> void { print(x) }
fn main() {
    let s = "hello"
    use(s)
    print(s) //~ ERROR was moved
}
