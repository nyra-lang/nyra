extern fn vec_i32_new() -> ptr
extern fn vec_i32_push(v: ptr, x: i32) -> void
extern fn vec_i32_get(v: ptr, i: i32) -> i32
extern fn vec_i32_set(v: ptr, i: i32, value: i32) -> void
extern fn vec_i32_len(v: ptr) -> i32
extern fn vec_i32_pop(v: ptr) -> i32
extern fn vec_i32_free(v: ptr) -> void

fn Vec_i32_new() -> ptr {
    return vec_i32_new()
}

fn Vec_i32_push(v: ptr, x: i32) -> void {
    vec_i32_push(v, x)
}

fn Vec_i32_get(v: ptr, i: i32) -> i32 {
    return vec_i32_get(v, i)
}

fn Vec_i32_set(v: ptr, i: i32, value: i32) -> void {
    vec_i32_set(v, i, value)
}

fn Vec_i32_len(v: ptr) -> i32 {
    return vec_i32_len(v)
}

fn Vec_i32_pop(v: ptr) -> i32 {
    return vec_i32_pop(v)
}

fn Vec_i32_free(v: ptr) -> void {
    vec_i32_free(v)
}

fn Vec_i32_from_range(start: i32, end: i32) -> ptr {
    let v = vec_i32_new()
    let mut i = start
    while i < end {
        vec_i32_push(v, i)
        i = i + 1
    }
    return v
}

// Free helpers for stdlib internals (ptr-backed Vec_i32 handles).
fn vec_len(v: ptr) -> i32 {
    return Vec_i32_len(v)
}

fn vec_get(v: ptr, i: i32) -> i32 {
    return Vec_i32_get(v, i)
}

fn vec_push(v: ptr, x: i32) -> ptr {
    Vec_i32_push(v, x)
    return v
}

// Method-bearing i32 vector (parity with StrVec) — prefer this in app code.
struct VecI32 {
    handle: ptr
}

fn vec() -> VecI32 {
    return VecI32 { handle: vec_i32_new() }
}

fn vec_range(start: i32, end: i32) -> VecI32 {
    return VecI32 { handle: Vec_i32_from_range(start, end) }
}

impl VecI32 {
    fn push(self, x: i32) -> VecI32 {
        vec_i32_push(self.handle, x)
        return self
    }

    fn get(self, i: i32) -> i32 {
        return vec_i32_get(self.handle, i)
    }

    fn set(self, i: i32, value: i32) -> VecI32 {
        vec_i32_set(self.handle, i, value)
        return self
    }

    fn len(self) -> i32 {
        return vec_i32_len(self.handle)
    }

    fn pop(self) -> i32 {
        return vec_i32_pop(self.handle)
    }

    fn contains(self, x: i32) -> i32 {
        let n = vec_i32_len(self.handle)
        let mut i = 0
        while i < n {
            if vec_i32_get(self.handle, i) == x {
                return 1
            }
            i = i + 1
        }
        return 0
    }

    fn includes(self, x: i32) -> i32 {
        return self.contains(x)
    }

    fn first(self, fallback: i32) -> i32 {
        if vec_i32_len(self.handle) == 0 {
            return fallback
        }
        return vec_i32_get(self.handle, 0)
    }

    fn last(self, fallback: i32) -> i32 {
        let n = vec_i32_len(self.handle)
        if n == 0 {
            return fallback
        }
        return vec_i32_get(self.handle, n - 1)
    }

    fn find(self, pred: fn(i32) -> i32, fallback: i32) -> i32 {
        let n = vec_i32_len(self.handle)
        let mut i = 0
        while i < n {
            let x = vec_i32_get(self.handle, i)
            if pred(x) != 0 {
                return x
            }
            i = i + 1
        }
        return fallback
    }

    fn filter(self, pred: fn(i32) -> i32) -> VecI32 {
        let out = vec_i32_new()
        let n = vec_i32_len(self.handle)
        let mut i = 0
        while i < n {
            let x = vec_i32_get(self.handle, i)
            if pred(x) != 0 {
                vec_i32_push(out, x)
            }
            i = i + 1
        }
        return VecI32 { handle: out }
    }

    fn map(self, f: fn(i32) -> i32) -> VecI32 {
        let out = vec_i32_new()
        let n = vec_i32_len(self.handle)
        let mut i = 0
        while i < n {
            vec_i32_push(out, f(vec_i32_get(self.handle, i)))
            i = i + 1
        }
        return VecI32 { handle: out }
    }

    fn reduce(self, init: i32, reducer: fn(i32, i32) -> i32) -> i32 {
        let mut acc = init
        let n = vec_i32_len(self.handle)
        let mut i = 0
        while i < n {
            acc = reducer(acc, vec_i32_get(self.handle, i))
            i = i + 1
        }
        return acc
    }

    // Value equality find (no callback needed).
    fn find_eq(self, x: i32, fallback: i32) -> i32 {
        if self.contains(x) == 1 {
            return x
        }
        return fallback
    }

    fn index_of(self, x: i32) -> i32 {
        let n = vec_i32_len(self.handle)
        let mut i = 0
        while i < n {
            if vec_i32_get(self.handle, i) == x {
                return i
            }
            i = i + 1
        }
        return -1
    }

    fn insert(self, index: i32, x: i32) -> VecI32 {
        vec_i32_insert(self.handle, index, x)
        return self
    }

    fn remove(self, index: i32) -> VecI32 {
        let _ = vec_i32_remove_at(self.handle, index)
        return self
    }

    fn clear(self) -> VecI32 {
        vec_i32_clear(self.handle)
        return self
    }

    fn sort(self) -> VecI32 {
        vec_i32_sort(self.handle)
        return self
    }

    fn reverse(self) -> VecI32 {
        vec_i32_reverse(self.handle)
        return self
    }

    fn sum(self) -> i32 {
        let mut acc = 0
        let n = vec_i32_len(self.handle)
        let mut i = 0
        while i < n {
            acc = acc + vec_i32_get(self.handle, i)
            i = i + 1
        }
        return acc
    }

    fn any(self, pred: fn(i32) -> i32) -> i32 {
        let n = vec_i32_len(self.handle)
        let mut i = 0
        while i < n {
            if pred(vec_i32_get(self.handle, i)) != 0 {
                return 1
            }
            i = i + 1
        }
        return 0
    }

    fn all(self, pred: fn(i32) -> i32) -> i32 {
        let n = vec_i32_len(self.handle)
        let mut i = 0
        while i < n {
            if pred(vec_i32_get(self.handle, i)) == 0 {
                return 0
            }
            i = i + 1
        }
        return 1
    }

    fn min_elem(self, fallback: i32) -> i32 {
        let n = vec_i32_len(self.handle)
        if n == 0 {
            return fallback
        }
        let mut best = vec_i32_get(self.handle, 0)
        let mut i = 1
        while i < n {
            let x = vec_i32_get(self.handle, i)
            if x < best {
                best = x
            }
            i = i + 1
        }
        return best
    }

    fn max_elem(self, fallback: i32) -> i32 {
        let n = vec_i32_len(self.handle)
        if n == 0 {
            return fallback
        }
        let mut best = vec_i32_get(self.handle, 0)
        let mut i = 1
        while i < n {
            let x = vec_i32_get(self.handle, i)
            if x > best {
                best = x
            }
            i = i + 1
        }
        return best
    }

    fn take(self, n: i32) -> VecI32 {
        let out = vec_i32_new()
        let len = vec_i32_len(self.handle)
        let mut i = 0
        while i < n && i < len {
            vec_i32_push(out, vec_i32_get(self.handle, i))
            i = i + 1
        }
        return VecI32 { handle: out }
    }

    fn skip(self, n: i32) -> VecI32 {
        let out = vec_i32_new()
        let len = vec_i32_len(self.handle)
        let mut i = n
        while i < len {
            vec_i32_push(out, vec_i32_get(self.handle, i))
            i = i + 1
        }
        return VecI32 { handle: out }
    }

    fn dedup(self) -> VecI32 {
        let out = vec_i32_new()
        let n = vec_i32_len(self.handle)
        let mut i = 0
        while i < n {
            let x = vec_i32_get(self.handle, i)
            let mut seen = 0
            let mut j = 0
            while j < vec_i32_len(out) {
                if vec_i32_get(out, j) == x {
                    seen = 1
                }
                j = j + 1
            }
            if seen == 0 {
                vec_i32_push(out, x)
            }
            i = i + 1
        }
        return VecI32 { handle: out }
    }

    fn binary_search(self, x: i32) -> i32 {
        let mut lo = 0
        let mut hi = vec_i32_len(self.handle) - 1
        while lo <= hi {
            let mid = (lo + hi) / 2
            let v = vec_i32_get(self.handle, mid)
            if v == x {
                return mid
            }
            if v < x {
                lo = mid + 1
            } else {
                hi = mid - 1
            }
        }
        return -1
    }
}

impl Drop for VecI32 {
    fn drop(self) -> void {
        vec_i32_free(self.handle)
    }
}

// [contrib-dev:vec_i32_clear:vec]
extern fn vec_i32_clear(handle: ptr)
// [/contrib-dev:vec_i32_clear:vec]
// [contrib-dev:vec_i32_insert:vec]
extern fn vec_i32_insert(handle: ptr, index: i32, value: i32)
// [/contrib-dev:vec_i32_insert:vec]
// [contrib-dev:vec_i32_remove_at:vec]
extern fn vec_i32_remove_at(handle: ptr, index: i32) -> i32
// [/contrib-dev:vec_i32_remove_at:vec]
// [contrib-dev:vec_i32_reverse:vec]
extern fn vec_i32_reverse(handle: ptr)
// [/contrib-dev:vec_i32_reverse:vec]
// [contrib-dev:vec_i32_sort:vec]
extern fn vec_i32_sort(handle: ptr)
// [/contrib-dev:vec_i32_sort:vec]
