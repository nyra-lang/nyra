"""Batch4 gap-fill catalog — definitions for `gen_batch4.py`.

Emits JSON for `make batch-add-builtin BATCH=batch4`.
"""
from __future__ import annotations

STRING_BUILTINS: list[dict] = [
    {
        "receiver": "string",
        "method": "before_sep",
        "args": ["sep:string"],
        "returns": "string",
        "c_name": "str_before_sep",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s) return str_dup("");
    if (!sep || sep[0] == '\\0') return str_dup(s);
    const char *p = strstr(s, sep);
    if (!p) return str_dup(s);
    size_t len = (size_t)(p - s);
    char *out = (char *)malloc(len + 1);
    if (!out) return NULL;
    memcpy(out, s, len);
    out[len] = '\\0';
    return out;""",
    },
    {
        "receiver": "string",
        "method": "is_ascii",
        "args": [],
        "returns": "i32",
        "c_name": "str_is_ascii",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s) return 1;
    for (int i = 0; s[i]; i++) {
        if ((unsigned char)s[i] > 127) return 0;
    }
    return 1;""",
    },
    {
        "receiver": "string",
        "method": "collapse_ws",
        "args": [],
        "returns": "string",
        "c_name": "str_collapse_ws",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    extern char *str_dup(const char *s);
    if (!s) return str_dup("");
    size_t cap = strlen(s) + 1;
    char *out = (char *)malloc(cap);
    if (!out) return NULL;
    size_t oi = 0;
    int in_ws = 0;
    for (size_t i = 0; s[i]; i++) {
        char c = s[i];
        if (c == ' ' || c == '\\t' || c == '\\n' || c == '\\r') {
            if (!in_ws && oi > 0) { out[oi++] = ' '; in_ws = 1; }
        } else {
            out[oi++] = c;
            in_ws = 0;
        }
    }
    while (oi > 0 && out[oi - 1] == ' ') oi--;
    out[oi] = '\\0';
    return out;""",
    },
]

MATH_EXTERN: list[dict] = [
    {
        "ny_module": "math.ny",
        "fn_name": "mod_i32",
        "args": ["a:i32", "b:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "c_body": """\
    if (b == 0) return 0;
    int r = a % b;
    if (r < 0) r += (b < 0 ? -b : b);
    return r;""",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "gcd_i32",
        "args": ["a:i32", "b:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "c_body": """\
    if (a < 0) a = -a;
    if (b < 0) b = -b;
    while (b != 0) { int t = a % b; a = b; b = t; }
    return a;""",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "lcm_i32",
        "args": ["a:i32", "b:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "c_body": """\
    if (a == 0 || b == 0) return 0;
    int x = a < 0 ? -a : a;
    int y = b < 0 ? -b : b;
    int g = x;
    int h = y;
    while (h != 0) { int t = g % h; g = h; h = t; }
    if (g == 0) return 0;
    return (x / g) * y;""",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "deg_to_rad_f64",
        "args": ["deg:f64"],
        "returns": "f64",
        "rt_module": "rt_math.c",
        "ny_alias": "deg_to_rad",
        "c_body": "    return deg * (3.141592653589793 / 180.0);",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "rad_to_deg_f64",
        "args": ["rad:f64"],
        "returns": "f64",
        "rt_module": "rt_math.c",
        "ny_alias": "rad_to_deg",
        "c_body": "    return rad * (180.0 / 3.141592653589793);",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "fract_f64",
        "args": ["x:f64"],
        "returns": "f64",
        "rt_module": "rt_math.c",
        "ny_alias": "fract",
        "c_body": "    return x - __builtin_trunc(x);",
    },
]

STRCONV_EXTERN: list[dict] = [
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "i32_to_string_radix",
        "args": ["n:i32", "base:i32"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "ny_alias": "format_radix",
        "c_body": """\
    extern char *str_dup(const char *s);
    char buf[34];
    if (base < 2) base = 10;
    if (base > 36) base = 36;
    if (base == 10) snprintf(buf, sizeof(buf), "%d", n);
    else snprintf(buf, sizeof(buf), "%x", n);
    return str_dup(buf);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "parse_i64_base",
        "args": ["s:string", "base:i32"],
        "returns": "i64",
        "rt_module": "rt_strings.c",
        "ny_alias": "parse_i64",
        "c_body": """\
    if (!s) return 0;
    return (long long)strtoll(s, NULL, base);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "u64_to_string",
        "args": ["n:i64"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "ny_alias": "format_u64",
        "c_body": """\
    extern char *str_dup(const char *s);
    char buf[32];
    snprintf(buf, sizeof(buf), "%llu", (unsigned long long)n);
    return str_dup(buf);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "str_to_f32",
        "args": ["s:string"],
        "returns": "f64",
        "rt_module": "rt_strings.c",
        "ny_alias": "parse_f32",
        "c_body": """\
    if (!s) return 0.0;
    return (double)strtof(s, NULL);""",
    },
]

FORMAT_EXTERN: list[dict] = [
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "format_i64_hex",
        "args": ["n:i64"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "ny_alias": "format_hex_i64",
        "c_body": """\
    extern char *str_dup(const char *s);
    char buf[24];
    snprintf(buf, sizeof(buf), "%llx", (long long)n);
    return str_dup(buf);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "format_u64_pad",
        "args": ["n:i64", "width:i32"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "c_body": """\
    extern char *str_dup(const char *s);
    char buf[48];
    if (width < 0) width = 0;
    if (width > 24) width = 24;
    snprintf(buf, sizeof(buf), "%0*llu", width, (unsigned long long)n);
    return str_dup(buf);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "format_i32_oct",
        "args": ["n:i32"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "ny_alias": "format_oct",
        "c_body": """\
    extern char *str_dup(const char *s);
    char buf[16];
    snprintf(buf, sizeof(buf), "%o", n);
    return str_dup(buf);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "format_i32_bin",
        "args": ["n:i32"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "ny_alias": "format_bin",
        "c_body": """\
    extern char *str_dup(const char *s);
    char tmp[33];
    int pos = 32;
    unsigned u = (unsigned)n;
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
        "fn_name": "vec_i32_swap",
        "args": ["handle:ptr", "i:i32", "j:i32"],
        "returns": "void",
        "rt_module": "rt_vec.c",
        "c_body": """\
    NyraVec *v = (NyraVec *)handle;
    if (!v || i < 0 || j < 0 || i >= v->len || j >= v->len) return;
    int *data = (int *)v->data;
    int tmp = data[i];
    data[i] = data[j];
    data[j] = tmp;""",
    },
    {
        "ny_module": "vec.ny",
        "fn_name": "vec_i32_extend",
        "args": ["dst:ptr", "src:ptr"],
        "returns": "void",
        "rt_module": "rt_vec.c",
        "c_body": """\
    extern void vec_i32_push(void *handle, int value);
    NyraVec *d = (NyraVec *)dst;
    NyraVec *s = (NyraVec *)src;
    if (!d || !s) return;
    for (int i = 0; i < s->len; i++) {
        vec_i32_push(d, ((int *)s->data)[i]);
    }""",
    },
]

MAP_EXTERN: list[dict] = [
    {
        "ny_module": "map.ny",
        "fn_name": "map_i32_i32_remove",
        "args": ["m:ptr", "key:i32"],
        "returns": "i32",
        "rt_module": "rt_map.c",
        "c_body": """\
    NyraMapI32I32 *map = (NyraMapI32I32 *)map_handle_inner(m);
    if (!map) return 0;
    unsigned h = hash_i32(key) % (unsigned)map->cap;
    for (int i = 0; i < map->cap; i++) {
        unsigned idx = (h + (unsigned)i) % (unsigned)map->cap;
        if (!map->entries[idx].used) return 0;
        if (map->entries[idx].key == key) {
            map->entries[idx].used = 0;
            map->len = map->len - 1;
            return 1;
        }
    }
    return 0;""",
    },
    {
        "ny_module": "map.ny",
        "fn_name": "map_i32_i32_len",
        "args": ["m:ptr"],
        "returns": "i32",
        "rt_module": "rt_map.c",
        "c_body": """\
    NyraMapI32I32 *map = (NyraMapI32I32 *)map_handle_inner(m);
    return map ? map->len : 0;""",
    },
    {
        "ny_module": "map.ny",
        "fn_name": "map_i32_i32_clear",
        "args": ["m:ptr"],
        "returns": "void",
        "rt_module": "rt_map.c",
        "c_body": """\
    NyraMapI32I32 *map = (NyraMapI32I32 *)map_handle_inner(m);
    if (!map) return;
    for (int i = 0; i < map->cap; i++) map->entries[i].used = 0;
    map->len = 0;""",
    },
]

ENCODING_EXTERN: list[dict] = [
    {
        "ny_module": "encoding/mod.ny",
        "fn_name": "hex_encode",
        "args": ["data:string"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "c_body": """\
    extern char *str_dup(const char *s);
    if (!data) return str_dup("");
    size_t n = strlen(data);
    char *out = (char *)malloc(n * 2 + 1);
    if (!out) return NULL;
    static const char *hex = "0123456789abcdef";
    for (size_t i = 0; i < n; i++) {
        unsigned char b = (unsigned char)data[i];
        out[i * 2] = hex[b >> 4];
        out[i * 2 + 1] = hex[b & 15];
    }
    out[n * 2] = '\\0';
    return out;""",
    },
    {
        "ny_module": "encoding/mod.ny",
        "fn_name": "hex_encode_upper",
        "args": ["data:string"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "ny_alias": "hex_encode_upper",
        "c_body": """\
    extern char *str_dup(const char *s);
    if (!data) return str_dup("");
    size_t n = strlen(data);
    char *out = (char *)malloc(n * 2 + 1);
    if (!out) return NULL;
    static const char *hex = "0123456789ABCDEF";
    for (size_t i = 0; i < n; i++) {
        unsigned char b = (unsigned char)data[i];
        out[i * 2] = hex[b >> 4];
        out[i * 2 + 1] = hex[b & 15];
    }
    out[n * 2] = '\\0';
    return out;""",
    },
]

SYNC_EXTERN: list[dict] = [
    {
        "ny_module": "sync/atomic.ny",
        "fn_name": "atomic_sub_i32",
        "args": ["p:ptr", "delta:i32"],
        "returns": "i32",
        "rt_module": "rt_sync.c",
        "c_body": """\
    int *cell = (int *)p;
    if (!cell) return 0;
    return __atomic_sub_fetch(cell, delta, __ATOMIC_SEQ_CST);""",
    },
]

STRVEC_SET_PURE = """\
impl StrVec {
    fn set(self, index: i32, value: string) -> StrVec {
        vec_str_set(self.handle, index, value)
        return self
    }
}"""

VECI32_SWAP_EXTEND_PURE = """\
impl VecI32 {
    fn swap(self, i: i32, j: i32) -> VecI32 {
        vec_i32_swap(self.handle, i, j)
        return self
    }

    fn extend(self, other: VecI32) -> VecI32 {
        vec_i32_extend(self.handle, other.handle)
        return self
    }

    fn append(self, x: i32) -> VecI32 {
        return self.push(x)
    }
}"""

MAP_I32_PURE = """\
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
}

impl Drop for HashMap_i32_i32 {
    fn drop(self) -> void {
        map_i32_i32_free(self.handle)
    }
}"""

HASHMAP_UPDATE_PURE = """\
impl HashMap_str_i32 {
    fn update(self, key: string, f: fn(i32) -> i32) -> HashMap_str_i32 {
        if self.contains(key) == 1 {
            let _ = self.insert(key, f(self.get(key)))
        }
        return self
    }
}"""

PURE_MODULES: list[dict] = [
    {
        "recipe": "stdlib-pure",
        "ny_module": "vec_str.ny",
        "fn_name": "strvec_set_method",
        "pure_source": STRVEC_SET_PURE,
    },
    {
        "recipe": "stdlib-pure",
        "ny_module": "vec.ny",
        "fn_name": "vec_i32_swap_extend",
        "pure_source": VECI32_SWAP_EXTEND_PURE,
    },
    {
        "recipe": "stdlib-pure",
        "ny_module": "map.ny",
        "fn_name": "hashmap_i32_i32",
        "pure_source": MAP_I32_PURE,
    },
    {
        "recipe": "stdlib-pure",
        "ny_module": "map.ny",
        "fn_name": "hashmap_update",
        "pure_source": HASHMAP_UPDATE_PURE,
    },
]

STRING_TEST_CASES: dict[str, list[str]] = {
    "before_sep": [
        '    assert_str_eq("a:b".before_sep(":"), "a")',
        '    assert_str_eq("abc".before_sep(":"), "abc")',
    ],
    "is_ascii": [
        '    assert_eq("hello".is_ascii(), 1)',
        '    assert_eq(if "café".is_ascii() == 0 { 1 } else { 0 }, 1)',
    ],
    "collapse_ws": [
        '    assert_str_eq("  a   b  ".collapse_ws(), "a b")',
    ],
}
