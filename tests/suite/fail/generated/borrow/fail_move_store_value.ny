fn store(x: string) -> void { print(x) }
fn main() {
    let value = "hello"
    store(value)
    print(value) //~ ERROR was moved
}
