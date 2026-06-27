fn eat(x: string) -> void { print(x) }
fn main() {
    let input = "hello"
    eat(input)
    print(input) //~ ERROR was moved
}
