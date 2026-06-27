fn eat(x: string) -> void { print(x) }
fn main() {
    let msg = "hello"
    eat(msg)
    print(msg) //~ ERROR was moved
}
