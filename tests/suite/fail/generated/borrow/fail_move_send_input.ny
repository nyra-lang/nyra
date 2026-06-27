fn send(x: string) -> void { print(x) }
fn main() {
    let input = "hello"
    send(input)
    print(input) //~ ERROR was moved
}
