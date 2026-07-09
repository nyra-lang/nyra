// [contrib-dev:atomic_sub_i32:sync_atomic]
import "stdlib/testing.ny"
import "stdlib/sync/atomic.ny"

test fn test_atomic_sub_i32() {
    let a = Atomic_i32_new(10)
    assert_eq(atomic_sub_i32(a.cell, 3), 7)
}
// [/contrib-dev:atomic_sub_i32:sync_atomic]
