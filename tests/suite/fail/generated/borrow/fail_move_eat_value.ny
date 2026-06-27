fn eat(x: string) -> void { print(x) }
fn main() {
    let value = "hello"
    eat(value)
    print(value) //~ ERROR was moved
}
