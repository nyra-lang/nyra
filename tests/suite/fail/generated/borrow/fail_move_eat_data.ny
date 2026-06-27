fn eat(x: string) -> void { print(x) }
fn main() {
    let data = "hello"
    eat(data)
    print(data) //~ ERROR was moved
}
