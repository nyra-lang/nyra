import "builtins_string.ny"

extern fn vec_str_new() -> ptr
extern fn vec_str_push(v: ptr, value: string) -> void
extern fn vec_str_get(v: ptr, index: i32) -> string
extern fn vec_str_len(v: ptr) -> i32
extern fn vec_str_free(v: ptr) -> void
extern fn vec_str_from_argv(start_index: i32) -> ptr
extern fn strlen(s: &string) -> i32
extern fn strcat(a: &string, b: &string) -> string

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
