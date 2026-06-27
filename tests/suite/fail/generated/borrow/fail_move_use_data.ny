fn use(x: string) -> void { print(x) }
fn main() {
    let data = "hello"
    use(data)
    print(data) //~ ERROR was moved
}
