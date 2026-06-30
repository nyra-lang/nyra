// nyra test tests/suite/pass/generated/borrow/string_auto_borrow.ny
fn greet(s: &string) -> void {
    print(s)
}

fn main() {
    let msg = "borrowed"
    greet(msg)
    print(msg)
}
