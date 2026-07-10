import "builtins_string.ny"

extern fn vec_str_new() -> ptr
extern fn vec_str_push(v: ptr, value: string) -> void
extern fn vec_str_get(v: ptr, index: i32) -> string
extern fn vec_str_len(v: ptr) -> i32
extern fn vec_str_free(v: ptr) -> void
extern fn vec_str_from_argv(start_index: i32) -> ptr
extern fn strlen(s: &string) -> i32
extern fn strcat(a: &string, b: &string) -> string
extern fn strcmp(a: &string, b: &string) -> i32

fn Vec_str_new() -> ptr {
    return vec_str_new()
}

fn Vec_str_push(v: ptr, value: string) -> void {
    vec_str_push(v, value)
}

fn Vec_str_get(v: ptr, index: i32) -> string {
    return vec_str_get(v, index)
}

fn Vec_str_len(v: ptr) -> i32 {
    return vec_str_len(v)
}

fn Vec_str_free(v: ptr) -> void {
    vec_str_free(v)
}

fn Vec_str_split(text: string, sep: string) -> ptr {
    return String_split(text, sep)
}

fn Vec_str_split_lines(text: string) -> ptr {
    let n = strlen(text)
    if n == 0 {
        let v = Vec_str_new()
        Vec_str_push(v, "")
        return v
    }
    return String_split(text, "\n")
}

fn Vec_str_join(lines: ptr, sep: string) -> string {
    let n = Vec_str_len(lines)
    if n == 0 {
        return ""
    }
    let mut out = Vec_str_get(lines, 0)
    let mut i = 1
    while i < n {
        out = strcat(strcat(out, sep), Vec_str_get(lines, i))
        i = i + 1
    }
    return out
}

fn Vec_str_join_lines(lines: ptr) -> string {
    return Vec_str_join(lines, "\n")
}

struct StrVec {
    handle: ptr
}

fn StrVec_new() -> StrVec {
    return StrVec { handle: vec_str_new() }
}

fn StrVec_from_lines(text: string) -> StrVec {
    return StrVec { handle: Vec_str_split_lines(text) }
}

fn StrVec_from_argv(start_index: i32) -> StrVec {
    return StrVec { handle: vec_str_from_argv(start_index) }
}

fn argv() -> StrVec {
    return StrVec_from_argv(1)
}

fn strs() -> StrVec {
    return StrVec_new()
}

fn lines(text: string) -> StrVec {
    return StrVec_from_lines(text)
}

fn StrVec_join_lines(vec: StrVec) -> string {
    return Vec_str_join_lines(vec.handle)
}

fn StrVec_raw(vec: StrVec) -> ptr {
    return vec.handle
}

impl StrVec {
    fn push(self, value: string) -> StrVec {
        Vec_str_push(self.handle, value)
        return self
    }

    fn get(self, index: i32) -> string {
        return Vec_str_get(self.handle, index)
    }

    fn len(self) -> i32 {
        return Vec_str_len(self.handle)
    }

    fn joined(self, sep: string) -> string {
        return Vec_str_join(self.handle, sep)
    }

    fn contains(self, needle: string) -> i32 {
        let n = Vec_str_len(self.handle)
        let mut i = 0
        while i < n {
            if strcmp(Vec_str_get(self.handle, i), needle) == 0 {
                return 1
            }
            i = i + 1
        }
        return 0
    }

    fn includes(self, needle: string) -> i32 {
        return self.contains(needle)
    }

    fn first(self, fallback: string) -> string {
        if Vec_str_len(self.handle) == 0 {
            return fallback
        }
        return Vec_str_get(self.handle, 0)
    }

    fn last(self, fallback: string) -> string {
        let n = Vec_str_len(self.handle)
        if n == 0 {
            return fallback
        }
        return Vec_str_get(self.handle, n - 1)
    }

    fn find(self, pred: fn(string) -> i32, fallback: string) -> string {
        let n = Vec_str_len(self.handle)
        let mut i = 0
        while i < n {
            let x = Vec_str_get(self.handle, i)
            if pred(x) != 0 {
                return x
            }
            i = i + 1
        }
        return fallback
    }

    fn filter(self, pred: fn(string) -> i32) -> StrVec {
        let out = vec_str_new()
        let n = Vec_str_len(self.handle)
        let mut i = 0
        while i < n {
            let x = Vec_str_get(self.handle, i)
            if pred(x) != 0 {
                vec_str_push(out, x)
            }
            i = i + 1
        }
        return StrVec { handle: out }
    }

    fn map(self, f: fn(string) -> string) -> StrVec {
        let out = vec_str_new()
        let n = Vec_str_len(self.handle)
        let mut i = 0
        while i < n {
            vec_str_push(out, f(Vec_str_get(self.handle, i)))
            i = i + 1
        }
        return StrVec { handle: out }
    }

    fn find_eq(self, needle: string, fallback: string) -> string {
        if self.contains(needle) == 1 {
            return needle
        }
        return fallback
    }


    fn insert(self, index: i32, value: string) -> StrVec {
        vec_str_insert(self.handle, index, value)
        return self
    }

    fn remove_at(self, index: i32) -> string {
        return vec_str_remove_at(self.handle, index)
    }

    fn extend(self, other: StrVec) -> StrVec {
        vec_str_extend(self.handle, other.handle)
        return self
    }

    fn append(self, value: string) -> StrVec {
        return self.push(value)
    }

    fn swap(self, i: i32, j: i32) -> StrVec {
        vec_str_swap(self.handle, i, j)
        return self
    }


    fn set(self, index: i32, value: string) -> StrVec {
        vec_str_set(self.handle, index, value)
        return self
    }

    fn index_of(self, needle: string) -> i32 {
        let n = Vec_str_len(self.handle)
        let mut i = 0
        while i < n {
            if strcmp(Vec_str_get(self.handle, i), needle) == 0 {
                return i
            }
            i = i + 1
        }
        return -1
    }

    fn pop(self) -> string {
        return vec_str_pop(self.handle)
    }

    fn clear(self) -> StrVec {
        vec_str_clear(self.handle)
        return self
    }

    fn reverse(self) -> StrVec {
        vec_str_reverse(self.handle)
        return self
    }

    fn is_empty(self) -> i32 {
        if Vec_str_len(self.handle) == 0 {
            return 1
        }
        return 0
    }

    fn reduce(self, init: string, reducer: fn(string, string) -> string) -> string {
        let mut acc = init
        let n = Vec_str_len(self.handle)
        let mut i = 0
        while i < n {
            acc = reducer(acc, Vec_str_get(self.handle, i))
            i = i + 1
        }
        return acc
    }
}

impl Drop for StrVec {
    fn drop(self) -> void {
        Vec_str_free(self.handle)
    }
}

// Generic `Vec<string>` syntax aliases (monomorph maps Vec<string> → StrVec).
fn Vec_string_new() -> StrVec {
    return StrVec_new()
}

fn Vec_string_push(v: StrVec, value: string) -> StrVec {
    return v.push(value)
}

fn Vec_string_get(v: StrVec, index: i32) -> string {
    return v.get(index)
}

fn Vec_string_len(v: StrVec) -> i32 {
    return v.len()
}

fn Vec_string_free(v: StrVec) -> void {
    Vec_str_free(v.handle)
}
// [contrib-dev:vec_str_clear:vec_str]
extern fn vec_str_clear(handle: ptr)
// [/contrib-dev:vec_str_clear:vec_str]
// [contrib-dev:vec_str_pop:vec_str]
extern fn vec_str_pop(handle: ptr) -> string
// [/contrib-dev:vec_str_pop:vec_str]
// [contrib-dev:vec_str_reverse:vec_str]
extern fn vec_str_reverse(handle: ptr)
// [/contrib-dev:vec_str_reverse:vec_str]
// [contrib-dev:vec_str_extend:vec_str]
extern fn vec_str_extend(dst: ptr, src: ptr)
// [/contrib-dev:vec_str_extend:vec_str]
// [contrib-dev:vec_str_insert:vec_str]
extern fn vec_str_insert(handle: ptr, index: i32, value: &string)
// [/contrib-dev:vec_str_insert:vec_str]
// [contrib-dev:vec_str_remove_at:vec_str]
extern fn vec_str_remove_at(handle: ptr, index: i32) -> string
// [/contrib-dev:vec_str_remove_at:vec_str]
// [contrib-dev:vec_str_set:vec_str]
extern fn vec_str_set(handle: ptr, index: i32, value: &string)
// [/contrib-dev:vec_str_set:vec_str]
// [contrib-dev:vec_str_swap:vec_str]
extern fn vec_str_swap(handle: ptr, i: i32, j: i32)
// [/contrib-dev:vec_str_swap:vec_str]
