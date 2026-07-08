import "tables.ny" as lut
import "stdlib/testing.ny"

test fn conf_comptime_003_imported_const() {
    assert_eq(lut::SEED, 42)
}
