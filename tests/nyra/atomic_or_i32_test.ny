// [contrib-dev:atomic_or_i32:sync_atomic]
import "stdlib/testing.ny"
import "stdlib/sync/atomic.ny"

test fn test_atomic_or_i32() {
    let a = Atomic_i32_new(1)
    assert_eq(atomic_or_i32(a.cell, 2), 3)
}
// [/contrib-dev:atomic_or_i32:sync_atomic]
