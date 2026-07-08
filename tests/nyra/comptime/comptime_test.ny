import "tables.ny" as lut

fn test_comptime_import_seed() {
    let x = lut::SEED
    if x != 42 {
        print("fail seed", x)
    }
}

fn test_comptime_const_fold() {
    const N = 2 + 3
    if N != 5 {
        print("fail fold", N)
    }
}
