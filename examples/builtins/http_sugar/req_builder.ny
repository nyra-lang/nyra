fn main() {
    let init = req().timeout(1000).header("X-Test", "1")
    print(init.timeout_ms)
}
