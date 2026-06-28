import "comptime_tables.ny" as tables

fn main() {
    let seed = tables::SEED
    let sum4 = tables::SUM_FOUR
    if seed <= 0 || sum4 <= 0 {
        print("unexpected comptime values")
    }
}
