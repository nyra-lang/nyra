import "tables.typed.ny" as lut

fn test_comptime_import_seed() {
    let x: i32 = lut::SEED
    if x != 42 {
        print("fail seed", x)
    }
}

fn test_comptime_const_fold() {
    const N: i32 = 2 + 3
    if N != 5 {
        print("fail fold", N)
    }
}
