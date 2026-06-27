fn send(x: string) -> void { print(x) }
fn main() {
    let value = "hello"
    send(value)
    print(value) //~ ERROR was moved
}
