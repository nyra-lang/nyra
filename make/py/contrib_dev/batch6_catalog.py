"""Batch6 gap-fill catalog — FS, slice utilities, bit math, sync atomics.

Emits JSON for `make batch-add-builtin BATCH=batch6`.
"""
from __future__ import annotations

STRING_BUILTINS: list[dict] = [
    {
        "receiver": "string",
        "method": "escape_json",
        "args": [],
        "returns": "string",
        "c_name": "str_escape_json",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    extern char *str_dup(const char *s);
    if (!s) return str_dup("");
    size_t n = strlen(s);
    char *out = (char *)malloc(n * 2 + 1);
    if (!out) return NULL;
    size_t oi = 0;
    for (size_t i = 0; i < n; i++) {
        char c = s[i];
        if (c == '"' || c == '\\\\' || c == '\\n' || c == '\\r' || c == '\\t') {
            out[oi++] = '\\\\';
            if (c == '\\n') out[oi++] = 'n';
            else if (c == '\\r') out[oi++] = 'r';
            else if (c == '\\t') out[oi++] = 't';
            else out[oi++] = c;
        } else {
            out[oi++] = c;
        }
    }
    out[oi] = '\\0';
    return out;""",
    },
    {
        "receiver": "string",
        "method": "truncate",
        "args": ["max_len:i32"],
        "returns": "string",
        "c_name": "str_truncate",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    extern char *str_dup(const char *s);
    if (!s) return str_dup("");
    if (max_len <= 0) return str_dup("");
    size_t n = strlen(s);
    size_t take = (size_t)max_len;
    if (take > n) take = n;
    char *out = (char *)malloc(take + 1);
    if (!out) return NULL;
    memcpy(out, s, take);
    out[take] = '\\0';
    return out;""",
    },
    {
        "receiver": "string",
        "method": "split_after",
        "args": ["sep:string"],
        "returns": "string",
        "c_name": "str_split_after",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    extern char *str_dup(const char *s);
    if (!s) return str_dup("");
    if (!sep || sep[0] == '\\0') return str_dup(s);
    const char *p = strstr(s, sep);
    if (!p) return str_dup(s);
    p += strlen(sep);
    return str_dup(p);""",
    },
]

MATH_EXTERN: list[dict] = [
    {
        "ny_module": "math.ny",
        "fn_name": "trailing_zeros_i32",
        "args": ["n:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "ny_alias": "trailing_zeros",
        "c_body": """\
    if (n == 0) return 32;
    unsigned u = (unsigned)n;
    int c = 0;
    while ((u & 1u) == 0u) { c++; u >>= 1; }
    return c;""",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "rotate_left_i32",
        "args": ["n:i32", "shift:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "ny_alias": "rotate_left",
        "c_body": """\
    unsigned u = (unsigned)n;
    int s = shift & 31;
    return (int)((u << s) | (u >> (32 - s)));""",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "rotate_right_i32",
        "args": ["n:i32", "shift:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "ny_alias": "rotate_right",
        "c_body": """\
    unsigned u = (unsigned)n;
    int s = shift & 31;
    return (int)((u >> s) | (u << (32 - s)));""",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "rem_euclid_i32",
        "args": ["a:i32", "b:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "ny_alias": "rem_euclid",
        "c_body": """\
    if (b == 0) return 0;
    int r = a % b;
    if (r < 0) r += (b < 0 ? -b : b);
    return r;""",
    },
]

FS_EXTERN: list[dict] = [
    {
        "ny_module": "fs/file.ny",
        "fn_name": "file_mtime",
        "args": ["path:string"],
        "returns": "i64",
        "rt_module": "rt_fs.c",
        "c_body": """\
#if defined(_WIN32)
    struct _stat st;
    if (!path || _stat(path, &st) != 0) return -1;
    return (long long)st.st_mtime;
#else
    struct stat st;
    if (!path || stat(path, &st) != 0) return -1;
    return (long long)st.st_mtime;
#endif""",
    },
    {
        "ny_module": "fs/file.ny",
        "fn_name": "rename_file",
        "args": ["src:string", "dst:string"],
        "returns": "i32",
        "rt_module": "rt_fs.c",
        "c_body": """\
    if (!src || !dst) return -1;
#if defined(_WIN32)
    return MoveFileA(src, dst) ? 0 : -1;
#else
    return rename(src, dst) == 0 ? 0 : -1;
#endif""",
    },
    {
        "ny_module": "fs/file.ny",
        "fn_name": "path_is_file",
        "args": ["path:string"],
        "returns": "i32",
        "rt_module": "rt_fs.c",
        "c_body": """\
    if (!path) return 0;
#if defined(_WIN32)
    struct _stat st;
    if (_stat(path, &st) != 0) return 0;
    return (st.st_mode & _S_IFDIR) ? 0 : 1;
#else
    struct stat st;
    if (stat(path, &st) != 0) return 0;
    return S_ISDIR(st.st_mode) ? 0 : 1;
#endif""",
    },
    {
        "ny_module": "fs/file.ny",
        "fn_name": "file_is_symlink",
        "args": ["path:string"],
        "returns": "i32",
        "rt_module": "rt_fs.c",
        "c_body": """\
    if (!path) return 0;
#if defined(_WIN32)
    DWORD attr = GetFileAttributesA(path);
    if (attr == INVALID_FILE_ATTRIBUTES) return 0;
    return (attr & FILE_ATTRIBUTE_REPARSE_POINT) ? 1 : 0;
#else
    struct stat st;
    if (lstat(path, &st) != 0) return 0;
    return S_ISLNK(st.st_mode) ? 1 : 0;
#endif""",
    },
]

SYNC_EXTERN: list[dict] = [
    {
        "ny_module": "sync/atomic.ny",
        "fn_name": "atomic_and_i32",
        "args": ["p:ptr", "mask:i32"],
        "returns": "i32",
        "rt_module": "rt_sync.c",
        "c_body": """\
    int *cell = (int *)p;
    if (!cell) return 0;
    return __atomic_and_fetch(cell, mask, __ATOMIC_SEQ_CST);""",
    },
    {
        "ny_module": "sync/atomic.ny",
        "fn_name": "atomic_or_i32",
        "args": ["p:ptr", "mask:i32"],
        "returns": "i32",
        "rt_module": "rt_sync.c",
        "c_body": """\
    int *cell = (int *)p;
    if (!cell) return 0;
    return __atomic_or_fetch(cell, mask, __ATOMIC_SEQ_CST);""",
    },
]

VEC_EXTERN: list[dict] = [
    {
        "ny_module": "vec.ny",
        "fn_name": "vec_i32_truncate",
        "args": ["handle:ptr", "len:i32"],
        "returns": "void",
        "rt_module": "rt_vec.c",
        "c_body": """\
    NyraVec *v = (NyraVec *)handle;
    if (!v || len < 0) return;
    if (len < v->len) v->len = len;""",
    },
]

VECI32_SLICE_PURE = """\
impl VecI32 {
    fn slice(self, start: i32, end: i32) -> VecI32 {
        let out = vec_i32_new()
        let n = vec_i32_len(self.handle)
        let mut i = start
        while i < end && i < n {
            vec_i32_push(out, vec_i32_get(self.handle, i))
            i = i + 1
        }
        return VecI32 { handle: out }
    }

    fn window(self, start: i32, size: i32) -> VecI32 {
        return self.slice(start, start + size)
    }

    fn retain(self, pred: fn(i32) -> i32) -> VecI32 {
        let n = vec_i32_len(self.handle)
        let mut i = 0
        let mut write = 0
        while i < n {
            let x = vec_i32_get(self.handle, i)
            if pred(x) != 0 {
                vec_i32_set(self.handle, write, x)
                write = write + 1
            }
            i = i + 1
        }
        vec_i32_truncate(self.handle, write)
        return self
    }
}"""

PURE_MODULES: list[dict] = [
    {
        "recipe": "stdlib-pure",
        "ny_module": "vec.ny",
        "fn_name": "vec_i32_slice_methods",
        "pure_source": VECI32_SLICE_PURE,
    },
]

STRING_TEST_CASES: dict[str, list[str]] = {
    "escape_json": [
        '    assert_str_eq("a\\"b".escape_json(), "a\\\\\\"b")',
        '    assert_str_eq("\\n".escape_json(), "\\\\n")',
    ],
    "truncate": [
        '    assert_str_eq("hello".truncate(3), "hel")',
        '    assert_str_eq("hi".truncate(10), "hi")',
    ],
    "split_after": [
        '    assert_str_eq("a:b:c".split_after(":"), "b:c")',
        '    assert_str_eq("abc".split_after(":"), "abc")',
    ],
}
