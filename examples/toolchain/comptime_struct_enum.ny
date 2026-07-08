// Comptime struct literals, field access, struct/tuple match, and enums.

struct Vec2 {
    x: i32
    y: i32
}

const LEN_SQ = comptime {
    let v = Vec2 { x: 3, y: 4 }
    match v {
        Vec2 { x, y } => x * x + y * y
    }
}

fn main() {
    print("LEN_SQ", LEN_SQ)
}
