// [contrib-dev:atomic_xor_i32:sync_atomic]
import "stdlib/testing.ny"
import "stdlib/sync/atomic.ny"

test fn test_atomic_xor_i32() {
    let a = Atomic_i32_new(5)
    assert_eq(atomic_xor_i32(a.cell, 3), 6)
}
// [/contrib-dev:atomic_xor_i32:sync_atomic]
