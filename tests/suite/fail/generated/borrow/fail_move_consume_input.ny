fn consume(x: string) -> void { print(x) }
fn main() {
    let input = "hello"
    consume(input)
    print(input) //~ ERROR was moved
}
