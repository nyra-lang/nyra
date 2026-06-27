fn take(x: string) -> void { print(x) }
fn main() {
    let input = "hello"
    take(input)
    print(input) //~ ERROR was moved
}
