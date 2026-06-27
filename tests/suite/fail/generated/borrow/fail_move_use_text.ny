fn use(x: string) -> void { print(x) }
fn main() {
    let text = "hello"
    use(text)
    print(text) //~ ERROR was moved
}
