"""Batch3 gap-fill catalog — definitions for `gen_batch3.py`.

Each entry is emitted as JSON for `make batch-add-builtin BATCH=batch3`.
C bodies and Nyra pure_source are included so scaffolds ship with real logic.
"""
from __future__ import annotations

# --- String built-in methods (builtin-dev → batch3/*.json) ---

STRING_BUILTINS: list[dict] = [
    {
        "receiver": "string",
        "method": "equal_fold",
        "args": ["other:string"],
        "returns": "i32",
        "c_name": "str_equal_fold",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s || !other) return 0;
    size_t i = 0;
    while (s[i] && other[i]) {
        unsigned char a = (unsigned char)s[i];
        unsigned char b = (unsigned char)other[i];
        if (a >= 'A' && a <= 'Z') a = (unsigned char)(a + 32);
        if (b >= 'A' && b <= 'Z') b = (unsigned char)(b + 32);
        if (a != b) return 0;
        i++;
    }
    return (s[i] == '\\0' && other[i] == '\\0') ? 1 : 0;""",
    },
    {
        "receiver": "string",
        "method": "index_byte",
        "args": ["byte:i32"],
        "returns": "i32",
        "c_name": "str_index_byte",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s) return -1;
    unsigned char ch = (unsigned char)byte;
    for (int i = 0; s[i]; i++) {
        if ((unsigned char)s[i] == ch) return i;
    }
    return -1;""",
    },
    {
        "receiver": "string",
        "method": "last_index_byte",
        "args": ["byte:i32"],
        "returns": "i32",
        "c_name": "str_last_index_byte",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s) return -1;
    unsigned char ch = (unsigned char)byte;
    int last = -1;
    for (int i = 0; s[i]; i++) {
        if ((unsigned char)s[i] == ch) last = i;
    }
    return last;""",
    },
    {
        "receiver": "string",
        "method": "compare",
        "args": ["other:string"],
        "returns": "i32",
        "c_name": "str_compare",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s && !other) return 0;
    if (!s) return -1;
    if (!other) return 1;
    return strcmp(s, other);""",
    },
    {
        "receiver": "string",
        "method": "starts_with",
        "args": ["prefix:string"],
        "returns": "i32",
        "c_name": "str_starts_with",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s || !prefix) return 0;
    size_t plen = strlen(prefix);
    if (plen == 0) return 1;
    return strncmp(s, prefix, plen) == 0 ? 1 : 0;""",
    },
    {
        "receiver": "string",
        "method": "ends_with",
        "args": ["suffix:string"],
        "returns": "i32",
        "c_name": "str_ends_with",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s || !suffix) return 0;
    size_t slen = strlen(s);
    size_t suflen = strlen(suffix);
    if (suflen > slen) return 0;
    return strcmp(s + slen - suflen, suffix) == 0 ? 1 : 0;""",
    },
    {
        "receiver": "string",
        "method": "contains",
        "args": ["needle:string"],
        "returns": "i32",
        "c_name": "str_contains",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": "    return strstr_pos(s, needle) >= 0 ? 1 : 0;",
    },
    {
        "receiver": "string",
        "method": "trim",
        "args": [],
        "returns": "string",
        "c_name": "str_trim",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s) return str_dup("");
    const char *start = s;
    while (*start && (*start == ' ' || *start == '\\t' || *start == '\\n' || *start == '\\r')) start++;
    const char *end = s + strlen(s);
    while (end > start && (end[-1] == ' ' || end[-1] == '\\t' || end[-1] == '\\n' || end[-1] == '\\r')) end--;
    size_t len = (size_t)(end - start);
    char *out = (char *)malloc(len + 1);
    if (!out) return NULL;
    memcpy(out, start, len);
    out[len] = '\\0';
    return out;""",
    },
    {
        "receiver": "string",
        "method": "replace",
        "args": ["from:string", "to:string"],
        "returns": "string",
        "c_name": "str_replace",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": "    return str_replacen(s, from, to, -1);",
    },
    {
        "receiver": "string",
        "method": "replacen",
        "args": ["from:string", "to:string", "count:i32"],
        "returns": "string",
        "c_name": "str_replacen",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": "    return str_replacen(s, from, to, count);",
    },
    {
        "receiver": "string",
        "method": "substring",
        "args": ["start:i32", "len:i32"],
        "returns": "string",
        "c_name": "substring",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s || start < 0 || len < 0) return str_dup("");
    size_t slen = strlen(s);
    if ((size_t)start >= slen) return str_dup("");
    if ((size_t)(start + len) > slen) len = (int)(slen - (size_t)start);
    char *out = (char *)malloc((size_t)len + 1);
    if (!out) return NULL;
    memcpy(out, s + start, (size_t)len);
    out[len] = '\\0';
    return out;""",
    },
    {
        "receiver": "string",
        "method": "push_char",
        "args": ["ch:i32"],
        "returns": "string",
        "c_name": "str_push_char",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s) s = "";
    size_t len = strlen(s);
    char *out = (char *)malloc(len + 2);
    if (!out) return NULL;
    memcpy(out, s, len);
    out[len] = (char)(unsigned char)ch;
    out[len + 1] = '\\0';
    return out;""",
    },
    {
        "receiver": "string",
        "method": "pop",
        "args": [],
        "returns": "string",
        "c_name": "str_pop",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s || s[0] == '\\0') return str_dup("");
    size_t len = strlen(s);
    char *out = (char *)malloc(len);
    if (!out) return NULL;
    memcpy(out, s, len - 1);
    out[len - 1] = '\\0';
    return out;""",
    },
    {
        "receiver": "string",
        "method": "strip_ansi",
        "args": [],
        "returns": "string",
        "c_name": "str_strip_ansi",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s) return str_dup("");
    size_t cap = strlen(s) + 1;
    char *out = (char *)malloc(cap);
    if (!out) return NULL;
    size_t oi = 0;
    for (size_t i = 0; s[i]; ) {
        if (s[i] == '\\033' && s[i + 1] == '[') {
            i += 2;
            while (s[i] && s[i] != 'm') i++;
            if (s[i] == 'm') i++;
            continue;
        }
        out[oi++] = s[i++];
    }
    out[oi] = '\\0';
    return out;""",
    },
    {
        "receiver": "string",
        "method": "after_sep",
        "args": ["sep:string"],
        "returns": "string",
        "c_name": "str_after_sep",
        "rt_module": "rt_strings.c",
        "borrows_receiver": True,
        "free_fn_alias": True,
        "c_body": """\
    if (!s) return str_dup("");
    if (!sep || sep[0] == '\\0') return str_dup(s);
    const char *p = strstr(s, sep);
    if (!p) return str_dup("");
    return str_dup(p + strlen(sep));""",
    },
]

# --- Stdlib extern (contrib batch3) ---

MATH_EXTERN: list[dict] = [
    {
        "ny_module": "math.ny",
        "fn_name": "floor_i32",
        "args": ["x:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "c_body": "    return (int)__builtin_floor((double)x);",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "ceil_i32",
        "args": ["x:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "c_body": "    return (int)__builtin_ceil((double)x);",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "round_i32",
        "args": ["x:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "c_body": "    return (int)__builtin_round((double)x);",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "signum_f64",
        "args": ["x:f64"],
        "returns": "f64",
        "rt_module": "rt_math.c",
        "ny_alias": "signum",
        "c_body": """\
    if (x > 0.0) return 1.0;
    if (x < 0.0) return -1.0;
    return 0.0;""",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "is_nan_f64",
        "args": ["x:f64"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "ny_alias": "is_nan",
        "c_body": "    return __builtin_isnan(x) ? 1 : 0;",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "is_finite_f64",
        "args": ["x:f64"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "ny_alias": "is_finite",
        "c_body": "    return __builtin_isfinite(x) ? 1 : 0;",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "signum_i32",
        "args": ["x:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "c_body": """\
    if (x > 0) return 1;
    if (x < 0) return -1;
    return 0;""",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "trunc_i32",
        "args": ["x:i32"],
        "returns": "i32",
        "rt_module": "rt_math.c",
        "c_body": "    return (int)__builtin_trunc((double)x);",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "fmod_f64",
        "args": ["x:f64", "y:f64"],
        "returns": "f64",
        "rt_module": "rt_math.c",
        "ny_alias": "fmod",
        "c_body": """\
    if (y == 0.0) return 0.0;
    int n = (int)(x / y);
    double r = x - (double)n * y;
    if ((r > 0.0) != (x > 0.0)) r += (x > 0.0 ? y : -y);
    return r;""",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "copysign_f64",
        "args": ["x:f64", "y:f64"],
        "returns": "f64",
        "rt_module": "rt_math.c",
        "c_body": "    return __builtin_copysign(x, y);",
    },
    {
        "ny_module": "math.ny",
        "fn_name": "lerp_f64",
        "args": ["a:f64", "b:f64", "t:f64"],
        "returns": "f64",
        "rt_module": "rt_math.c",
        "ny_alias": "lerp",
        "c_body": "    return a + (b - a) * t;",
    },
]

STRCONV_EXTERN: list[dict] = [
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "str_to_i64",
        "args": ["s:string"],
        "returns": "i64",
        "rt_module": "rt_strings.c",
        "c_body": """\
    if (!s) return 0;
    return (long long)strtoll(s, NULL, 10);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "parse_int_base",
        "args": ["s:string", "base:i32"],
        "returns": "i32",
        "rt_module": "rt_strings.c",
        "ny_alias": "parse_int",
        "c_body": """\
    if (!s) return 0;
    return (int)strtol(s, NULL, base);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "f64_to_string_prec",
        "args": ["n:f64", "prec:i32"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "ny_alias": "format_f64",
        "c_body": """\
    extern char *str_dup(const char *s);
    char buf[64];
    if (prec < 0) prec = 0;
    if (prec > 12) prec = 12;
    snprintf(buf, sizeof(buf), "%.*f", prec, n);
    return str_dup(buf);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "i64_to_string",
        "args": ["n:i64"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "ny_alias": "format_i64",
        "c_body": """\
    extern char *str_dup(const char *s);
    char buf[32];
    int len = snprintf(buf, sizeof(buf), "%lld", n);
    if (len < 0) return str_dup("0");
    return str_dup(buf);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "str_to_u64",
        "args": ["s:string"],
        "returns": "i64",
        "rt_module": "rt_strings.c",
        "ny_alias": "parse_u64",
        "c_body": """\
    if (!s) return 0;
    return (long long)strtoull(s, NULL, 10);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "parse_uint_base",
        "args": ["s:string", "base:i32"],
        "returns": "i32",
        "rt_module": "rt_strings.c",
        "ny_alias": "parse_uint",
        "c_body": """\
    if (!s) return 0;
    return (int)strtoul(s, NULL, base);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "format_i32_hex",
        "args": ["n:i32"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "ny_alias": "format_hex",
        "c_body": """\
    extern char *str_dup(const char *s);
    char buf[16];
    snprintf(buf, sizeof(buf), "%x", n);
    return str_dup(buf);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "str_to_f64",
        "args": ["s:string"],
        "returns": "f64",
        "rt_module": "rt_strings.c",
        "ny_alias": "parse_f64",
        "c_body": """\
    if (!s) return 0.0;
    return strtod(s, NULL);""",
    },
]

VEC_STR_EXTERN: list[dict] = [
    {
        "ny_module": "vec_str.ny",
        "fn_name": "vec_str_pop",
        "args": ["handle:ptr"],
        "returns": "string",
        "rt_module": "rt_vec.c",
        "c_body": """\
    extern char *str_dup(const char *s);
    NyraVec *v = (NyraVec *)handle;
    if (!v || v->len <= 0) return str_dup("");
    char *top = ((char **)v->data)[--v->len];
    char *out = top ? str_dup(top) : str_dup("");
    free(top);
    return out;""",
    },
    {
        "ny_module": "vec_str.ny",
        "fn_name": "vec_str_clear",
        "args": ["handle:ptr"],
        "returns": "void",
        "rt_module": "rt_vec.c",
        "c_body": """\
    NyraVec *v = (NyraVec *)handle;
    if (!v) return;
    for (int i = 0; i < v->len; i++) free(((char **)v->data)[i]);
    v->len = 0;""",
    },
    {
        "ny_module": "vec_str.ny",
        "fn_name": "vec_str_reverse",
        "args": ["handle:ptr"],
        "returns": "void",
        "rt_module": "rt_vec.c",
        "c_body": """\
    NyraVec *v = (NyraVec *)handle;
    if (!v || v->len <= 1) return;
    char **data = (char **)v->data;
    int lo = 0, hi = v->len - 1;
    while (lo < hi) {
        char *tmp = data[lo];
        data[lo] = data[hi];
        data[hi] = tmp;
        lo++; hi--;
    }""",
    },
    {
        "ny_module": "vec_str.ny",
        "fn_name": "vec_str_insert",
        "args": ["handle:ptr", "index:i32", "value:string"],
        "returns": "void",
        "rt_module": "rt_vec.c",
        "c_body": """\
    extern char *str_dup(const char *s);
    NyraVec *v = (NyraVec *)handle;
    if (!v || index < 0 || index > v->len) return;
    if (v->len >= v->cap) {
        int nc = v->cap * 2;
        void *nd = realloc(v->data, (size_t)nc * (size_t)v->elem_size);
        if (!nd) return;
        v->data = nd;
        v->cap = nc;
    }
    char **data = (char **)v->data;
    memmove(data + index + 1, data + index, (size_t)(v->len - index) * sizeof(char *));
    data[index] = str_dup(value ? value : "");
    v->len++;""",
    },
    {
        "ny_module": "vec_str.ny",
        "fn_name": "vec_str_remove_at",
        "args": ["handle:ptr", "index:i32"],
        "returns": "string",
        "rt_module": "rt_vec.c",
        "c_body": """\
    extern char *str_dup(const char *s);
    NyraVec *v = (NyraVec *)handle;
    if (!v || index < 0 || index >= v->len) return str_dup("");
    char **data = (char **)v->data;
    char *removed = data[index];
    memmove(data + index, data + index + 1, (size_t)(v->len - index - 1) * sizeof(char *));
    v->len--;
    char *out = removed ? str_dup(removed) : str_dup("");
    free(removed);
    return out;""",
    },
    {
        "ny_module": "vec_str.ny",
        "fn_name": "vec_str_swap",
        "args": ["handle:ptr", "i:i32", "j:i32"],
        "returns": "void",
        "rt_module": "rt_vec.c",
        "c_body": """\
    NyraVec *v = (NyraVec *)handle;
    if (!v || i < 0 || j < 0 || i >= v->len || j >= v->len) return;
    char **data = (char **)v->data;
    char *tmp = data[i];
    data[i] = data[j];
    data[j] = tmp;""",
    },
    {
        "ny_module": "vec_str.ny",
        "fn_name": "vec_str_set",
        "args": ["handle:ptr", "index:i32", "value:string"],
        "returns": "void",
        "rt_module": "rt_vec.c",
        "c_body": """\
    extern char *str_dup(const char *s);
    NyraVec *v = (NyraVec *)handle;
    if (!v || index < 0 || index >= v->len) return;
    char **data = (char **)v->data;
    free(data[index]);
    data[index] = str_dup(value ? value : "");""",
    },
    {
        "ny_module": "vec_str.ny",
        "fn_name": "vec_str_extend",
        "args": ["dst:ptr", "src:ptr"],
        "returns": "void",
        "rt_module": "rt_vec.c",
        "c_body": """\
    extern void vec_str_push(void *handle, const char *value);
    NyraVec *d = (NyraVec *)dst;
    NyraVec *s = (NyraVec *)src;
    if (!d || !s) return;
    for (int i = 0; i < s->len; i++) {
        const char *item = ((char **)s->data)[i];
        vec_str_push(d, item ? item : "");
    }""",
    },
]

FORMAT_EXTERN: list[dict] = [
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "format_i32_pad",
        "args": ["n:i32", "width:i32"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "ny_alias": "format_pad",
        "c_body": """\
    extern char *str_dup(const char *s);
    char buf[32];
    if (width < 0) width = 0;
    if (width > 20) width = 20;
    snprintf(buf, sizeof(buf), "%0*d", width, n);
    return str_dup(buf);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "format_bool",
        "args": ["b:i32"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "ny_alias": "bool_to_string",
        "c_body": """\
    extern char *str_dup(const char *s);
    return str_dup(b ? "true" : "false");""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "format_i64_pad",
        "args": ["n:i64", "width:i32"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "c_body": """\
    extern char *str_dup(const char *s);
    char buf[48];
    if (width < 0) width = 0;
    if (width > 24) width = 24;
    snprintf(buf, sizeof(buf), "%0*lld", width, n);
    return str_dup(buf);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "format_f64_pad",
        "args": ["n:f64", "width:i32", "prec:i32"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "c_body": """\
    extern char *str_dup(const char *s);
    char buf[64];
    if (width < 0) width = 0;
    if (prec < 0) prec = 0;
    if (prec > 12) prec = 12;
    snprintf(buf, sizeof(buf), "%*.*f", width, prec, n);
    return str_dup(buf);""",
    },
    {
        "ny_module": "strconv/mod.ny",
        "fn_name": "format_i32_hex_pad",
        "args": ["n:i32", "width:i32"],
        "returns": "string",
        "rt_module": "rt_strings.c",
        "c_body": """\
    extern char *str_dup(const char *s);
    char buf[24];
    if (width < 0) width = 0;
    if (width > 16) width = 16;
    snprintf(buf, sizeof(buf), "%0*x", width, n);
    return str_dup(buf);""",
    },
]

# --- Pure Nyra modules (contrib batch3 pure_*.json) ---

STRVEC_PURE_SOURCE = """\
impl StrVec {
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
}"""

OPTION_PURE_SOURCE = """\
import "../result.ny"

fn Option_i32_map(opt: Option_i32, f: fn(i32) -> i32) -> Option_i32 {
    return match opt {
        Option_i32.Some(v) => Option_i32.Some(f(v))
        Option_i32.None => Option_i32.None
    }
}

fn Option_i32_and_then(opt: Option_i32, f: fn(i32) -> Option_i32) -> Option_i32 {
    return match opt {
        Option_i32.Some(v) => f(v)
        Option_i32.None => Option_i32.None
    }
}

fn Option_i32_unwrap_or(opt: Option_i32, default_val: i32) -> i32 {
    return match opt {
        Option_i32.Some(v) => v
        Option_i32.None => default_val
    }
}

fn Option_i32_is_none(opt: Option_i32) -> i32 {
    return match opt {
        Option_i32.Some(_v) => 0
        Option_i32.None => 1
    }
}

fn Option_i32_ok_or(opt: Option_i32, err: i32) -> Result_i32_i32 {
    return match opt {
        Option_i32.Some(v) => Result_i32_i32.Ok(v)
        Option_i32.None => Result_i32_i32.Err(err)
    }
}"""

RESULT_PURE_SOURCE = """\
import "../result.ny"

fn Result_i32_i32_map(r: Result_i32_i32, f: fn(i32) -> i32) -> Result_i32_i32 {
    return match r {
        Result_i32_i32.Ok(v) => Result_i32_i32.Ok(f(v))
        Result_i32_i32.Err(e) => Result_i32_i32.Err(e)
    }
}

fn Result_i32_i32_map_err(r: Result_i32_i32, f: fn(i32) -> i32) -> Result_i32_i32 {
    return match r {
        Result_i32_i32.Ok(v) => Result_i32_i32.Ok(v)
        Result_i32_i32.Err(e) => Result_i32_i32.Err(f(e))
    }
}

fn Result_i32_i32_and_then(r: Result_i32_i32, f: fn(i32) -> Result_i32_i32) -> Result_i32_i32 {
    return match r {
        Result_i32_i32.Ok(v) => f(v)
        Result_i32_i32.Err(e) => Result_i32_i32.Err(e)
    }
}

fn Result_i32_i32_unwrap_or(r: Result_i32_i32, default_val: i32) -> i32 {
    return match r {
        Result_i32_i32.Ok(v) => v
        Result_i32_i32.Err(_e) => default_val
    }
}

fn Result_i32_i32_is_err(r: Result_i32_i32) -> i32 {
    return match r {
        Result_i32_i32.Ok(_v) => 0
        Result_i32_i32.Err(_e) => 1
    }
}"""

MAP_PURE_SOURCE = """\
impl HashMap_str_i32 {
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
}

impl HashMap_str_str {
    fn or_insert(self, key: string, value: string) -> string {
        if self.contains(key) == 1 {
            return self.get(key)
        }
        let _ = self.insert(key, value)
        return value
    }
}"""

STRVEC_INSERT_PURE_SOURCE = """\
impl StrVec {
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
}"""

MAP_EXTRA_PURE_SOURCE = """\
impl HashMap_str_i32 {
    fn is_empty(self) -> i32 {
        if self.len() == 0 {
            return 1
        }
        return 0
    }
}

impl HashMap_str_str {
    fn get_or_insert(self, key: string, value: string) -> string {
        return self.or_insert(key, value)
    }

    fn is_empty(self) -> i32 {
        if self.len() == 0 {
            return 1
        }
        return 0
    }
}"""

STRING_TEST_CASES: dict[str, list[str]] = {
    "equal_fold": [
        '    assert_eq("Hello".equal_fold("hello"), 1)',
        '    assert_eq("Hello".equal_fold("world"), 0)',
    ],
    "index_byte": [
        '    assert_eq("hello".index_byte(101), 1)',
        '    assert_eq("hello".index_byte(120), -1)',
    ],
    "last_index_byte": [
        '    assert_eq("hello".last_index_byte(108), 3)',
        '    assert_eq("hello".last_index_byte(120), -1)',
    ],
    "compare": [
        '    assert_eq("abc".compare("abc"), 0)',
        '    assert_eq(if "abc".compare("abd") < 0 { 1 } else { 0 }, 1)',
    ],
    "starts_with": [
        '    assert_eq("hello".starts_with("he"), 1)',
        '    assert_eq("hello".starts_with("lo"), 0)',
    ],
    "ends_with": [
        '    assert_eq("hello".ends_with("lo"), 1)',
        '    assert_eq("hello".ends_with("he"), 0)',
    ],
    "contains": [
        '    assert_eq("hello".contains("ell"), 1)',
        '    assert_eq("hello".contains("xyz"), 0)',
    ],
    "trim": [
        '    assert_str_eq("  hi  ".trim(), "hi")',
    ],
    "replace": [
        '    assert_str_eq("a-b-a".replace("a", "x"), "x-b-x")',
    ],
    "replacen": [
        '    assert_str_eq("a-b-a".replacen("a", "x", 1), "x-b-a")',
    ],
    "substring": [
        '    assert_str_eq("hello".substring(1, 3), "ell")',
    ],
    "push_char": [
        '    assert_str_eq("ab".push_char(99), "abc")',
    ],
    "pop": [
        '    assert_str_eq("abc".pop(), "ab")',
    ],
    "strip_ansi": [
        '    assert_str_eq("\\033[31mok\\033[0m".strip_ansi(), "ok")',
    ],
    "after_sep": [
        '    assert_str_eq("a:b:c".after_sep(":"), "b:c")',
    ],
}

BUILTINS_MATH_PATCHES = {
    "fn Math_floor(x: i32) -> i32 {\n    return x\n}": "fn Math_floor(x: i32) -> i32 {\n    return floor_i32(x)\n}",
    "fn Math_ceil(x: i32) -> i32 {\n    return x\n}": "fn Math_ceil(x: i32) -> i32 {\n    return ceil_i32(x)\n}",
    "fn Math_round(x: i32) -> i32 {\n    return x\n}": "fn Math_round(x: i32) -> i32 {\n    return round_i32(x)\n}",
}

PURE_MODULES: list[dict] = [
    {
        "recipe": "stdlib-pure",
        "ny_module": "vec_str.ny",
        "fn_name": "strvec_methods",
        "pure_source": STRVEC_PURE_SOURCE,
    },
    {
        "recipe": "stdlib-pure",
        "ny_module": "map.ny",
        "fn_name": "hashmap_or_insert",
        "pure_source": MAP_PURE_SOURCE,
    },
    {
        "recipe": "stdlib-pure",
        "ny_module": "vec_str.ny",
        "fn_name": "strvec_insert_extend",
        "pure_source": STRVEC_INSERT_PURE_SOURCE,
    },
    {
        "recipe": "stdlib-pure",
        "ny_module": "map.ny",
        "fn_name": "hashmap_extra_methods",
        "pure_source": MAP_EXTRA_PURE_SOURCE,
    },
]
