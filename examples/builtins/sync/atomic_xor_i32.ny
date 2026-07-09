// [contrib-dev:atomic_xor_i32:sync_atomic]
import "stdlib/sync/atomic.ny"

fn main() {
    let a = Atomic_i32_new(5)
    print(atomic_xor_i32(a.cell, 3))
}
// [/contrib-dev:atomic_xor_i32:sync_atomic]
