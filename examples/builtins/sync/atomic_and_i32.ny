// [contrib-dev:atomic_and_i32:sync_atomic]
import "stdlib/sync/atomic.ny"

fn main() {
    let a = Atomic_i32_new(7)
    print(atomic_and_i32(a.cell, 3))
}
// [/contrib-dev:atomic_and_i32:sync_atomic]
