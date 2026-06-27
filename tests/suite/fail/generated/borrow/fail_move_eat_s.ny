fn eat(x: string) -> void { print(x) }
fn main() {
    let s = "hello"
    eat(s)
    print(s) //~ ERROR was moved
}
