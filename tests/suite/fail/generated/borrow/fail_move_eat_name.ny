fn eat(x: string) -> void { print(x) }
fn main() {
    let name = "hello"
    eat(name)
    print(name) //~ ERROR was moved
}
