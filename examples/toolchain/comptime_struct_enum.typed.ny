// Typed variant — comptime struct + enum.

struct Vec2 {
    x: i32
    y: i32
}

enum Kind {
    A
    B
}

const SCORE: i32 = comptime {
    let v = Vec2 { x: 1, y: 2 }
    let k = Kind.B
    match k {
        Kind.A => v.x
        Kind.B => v.x + v.y
    }
}

fn main() {
    print("SCORE", SCORE)
}
