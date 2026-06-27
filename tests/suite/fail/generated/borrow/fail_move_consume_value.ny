fn consume(x: string) -> void { print(x) }
fn main() {
    let value = "hello"
    consume(value)
    print(value) //~ ERROR was moved
}
