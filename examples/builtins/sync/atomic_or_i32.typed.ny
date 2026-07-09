// [contrib-dev:atomic_or_i32:sync_atomic]
import "stdlib/sync/atomic.ny"

fn main() -> void {
    let a = Atomic_i32_new(1)
    print(atomic_or_i32(a.cell, 2))
}
// [/contrib-dev:atomic_or_i32:sync_atomic]
