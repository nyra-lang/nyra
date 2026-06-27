fn eat(x: string) -> void { print(x) }
fn main() {
    let text = "hello"
    eat(text)
    print(text) //~ ERROR was moved
}
