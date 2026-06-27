fn use(x: string) -> void { print(x) }
fn main() {
    let name = "hello"
    use(name)
    print(name) //~ ERROR was moved
}
