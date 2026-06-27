fn use(x: string) -> void { print(x) }
fn main() {
    let msg = "hello"
    use(msg)
    print(msg) //~ ERROR was moved
}
