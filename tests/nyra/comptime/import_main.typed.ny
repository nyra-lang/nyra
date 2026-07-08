import "tables.typed.ny" as lut

fn main() {
    let x: i32 = lut::SEED
    if x != 42 {
        print("fail comptime import", x)
    }
}
