fn use(x: string) -> void { print(x) }
fn main() {
    let value = "hello"
    use(value)
    print(value) //~ ERROR was moved
}
