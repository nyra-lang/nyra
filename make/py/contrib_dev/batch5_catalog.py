"""Batch5 gap-fill catalog — definitions for `gen_batch5.py`.

Emits JSON for `make batch-add-builtin BATCH=batch5`.
"""
from __future__ import annotations

STRING_BUILTINS: list[dict] = [
    {
        "receiver": "string",
        "method": "reverse",
        "args": [],
        "returns": "string",
        "c_name": "str_reverse",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    extern char *str_dup(const char *s);
    if (!s) return str_dup("");
    size_t n = strlen(s);
    char *out = (char *)malloc(n + 1);
    if (!out) return NULL;
    for (size_t i = 0; i < n; i++) out[i] = s[n - 1 - i];
    out[n] = '\\0';
    return out;""",
    },
    {
        "receiver": "string",
        "method": "is_digit",
        "args": [],
        "returns": "i32",
        "c_name": "str_is_digit",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s || s[0] == '\\0') return 0;
    for (int i = 0; s[i]; i++) {
        if (s[i] < '0' || s[i] > '9') return 0;
    }
    return 1;""",
    },
    {
        "receiver": "string",
        "method": "is_alpha",
        "args": [],
        "returns": "i32",
        "c_name": "str_is_alpha",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s || s[0] == '\\0') return 0;
    for (int i = 0; s[i]; i++) {
        unsigned char c = (unsigned char)s[i];
        if (!((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z'))) return 0;
    }
    return 1;""",
    },
    {
        "receiver": "string",
        "method": "is_alnum",
        "args": [],
        "returns": "i32",
        "c_name": "str_is_alnum",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s || s[0] == '\\0') return 0;
    for (int i = 0; s[i]; i++) {
        unsigned char c = (unsigned char)s[i];
        if (!((c >= '0' && c <= '9') || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')))
            return 0;
    }
    return 1;""",
    },
    {
        "receiver": "string",
        "method": "common_prefix_len",
        "args": ["other:string"],
        "returns": "i32",
        "c_name": "str_common_prefix_len",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s || !other) return 0;
    int i = 0;
    while (s[i] && other[i] && s[i] == other[i]) i++;
    return i;""",
    },
    {
        "receiver": "string",
        "method": "pad_center",
        "args": ["width:i32", "pad:string"],
        "returns": "string",
        "c_name": "str_pad_center",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    extern char *str_dup(const char *s);
    extern char *str_pad_start(const char *s, int width, const char *pad);
    if (!s) return str_dup("");
    if (width <= 0) return str_dup(s);
    int slen = (int)strlen(s);
    if (slen >= width) return str_dup(s);
    int total_pad = width - slen;
    int left = total_pad / 2;
    int right = total_pad - left;
    char *tmp = (char *)malloc((size_t)width + 1);
    if (!tmp) return NULL;
    const char *pch = (pad && pad[0]) ? pad : " ";
    int pi = 0;
    int oi = 0;
    for (int i = 0; i < left; i++) tmp[oi++] = pch[pi++ % (int)strlen(pch)];
    for (int i = 0; i < slen; i++) tmp[oi++] = s[i];
    pi = 0;
    for (int i = 0; i < right; i++) tmp[oi++] = pch[pi++ % (int)strlen(pch)];
    tmp[oi] = '\\0';
    return tmp;""",
    },
]

MATH_EXTERN: list[dict] = [
    {
        "ny_module": "math.ny",
        "fn_name": "saturating_add_i32",
        "args": ["a:i32", "b:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "ny_alias": "saturating_add",
        "c_body": """\
    long long r = (long long)a + (long long)b;
    if (r > 2147483647LL) return 2147483647;
    if (r < -2147483648LL) return (int)-2147483648LL;
    return (int)r;""",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "saturating_sub_i32",
        "args": ["a:i32", "b:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "ny_alias": "saturating_sub",
        "c_body": """\
    long long r = (long long)a - (long long)b;
    if (r > 2147483647LL) return 2147483647;
    if (r < -2147483648LL) return (int)-2147483648LL;
    return (int)r;""",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "wrapping_add_i32",
        "args": ["a:i32", "b:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "ny_alias": "wrapping_add",
        "c_body": "    return (int)((unsigned int)a + (unsigned int)b);",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "leading_zeros_i32",
        "args": ["n:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "ny_alias": "leading_zeros",
        "c_body": """\
    if (n == 0) return 32;
    unsigned u = (unsigned)n;
    int c = 0;
    if ((u & 0xFFFF0000u) == 0) { c += 16; u <<= 16; }
    if ((u & 0xFF000000u) == 0) { c += 8; u <<= 8; }
    if ((u & 0xF0000000u) == 0) { c += 4; u <<= 4; }
    if ((u & 0xC0000000u) == 0) { c += 2; u <<= 2; }
    if ((u & 0x80000000u) == 0) { c += 1; }
    return c;""",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "count_ones_i32",
        "args": ["n:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "ny_alias": "count_ones",
        "c_body": """\
    unsigned u = (unsigned)n;
    int c = 0;
    while (u) { c += (int)(u & 1u); u >>= 1; }
    return c;""",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "is_infinite_f64",
        "args": ["x:f64"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "ny_alias": "is_infinite",
        "c_body": "    return (x == x && (x > 1e308 || x < -1e308)) ? 1 : 0;",
    },
]

FORMAT_EXTERN: list[dict] = [
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "format_quote",
        "args": ["s:string"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "ny_alias": "quote",
        "c_body": """\
    extern char *str_dup(const char *s);
    if (!s) return str_dup("\\"\\"");
    size_t n = strlen(s);
    char *out = (char *)malloc(n * 2 + 3);
    if (!out) return NULL;
    int oi = 0;
    out[oi++] = '"';
    for (size_t i = 0; i < n; i++) {
        char c = s[i];
        if (c == '"' || c == '\\\\') out[oi++] = '\\\\';
        out[oi++] = c;
    }
    out[oi++] = '"';
    out[oi] = '\\0';
    return out;""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "format_i64_bin",
        "args": ["n:i64"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "ny_alias": "format_bin_i64",
        "c_body": """\
    extern char *str_dup(const char *s);
    char tmp[65];
    int pos = 64;
    unsigned long long u = (unsigned long long)n;
    tmp[pos] = '\\0';
    if (u == 0) return str_dup("0");
    while (u > 0 && pos > 0) {
        tmp[--pos] = (char)('0' + (u & 1));
        u >>= 1;
    }
    return str_dup(tmp + pos);""",
    },
]

VEC_EXTERN: list[dict] = [
    {
        "ny_module": "vec.ny",
        "fn_name": "vec_i32_capacity",
        "args": ["handle:ptr"],
        "returns": "i32",
        "rt_module": "rt_vec.c",
        "c_body": """\
    NyraVec *v = (NyraVec *)handle;
    return v ? v->cap : 0;""",
    },
    {
        "ny_module": "vec.ny",
        "fn_name": "vec_i32_reserve",
        "args": ["handle:ptr", "min_cap:i32"],
        "returns": "void",
        "rt_module": "rt_vec.c",
        "c_body": """\
    NyraVec *v = (NyraVec *)handle;
    if (!v || min_cap <= v->cap) return;
    int nc = v->cap;
    while (nc < min_cap) nc = nc < 1 ? 8 : nc * 2;
    void *nd = realloc(v->data, (size_t)nc * (size_t)v->elem_size);
    if (!nd) return;
    v->data = nd;
    v->cap = nc;""",
    },
    {
        "ny_module": "vec.ny",
        "fn_name": "vec_i32_fill",
        "args": ["handle:ptr", "value:i32"],
        "returns": "void",
        "rt_module": "rt_vec.c",
        "c_body": """\
    NyraVec *v = (NyraVec *)handle;
    if (!v) return;
    int *data = (int *)v->data;
    for (int i = 0; i < v->len; i++) data[i] = value;""",
    },
    {
        "ny_module": "vec.ny",
        "fn_name": "vec_i32_swap_remove",
        "args": ["handle:ptr", "index:i32"],
        "returns": "i32",
        "rt_module": "rt_vec.c",
        "c_body": """\
    NyraVec *v = (NyraVec *)handle;
    if (!v || index < 0 || index >= v->len) return 0;
    int *data = (int *)v->data;
    int removed = data[index];
    data[index] = data[v->len - 1];
    v->len--;
    return removed;""",
    },
]

SYNC_EXTERN: list[dict] = [
    {
        "ny_module": "sync/atomic.ny",
        "fn_name": "atomic_xor_i32",
        "args": ["p:ptr", "mask:i32"],
        "returns": "i32",
        "rt_module": "rt_sync.c",
        "c_body": """\
    int *cell = (int *)p;
    if (!cell) return 0;
    return __atomic_xor_fetch(cell, mask, __ATOMIC_SEQ_CST);""",
    },
]

VECI32_EXTRA_PURE = """\
impl VecI32 {
    fn capacity(self) -> i32 {
        return vec_i32_capacity(self.handle)
    }

    fn reserve(self, min_cap: i32) -> VecI32 {
        vec_i32_reserve(self.handle, min_cap)
        return self
    }

    fn fill(self, value: i32) -> VecI32 {
        vec_i32_fill(self.handle, value)
        return self
    }

    fn swap_remove(self, index: i32) -> i32 {
        return vec_i32_swap_remove(self.handle, index)
    }

    fn is_empty(self) -> i32 {
        if self.len() == 0 {
            return 1
        }
        return 0
    }
}"""

HASHMAP_I32_GET_OR_PURE = """\
impl HashMap_i32_i32 {
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
}"""

PURE_MODULES: list[dict] = [
    {
        "recipe": "stdlib-pure",
        "ny_module": "vec.ny",
        "fn_name": "vec_i32_extra_methods",
        "pure_source": VECI32_EXTRA_PURE,
    },
    {
        "recipe": "stdlib-pure",
        "ny_module": "map.ny",
        "fn_name": "hashmap_i32_get_or",
        "pure_source": HASHMAP_I32_GET_OR_PURE,
    },
]

STRING_TEST_CASES: dict[str, list[str]] = {
    "reverse": [
        '    assert_str_eq("abc".reverse(), "cba")',
        '    assert_str_eq("".reverse(), "")',
    ],
    "is_digit": [
        '    assert_eq("123".is_digit(), 1)',
        '    assert_eq("12a".is_digit(), 0)',
    ],
    "is_alpha": [
        '    assert_eq("abc".is_alpha(), 1)',
        '    assert_eq("ab1".is_alpha(), 0)',
    ],
    "is_alnum": [
        '    assert_eq("abc123".is_alnum(), 1)',
        '    assert_eq("abc-1".is_alnum(), 0)',
    ],
    "common_prefix_len": [
        '    assert_eq("abcdef".common_prefix_len("abcxyz"), 3)',
        '    assert_eq("x".common_prefix_len("y"), 0)',
    ],
    "pad_center": [
        '    assert_str_eq("hi".pad_center(6, " "), "  hi  ")',
    ],
}
