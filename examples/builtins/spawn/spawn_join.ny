allow_extended
fn main() {
    let h = spawn {
        print(99)
    }
    h.join()
    print(0)
}
