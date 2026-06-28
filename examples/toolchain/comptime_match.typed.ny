// Typed variant — comptime enum match.

enum Mode {
    Fast
    Slow
}

const SCORE: i32 = comptime {
    let m = Mode.Fast
    match m {
        Mode.Fast => 100
        Mode.Slow => 10
    }
}

fn main() {
    print("SCORE", SCORE)
}
