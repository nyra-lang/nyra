// [contrib-dev:atomic_and_i32:sync_atomic]
import "stdlib/testing.ny"
import "stdlib/sync/atomic.ny"

test fn test_atomic_and_i32() {
    let a = Atomic_i32_new(7)
    assert_eq(atomic_and_i32(a.cell, 3), 3)
}
// [/contrib-dev:atomic_and_i32:sync_atomic]
