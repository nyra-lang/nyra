fn take(x: string) -> void { print(x) }
fn main() {
    let value = "hello"
    take(value)
    print(value) //~ ERROR was moved
}
