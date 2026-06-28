// Comptime `match` on enums, bools, and integers (with guards).

enum Mode {
    Fast
    Slow
}

const SCORE = comptime {
    let m = Mode.Fast
    match m {
        Mode.Fast => 100
        Mode.Slow => 10
    }
}

fn main() {
    print("SCORE", SCORE)
}
