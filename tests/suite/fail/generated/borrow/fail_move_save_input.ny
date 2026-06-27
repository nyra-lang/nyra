fn save(x: string) -> void { print(x) }
fn main() {
    let input = "hello"
    save(input)
    print(input) //~ ERROR was moved
}
