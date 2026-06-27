fn store(x: string) -> void { print(x) }
fn main() {
    let input = "hello"
    store(input)
    print(input) //~ ERROR was moved
}
