fn use(x: string) -> void { print(x) }
fn main() {
    let input = "hello"
    use(input)
    print(input) //~ ERROR was moved
}
