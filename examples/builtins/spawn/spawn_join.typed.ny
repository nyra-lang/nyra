allow_extended
fn main() -> void {
    let h = spawn {
        print(99)
    }
    h.join()
    print(0)
}
