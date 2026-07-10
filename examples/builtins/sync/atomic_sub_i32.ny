// [contrib-dev:atomic_sub_i32:sync_atomic]
import "stdlib/sync/atomic.ny"

fn main() {
    let a = Atomic_i32_new(10)
    print(atomic_sub_i32(a.cell, 3))
}
// [/contrib-dev:atomic_sub_i32:sync_atomic]
