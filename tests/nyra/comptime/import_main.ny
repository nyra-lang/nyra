import "tables.ny" as lut

fn main() {
    let x = lut::SEED
    if x != 42 {
        print("fail comptime import", x)
    }
}
