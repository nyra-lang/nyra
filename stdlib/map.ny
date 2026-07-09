struct HashMap<K, V> {
    handle: ptr
}

struct HashMap_str_i32 {
    handle: ptr
}

extern fn map_str_i32_new() -> ptr
extern fn map_str_i32_insert(m: ptr, key: string, value: i32) -> void
extern fn map_str_i32_get(m: ptr, key: string) -> i32
extern fn map_str_i32_contains(m: ptr, key: string) -> i32
extern fn map_str_i32_keys(m: ptr) -> ptr
extern fn map_str_i32_remove(m: ptr, key: string) -> i32
extern fn map_str_i32_free(m: ptr) -> void
extern fn map_str_i32_retain(m: ptr) -> void
extern fn map_str_i32_values(m: ptr) -> ptr
extern fn map_str_i32_len(m: ptr) -> i32
extern fn map_str_i32_clear(m: ptr) -> void

fn HashMap_str_i32_new() -> HashMap_str_i32 {
    return HashMap_str_i32 { handle: map_str_i32_new() }
}

impl HashMap_str_i32 {
    fn insert(self, key: string, value: i32) -> HashMap_str_i32 {
        map_str_i32_retain(self.handle)
        map_str_i32_insert(self.handle, key, value)
        map_str_i32_retain(self.handle)
        return self
    }

    fn get(self, key: string) -> i32 {
        return map_str_i32_get(self.handle, key)
    }

    fn get_or(self, key: string, default: i32) -> i32 {
        if self.contains(key) == 1 {
            return self.get(key)
        }
        return default
    }

    fn contains(self, key: string) -> i32 {
        return map_str_i32_contains(self.handle, key)
    }

    fn keys(self) -> StrVec {
        return StrVec { handle: map_str_i32_keys(self.handle) }
    }

    fn values(self) -> VecI32 {
        return VecI32 { handle: map_str_i32_values(self.handle) }
    }

    fn len(self) -> i32 {
        return map_str_i32_len(self.handle)
    }

    fn clear(self) -> HashMap_str_i32 {
        map_str_i32_clear(self.handle)
        return self
    }

    fn remove(self, key: string) -> HashMap_str_i32 {
        map_str_i32_retain(self.handle)
        map_str_i32_remove(self.handle, key)
        map_str_i32_retain(self.handle)
        return self
    }

    fn or_insert(self, key: string, value: i32) -> i32 {
        if self.contains(key) == 1 {
            return self.get(key)
        }
        let _ = self.insert(key, value)
        return value
    }

    fn get_or_insert(self, key: string, value: i32) -> i32 {
        return self.or_insert(key, value)
    }

    fn is_empty(self) -> i32 {
        if self.len() == 0 {
            return 1
        }
        return 0
    }

    fn update(self, key: string, f: fn(i32) -> i32) -> HashMap_str_i32 {
        if self.contains(key) == 1 {
            let _ = self.insert(key, f(self.get(key)))
        }
        return self
    }
}

impl Drop for HashMap_str_i32 {
    fn drop(self) -> void {
        map_str_i32_free(self.handle)
    }
}


struct HashMap_str_str {
    handle: ptr
}

extern fn map_str_str_new() -> ptr
extern fn map_str_str_insert(m: ptr, key: string, value: string) -> void
extern fn map_str_str_get(m: ptr, key: string) -> string
extern fn map_str_str_contains(m: ptr, key: string) -> i32
extern fn map_str_str_keys(m: ptr) -> ptr
extern fn map_str_str_remove(m: ptr, key: string) -> i32
extern fn map_str_str_free(m: ptr) -> void
extern fn map_str_str_retain(m: ptr) -> void
extern fn map_str_str_values(m: ptr) -> ptr
extern fn map_str_str_len(m: ptr) -> i32
extern fn map_str_str_clear(m: ptr) -> void

fn HashMap_str_str_new() -> HashMap_str_str {
    return HashMap_str_str { handle: map_str_str_new() }
}

impl HashMap_str_str {
    fn insert(self, key: string, value: string) -> HashMap_str_str {
        map_str_str_retain(self.handle)
        map_str_str_insert(self.handle, key, value)
        map_str_str_retain(self.handle)
        return self
    }

    fn get(self, key: string) -> string {
        return map_str_str_get(self.handle, key)
    }

    fn get_or(self, key: string, default: string) -> string {
        if self.contains(key) == 1 {
            return self.get(key)
        }
        return default
    }

    fn contains(self, key: string) -> i32 {
        return map_str_str_contains(self.handle, key)
    }

    fn keys(self) -> StrVec {
        return StrVec { handle: map_str_str_keys(self.handle) }
    }

    fn values(self) -> StrVec {
        return StrVec { handle: map_str_str_values(self.handle) }
    }

    fn len(self) -> i32 {
        return map_str_str_len(self.handle)
    }

    fn clear(self) -> HashMap_str_str {
        map_str_str_clear(self.handle)
        return self
    }

    fn remove(self, key: string) -> HashMap_str_str {
        map_str_str_retain(self.handle)
        map_str_str_remove(self.handle, key)
        map_str_str_retain(self.handle)
        return self
    }

    fn or_insert(self, key: string, value: string) -> string {
        if self.contains(key) == 1 {
            return self.get(key)
        }
        let _ = self.insert(key, value)
        return value
    }

    fn get_or_insert(self, key: string, value: string) -> string {
        return self.or_insert(key, value)
    }

    fn is_empty(self) -> i32 {
        if self.len() == 0 {
            return 1
        }
        return 0
    }
}

impl Drop for HashMap_str_str {
    fn drop(self) -> void {
        map_str_str_free(self.handle)
    }
}

// [contrib-dev:map_i32_i32_clear:map]
extern fn map_i32_i32_clear(m: ptr)
// [/contrib-dev:map_i32_i32_clear:map]
// [contrib-dev:map_i32_i32_len:map]
extern fn map_i32_i32_len(m: ptr) -> i32
// [/contrib-dev:map_i32_i32_len:map]
// [contrib-dev:map_i32_i32_remove:map]
extern fn map_i32_i32_remove(m: ptr, key: i32) -> i32
// [/contrib-dev:map_i32_i32_remove:map]
// [contrib-dev:hashmap_i32_i32:map]
struct HashMap_i32_i32 {
    handle: ptr
}

extern fn map_i32_i32_new() -> ptr
extern fn map_i32_i32_insert(m: ptr, key: i32, value: i32) -> void
extern fn map_i32_i32_get(m: ptr, key: i32) -> i32
extern fn map_i32_i32_contains(m: ptr, key: i32) -> i32
extern fn map_i32_i32_remove(m: ptr, key: i32) -> i32
extern fn map_i32_i32_len(m: ptr) -> i32
extern fn map_i32_i32_clear(m: ptr) -> void
extern fn map_i32_i32_free(m: ptr) -> void
extern fn map_i32_i32_retain(m: ptr) -> void

fn HashMap_i32_i32_new() -> HashMap_i32_i32 {
    return HashMap_i32_i32 { handle: map_i32_i32_new() }
}

impl HashMap_i32_i32 {
    fn insert(self, key: i32, value: i32) -> HashMap_i32_i32 {
        map_i32_i32_retain(self.handle)
        map_i32_i32_insert(self.handle, key, value)
        map_i32_i32_retain(self.handle)
        return self
    }

    fn get(self, key: i32) -> i32 {
        return map_i32_i32_get(self.handle, key)
    }

    fn contains(self, key: i32) -> i32 {
        return map_i32_i32_contains(self.handle, key)
    }

    fn len(self) -> i32 {
        return map_i32_i32_len(self.handle)
    }

    fn clear(self) -> HashMap_i32_i32 {
        map_i32_i32_clear(self.handle)
        return self
    }

    fn remove(self, key: i32) -> HashMap_i32_i32 {
        map_i32_i32_retain(self.handle)
        map_i32_i32_remove(self.handle, key)
        map_i32_i32_retain(self.handle)
        return self
    }

    fn is_empty(self) -> i32 {
        if self.len() == 0 {
            return 1
        }
        return 0
    }

    fn get_or(self, key: i32, default: i32) -> i32 {
        if self.contains(key) == 1 {
            return self.get(key)
        }
        return default
    }

    fn get_or_insert(self, key: i32, value: i32) -> i32 {
        if self.contains(key) == 1 {
            return self.get(key)
        }
        let _ = self.insert(key, value)
        return value
    }
}

impl Drop for HashMap_i32_i32 {
    fn drop(self) -> void {
        map_i32_i32_free(self.handle)
    }
}
// [/contrib-dev:hashmap_i32_i32:map]

