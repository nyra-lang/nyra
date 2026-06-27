fn save(x: string) -> void { print(x) }
fn main() {
    let value = "hello"
    save(value)
    print(value) //~ ERROR was moved
}
