extern fn atomic_load_i32(p: ptr) -> i32
extern fn atomic_store_i32(p: ptr, v: i32) -> void
extern fn atomic_add_i32(p: ptr, delta: i32) -> i32
extern fn atomic_cas_i32(p: ptr, expected: i32, desired: i32) -> i32
extern fn atomic_i32_new(initial: i32) -> ptr
extern fn atomic_i32_free(p: ptr) -> void

struct Atomic_i32 {
    cell: ptr
}

fn Atomic_i32_new(initial: i32) -> Atomic_i32 {
    return Atomic_i32 { cell: atomic_i32_new(initial) }
}

impl Atomic_i32 {
    fn load(self) -> i32 {
        return atomic_load_i32(self.cell)
    }

    fn store(self, v: i32) -> Atomic_i32 {
        atomic_store_i32(self.cell, v)
        return Atomic_i32 { cell: self.cell }
    }

    fn add(self, delta: i32) -> i32 {
        return atomic_add_i32(self.cell, delta)
    }

    fn compare_and_swap(self, expected: i32, desired: i32) -> i32 {
        return atomic_cas_i32(self.cell, expected, desired)
    }
}

impl Drop for Atomic_i32 {
    fn drop(self) -> void {
        atomic_i32_free(self.cell)
    }
}

// AtomicBool — bool wrapper over Atomic_i32 (0/1). Lock-free on supported targets.

struct AtomicBool {
    inner: Atomic_i32
}

fn AtomicBool_new(initial: bool) -> AtomicBool {
    let v = if initial { 1 } else { 0 }
    return AtomicBool { inner: Atomic_i32_new(v) }
}

impl AtomicBool {
    fn load(self) -> bool {
        return self.inner.load() != 0
    }

    fn store(self, v: bool) -> AtomicBool {
        let n = if v { 1 } else { 0 }
        return AtomicBool { inner: self.inner.store(n) }
    }

    fn compare_and_swap(self, expected: bool, desired: bool) -> bool {
        let e = if expected { 1 } else { 0 }
        let d = if desired { 1 } else { 0 }
        return self.inner.compare_and_swap(e, d) == e
    }
}

impl Drop for AtomicBool {
    fn drop(self) -> void {
        self.inner.drop()
    }
}
// [contrib-dev:atomic_sub_i32:sync_atomic]
extern fn atomic_sub_i32(p: ptr, delta: i32) -> i32
// [/contrib-dev:atomic_sub_i32:sync_atomic]
// [contrib-dev:atomic_xor_i32:sync_atomic]
extern fn atomic_xor_i32(p: ptr, mask: i32) -> i32
// [/contrib-dev:atomic_xor_i32:sync_atomic]
// [contrib-dev:atomic_and_i32:sync_atomic]
extern fn atomic_and_i32(p: ptr, mask: i32) -> i32
// [/contrib-dev:atomic_and_i32:sync_atomic]
// [contrib-dev:atomic_or_i32:sync_atomic]
extern fn atomic_or_i32(p: ptr, mask: i32) -> i32
// [/contrib-dev:atomic_or_i32:sync_atomic]
