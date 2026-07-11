#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

extern int vec_str_len(void *handle);
extern const char *vec_str_get(void *handle, int index);
extern void vec_str_push(void *handle, const char *value);
extern void *vec_str_new(void);
extern char *str_cat(const char *a, const char *b);

static char *dup_slice(const char *start, size_t len) {
    char *out = (char *)malloc(len + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, start, len);
    out[len] = '\0';
    return out;
}

static const char *find_key_colon(const char *json, const char *key) {
    if (!json || !key) {
        return NULL;
    }
    char pattern[128];
    snprintf(pattern, sizeof(pattern), "\"%s\"", key);
    const char *k = strstr(json, pattern);
    if (!k) {
        return NULL;
    }
    return strchr(k + strlen(pattern), ':');
}

static const char *json_value_start(const char *json, const char *key) {
    const char *colon = find_key_colon(json, key);
    if (!colon) {
        return NULL;
    }
    const char *p = colon + 1;
    while (*p == ' ' || *p == '\t' || *p == '\n' || *p == '\r') {
        p++;
    }
    return p;
}

int json_has_key(const char *json, const char *key) {
    return find_key_colon(json, key) ? 1 : 0;
}

int json_has_string(const char *json, const char *key) {
    const char *p = json_value_start(json, key);
    return (p && *p == '"') ? 1 : 0;
}

int json_has_i32(const char *json, const char *key) {
    const char *p = json_value_start(json, key);
    if (!p) {
        return 0;
    }
    if (*p == '-') {
        p++;
    }
    return (*p >= '0' && *p <= '9') ? 1 : 0;
}

int json_has_bool(const char *json, const char *key) {
    const char *p = json_value_start(json, key);
    if (!p) {
        return 0;
    }
    return (strncmp(p, "true", 4) == 0 || strncmp(p, "false", 5) == 0) ? 1 : 0;
}

/* Minimal JSON string field extractor for {"key":"value"} objects. */
char *json_get_string(const char *json, const char *key) {
    const char *colon = find_key_colon(json, key);
    if (!colon) {
        return NULL;
    }
    const char *q = strchr(colon + 1, '"');
    if (!q) {
        return NULL;
    }
    q++;
    const char *end = strchr(q, '"');
    if (!end) {
        return NULL;
    }
    return dup_slice(q, (size_t)(end - q));
}

/* Parse a JSON number field after "key": (integer only). Returns -2147483648 if missing. */
int json_get_i32(const char *json, const char *key) {
    const char *colon = find_key_colon(json, key);
    if (!colon) {
        return -2147483648;
    }
    const char *p = colon + 1;
    while (*p == ' ' || *p == '\t') {
        p++;
    }
    int sign = 1;
    if (*p == '-') {
        sign = -1;
        p++;
    }
    if (*p < '0' || *p > '9') {
        return -2147483648;
    }
    long long v = 0;
    while (*p >= '0' && *p <= '9') {
        v = v * 10 + (*p - '0');
        p++;
    }
    return (int)(v * sign);
}

/* 1 = true, 0 = false, -1 = missing/invalid */
int json_get_bool(const char *json, const char *key) {
    const char *colon = find_key_colon(json, key);
    if (!colon) {
        return -1;
    }
    const char *p = colon + 1;
    while (*p == ' ' || *p == '\t') {
        p++;
    }
    if (strncmp(p, "true", 4) == 0) {
        return 1;
    }
    if (strncmp(p, "false", 5) == 0) {
        return 0;
    }
    return -1;
}

/* Extract nested object value for "key":{...} (one level, balanced braces). */
char *json_get_object(const char *json, const char *key) {
    const char *colon = find_key_colon(json, key);
    if (!colon) {
        return NULL;
    }
    const char *p = colon + 1;
    while (*p == ' ' || *p == '\t') {
        p++;
    }
    if (*p != '{') {
        return NULL;
    }
    const char *start = p;
    int depth = 0;
    for (; *p; p++) {
        if (*p == '{') {
            depth++;
        } else if (*p == '}') {
            depth--;
            if (depth == 0) {
                return dup_slice(start, (size_t)(p - start + 1));
            }
        }
    }
    return NULL;
}

static int json_value_is_integer(const char *v) {
    if (!v || !*v) {
        return 0;
    }
    const char *p = v;
    if (*p == '-') {
        p++;
        if (!*p) {
            return 0;
        }
    }
    if (*p < '0' || *p > '9') {
        return 0;
    }
    while (*p >= '0' && *p <= '9') {
        p++;
    }
    return *p == '\0';
}

char *json_encode_object(void *keys_vec, void *values_vec) {
    if (!keys_vec || !values_vec) {
        return NULL;
    }
    int n = vec_str_len(keys_vec);
    if (n != vec_str_len(values_vec) || n < 0) {
        return NULL;
    }
    char *out = (char *)malloc(2);
    if (!out) {
        return NULL;
    }
    out[0] = '{';
    out[1] = '\0';
    for (int i = 0; i < n; i++) {
        const char *k = vec_str_get(keys_vec, i);
        const char *v = vec_str_get(values_vec, i);
        if (!k || !v) {
            free(out);
            return NULL;
        }
        if (i > 0) {
            char *tmp = str_cat(out, ",");
            free(out);
            out = tmp;
        }
        int is_nested = (v[0] == '{' || v[0] == '[');
        int is_num = json_value_is_integer(v);
        int is_bool = (strcmp(v, "true") == 0 || strcmp(v, "false") == 0);
        char *part = NULL;
        if (is_nested || is_num || is_bool) {
            size_t plen = strlen(k) + strlen(v) + 5;
            part = (char *)malloc(plen);
            if (!part) {
                free(out);
                return NULL;
            }
            snprintf(part, plen, "\"%s\":%s", k, v);
        } else {
            part = (char *)malloc(strlen(k) + strlen(v) + 6);
            if (!part) {
                free(out);
                return NULL;
            }
            snprintf(part, strlen(k) + strlen(v) + 6, "\"%s\":\"%s\"", k, v);
        }
        char *tmp2 = str_cat(out, part);
        free(part);
        free(out);
        out = tmp2;
    }
    char *done = str_cat(out, "}");
    free(out);
    return done;
}

extern void *vec_i32_new(void);
extern void vec_i32_push(void *handle, int value);
extern int vec_i32_get(void *handle, int index);
extern int vec_i32_len(void *handle);

/* Extract array value for "key":[...] (balanced brackets). */
char *json_get_array(const char *json, const char *key) {
    const char *colon = find_key_colon(json, key);
    if (!colon) {
        return NULL;
    }
    const char *p = colon + 1;
    while (*p == ' ' || *p == '\t') {
        p++;
    }
    if (*p != '[') {
        return NULL;
    }
    const char *start = p;
    int depth = 0;
    for (; *p; p++) {
        if (*p == '[') {
            depth++;
        } else if (*p == ']') {
            depth--;
            if (depth == 0) {
                return dup_slice(start, (size_t)(p - start + 1));
            }
        }
    }
    return NULL;
}

char *json_encode_i32_array(void *handle) {
    if (!handle) {
        return NULL;
    }
    int n = vec_i32_len(handle);
    char *out = (char *)malloc(2);
    if (!out) {
        return NULL;
    }
    out[0] = '[';
    out[1] = '\0';
    for (int i = 0; i < n; i++) {
        char num[32];
        snprintf(num, sizeof(num), "%d", vec_i32_get(handle, i));
        if (i > 0) {
            char *tmp = str_cat(out, ",");
            free(out);
            out = tmp;
        }
        char *tmp2 = str_cat(out, num);
        free(out);
        out = tmp2;
    }
    char *done = str_cat(out, "]");
    free(out);
    return done;
}

extern void *vec_str_new(void);
extern void vec_str_push(void *handle, const char *value);
extern const char *vec_str_get(void *handle, int index);

char *json_encode_str_array(void *handle) {
    if (!handle) {
        return NULL;
    }
    int n = vec_str_len(handle);
    char *out = (char *)malloc(2);
    if (!out) {
        return NULL;
    }
    out[0] = '[';
    out[1] = '\0';
    for (int i = 0; i < n; i++) {
        const char *s = vec_str_get(handle, i);
        if (!s) {
            s = "";
        }
        if (i > 0) {
            char *tmp = str_cat(out, ",");
            free(out);
            out = tmp;
        }
        size_t slen = strlen(s);
        size_t plen = slen + 3;
        char *part = (char *)malloc(plen);
        if (!part) {
            free(out);
            return NULL;
        }
        part[0] = '"';
        memcpy(part + 1, s, slen);
        part[1 + slen] = '"';
        part[2 + slen] = '\0';
        char *tmp2 = str_cat(out, part);
        free(part);
        free(out);
        out = tmp2;
    }
    char *done = str_cat(out, "]");
    free(out);
    return done;
}

/* Join JSON object literals into a raw array: [{...},{...}] without extra quoting. */
char *json_join_raw_array(void *handle) {
    if (!handle) {
        return dup_slice("[]", 2);
    }
    int n = vec_str_len(handle);
    if (n <= 0) {
        return dup_slice("[]", 2);
    }
    char *out = (char *)malloc(2);
    if (!out) {
        return NULL;
    }
    out[0] = '[';
    out[1] = '\0';
    for (int i = 0; i < n; i++) {
        const char *part = vec_str_get(handle, i);
        if (!part) {
            part = "";
        }
        if (i > 0) {
            char *tmp = str_cat(out, ",");
            free(out);
            out = tmp;
        }
        char *tmp2 = str_cat(out, part);
        free(out);
        out = tmp2;
    }
    char *done = str_cat(out, "]");
    free(out);
    return done;
}

void *json_decode_str_array(const char *array_json) {
    void *v = vec_str_new();
    if (!v || !array_json) {
        return v;
    }
    const char *p = array_json;
    while (*p && *p != '[') {
        p++;
    }
    if (*p != '[') {
        return v;
    }
    p++;
    while (*p && *p != ']') {
        while (*p == ' ' || *p == '\t' || *p == ',') {
            p++;
        }
        if (*p == ']' || !*p) {
            break;
        }
        if (*p != '"') {
            while (*p && *p != ',' && *p != ']') {
                p++;
            }
            continue;
        }
        p++;
        const char *start = p;
        while (*p && *p != '"') {
            p++;
        }
        if (*p == '"') {
            char *slice = dup_slice(start, (size_t)(p - start));
            vec_str_push(v, slice ? slice : "");
            free(slice);
            p++;
        }
        while (*p && *p != ',' && *p != ']') {
            p++;
        }
    }
    return v;
}

void *json_split_array_elements(const char *array_json) {
    void *v = vec_str_new();
    if (!v || !array_json) {
        return v;
    }
    const char *p = array_json;
    while (*p && *p != '[') {
        p++;
    }
    if (*p != '[') {
        return v;
    }
    p++;
    while (*p) {
        while (*p == ' ' || *p == '\t' || *p == '\n' || *p == '\r' || *p == ',') {
            p++;
        }
        if (*p == ']' || !*p) {
            break;
        }
        const char *start = p;
        int depth = 0;
        int in_string = 0;
        while (*p) {
            char c = *p;
            if (in_string) {
                if (c == '"' && p > start && p[-1] != '\\') {
                    in_string = 0;
                }
                p++;
                continue;
            }
            if (c == '"') {
                in_string = 1;
                p++;
                continue;
            }
            if (c == '{' || c == '[') {
                depth++;
            } else if (c == '}' || c == ']') {
                if (depth == 0) {
                    break;
                }
                depth--;
            } else if (c == ',' && depth == 0) {
                break;
            }
            p++;
        }
        size_t len = (size_t)(p - start);
        while (len > 0 && (start[len - 1] == ' ' || start[len - 1] == '\t')) {
            len--;
        }
        if (len > 0) {
            char *slice = dup_slice(start, len);
            if (slice) {
                vec_str_push(v, slice);
                free(slice);
            }
        }
        if (*p == ',') {
            p++;
        }
    }
    return v;
}

void *json_decode_i32_array(const char *array_json) {
    void *v = vec_i32_new();
    if (!v || !array_json) {
        return v;
    }
    const char *p = array_json;
    while (*p && *p != '[') {
        p++;
    }
    if (*p != '[') {
        return v;
    }
    p++;
    while (*p && *p != ']') {
        while (*p == ' ' || *p == '\t' || *p == ',') {
            p++;
        }
        if (*p == ']' || !*p) {
            break;
        }
        int sign = 1;
        if (*p == '-') {
            sign = -1;
            p++;
        }
        long long val = 0;
        int any = 0;
        while (*p >= '0' && *p <= '9') {
            val = val * 10 + (*p - '0');
            p++;
            any = 1;
        }
        if (any) {
            vec_i32_push(v, (int)(val * sign));
        }
        while (*p && *p != ',' && *p != ']') {
            p++;
        }
    }
    return v;
}

/* Opaque pointer as JSON integer token (same-process roundtrip; not for persisted handles). */
char *json_encode_ptr_token(void *p) {
    char buf[32];
    snprintf(buf, sizeof(buf), "%lld", (long long)(intptr_t)p);
    return dup_slice(buf, strlen(buf));
}

void *json_decode_ptr_token(const char *json, const char *key) {
    int v = json_get_i32(json, key);
    if (v == -2147483648) {
        return NULL;
    }
    return (void *)(intptr_t)v;
}



/* Top-level object keys (StrVec). Empty vec if not an object. */
void *json_top_keys(const char *json) {
    void *v = vec_str_new();
    if (!v || !json) {
        return v;
    }
    const char *p = json;
    while (*p == ' ' || *p == '\t' || *p == '\n' || *p == '\r') {
        p++;
    }
    if (*p != '{') {
        return v;
    }
    p++;
    while (*p) {
        while (*p == ' ' || *p == '\t' || *p == '\n' || *p == '\r' || *p == ',') {
            p++;
        }
        if (*p == '}' || !*p) {
            break;
        }
        if (*p != '"') {
            break;
        }
        p++;
        const char *start = p;
        while (*p && *p != '"') {
            if (*p == '\\' && p[1]) {
                p += 2;
                continue;
            }
            p++;
        }
        if (*p != '"') {
            break;
        }
        char *key = dup_slice(start, (size_t)(p - start));
        if (key) {
            vec_str_push(v, key);
            free(key);
        }
        p++;
        while (*p && *p != ':') {
            p++;
        }
        if (*p != ':') {
            break;
        }
        p++;
        while (*p == ' ' || *p == '\t' || *p == '\n' || *p == '\r') {
            p++;
        }
        /* skip value */
        if (*p == '"') {
            p++;
            while (*p && *p != '"') {
                if (*p == '\\' && p[1]) {
                    p += 2;
                    continue;
                }
                p++;
            }
            if (*p == '"') {
                p++;
            }
        } else if (*p == '{' || *p == '[') {
            char open = *p;
            char close = open == '{' ? '}' : ']';
            int depth = 0;
            int in_string = 0;
            while (*p) {
                char c = *p;
                if (in_string) {
                    if (c == '"' && p[-1] != '\\') {
                        in_string = 0;
                    }
                    p++;
                    continue;
                }
                if (c == '"') {
                    in_string = 1;
                    p++;
                    continue;
                }
                if (c == open) {
                    depth++;
                } else if (c == close) {
                    depth--;
                    if (depth == 0) {
                        p++;
                        break;
                    }
                }
                p++;
            }
        } else {
            while (*p && *p != ',' && *p != '}') {
                p++;
            }
        }
    }
    return v;
}

/* Raw JSON value text for a top-level (or nested via find_key) key. */
char *json_raw_get(const char *json, const char *key) {
    const char *p = json_value_start(json, key);
    if (!p) {
        return NULL;
    }
    const char *start = p;
    if (*p == '"') {
        p++;
        while (*p && *p != '"') {
            if (*p == '\\' && p[1]) {
                p += 2;
                continue;
            }
            p++;
        }
        if (*p == '"') {
            p++;
        }
        return dup_slice(start, (size_t)(p - start));
    }
    if (*p == '{' || *p == '[') {
        char open = *p;
        char close = open == '{' ? '}' : ']';
        int depth = 0;
        int in_string = 0;
        while (*p) {
            char c = *p;
            if (in_string) {
                if (c == '"' && p[-1] != '\\') {
                    in_string = 0;
                }
                p++;
                continue;
            }
            if (c == '"') {
                in_string = 1;
                p++;
                continue;
            }
            if (c == open) {
                depth++;
            } else if (c == close) {
                depth--;
                if (depth == 0) {
                    p++;
                    break;
                }
            }
            p++;
        }
        return dup_slice(start, (size_t)(p - start));
    }
    if (strncmp(p, "true", 4) == 0) {
        return dup_slice(p, 4);
    }
    if (strncmp(p, "false", 5) == 0) {
        return dup_slice(p, 5);
    }
    if (strncmp(p, "null", 4) == 0) {
        return dup_slice(p, 4);
    }
    while (*p && *p != ',' && *p != '}' && *p != ']' && *p != ' ' && *p != '\n' && *p != '\r' && *p != '\t') {
        p++;
    }
    return dup_slice(start, (size_t)(p - start));
}

/* Kind: 0=null/invalid, 1=object, 2=array, 3=string, 4=number, 5=bool */
int json_value_kind(const char *json) {
    if (!json) {
        return 0;
    }
    while (*json == ' ' || *json == '\t' || *json == '\n' || *json == '\r') {
        json++;
    }
    if (*json == '{') return 1;
    if (*json == '[') return 2;
    if (*json == '"') return 3;
    if (*json == '-' || (*json >= '0' && *json <= '9')) return 4;
    if (strncmp(json, "true", 4) == 0 || strncmp(json, "false", 5) == 0) return 5;
    if (strncmp(json, "null", 4) == 0) return 0;
    return 0;
}

/* --- Document-level parse / stringify (validate + compact) --- */

typedef struct {
    const char *p;
    char *out;
    size_t len;
    size_t cap;
    int ok;
} JsonDocBuf;

static void jd_fail(JsonDocBuf *d) {
    d->ok = 0;
}

static void jd_emit(JsonDocBuf *d, char c) {
    if (!d->ok) {
        return;
    }
    if (d->len + 1 >= d->cap) {
        size_t ncap = d->cap ? d->cap * 2 : 64;
        char *n = (char *)realloc(d->out, ncap);
        if (!n) {
            jd_fail(d);
            return;
        }
        d->out = n;
        d->cap = ncap;
    }
    d->out[d->len++] = c;
}

static void jd_emit_n(JsonDocBuf *d, const char *s, size_t n) {
    size_t i;
    for (i = 0; i < n; i++) {
        jd_emit(d, s[i]);
    }
}

static void jd_skip_ws(JsonDocBuf *d) {
    while (*d->p == ' ' || *d->p == '\t' || *d->p == '\n' || *d->p == '\r') {
        d->p++;
    }
}

static void jd_parse_value(JsonDocBuf *d);

static void jd_parse_string(JsonDocBuf *d) {
    if (*d->p != '"') {
        jd_fail(d);
        return;
    }
    jd_emit(d, '"');
    d->p++;
    while (*d->p && *d->p != '"') {
        if (*d->p == '\\') {
            jd_emit(d, '\\');
            d->p++;
            if (!*d->p) {
                jd_fail(d);
                return;
            }
            char esc = *d->p;
            if (esc == '"' || esc == '\\' || esc == '/' || esc == 'b' || esc == 'f' ||
                esc == 'n' || esc == 'r' || esc == 't') {
                jd_emit(d, esc);
                d->p++;
            } else if (esc == 'u') {
                jd_emit(d, 'u');
                d->p++;
                int i;
                for (i = 0; i < 4; i++) {
                    char h = *d->p;
                    if (!((h >= '0' && h <= '9') || (h >= 'a' && h <= 'f') || (h >= 'A' && h <= 'F'))) {
                        jd_fail(d);
                        return;
                    }
                    jd_emit(d, h);
                    d->p++;
                }
            } else {
                jd_fail(d);
                return;
            }
            continue;
        }
        if ((unsigned char)*d->p < 0x20) {
            jd_fail(d);
            return;
        }
        jd_emit(d, *d->p);
        d->p++;
    }
    if (*d->p != '"') {
        jd_fail(d);
        return;
    }
    jd_emit(d, '"');
    d->p++;
}

static void jd_parse_number(JsonDocBuf *d) {
    const char *start = d->p;
    if (*d->p == '-') {
        d->p++;
    }
    if (*d->p == '0') {
        d->p++;
    } else if (*d->p >= '1' && *d->p <= '9') {
        while (*d->p >= '0' && *d->p <= '9') {
            d->p++;
        }
    } else {
        jd_fail(d);
        return;
    }
    if (*d->p == '.') {
        d->p++;
        if (*d->p < '0' || *d->p > '9') {
            jd_fail(d);
            return;
        }
        while (*d->p >= '0' && *d->p <= '9') {
            d->p++;
        }
    }
    if (*d->p == 'e' || *d->p == 'E') {
        d->p++;
        if (*d->p == '+' || *d->p == '-') {
            d->p++;
        }
        if (*d->p < '0' || *d->p > '9') {
            jd_fail(d);
            return;
        }
        while (*d->p >= '0' && *d->p <= '9') {
            d->p++;
        }
    }
    jd_emit_n(d, start, (size_t)(d->p - start));
}

static void jd_parse_object(JsonDocBuf *d) {
    if (*d->p != '{') {
        jd_fail(d);
        return;
    }
    jd_emit(d, '{');
    d->p++;
    jd_skip_ws(d);
    if (*d->p == '}') {
        jd_emit(d, '}');
        d->p++;
        return;
    }
    for (;;) {
        jd_skip_ws(d);
        if (*d->p != '"') {
            jd_fail(d);
            return;
        }
        jd_parse_string(d);
        if (!d->ok) {
            return;
        }
        jd_skip_ws(d);
        if (*d->p != ':') {
            jd_fail(d);
            return;
        }
        jd_emit(d, ':');
        d->p++;
        jd_skip_ws(d);
        jd_parse_value(d);
        if (!d->ok) {
            return;
        }
        jd_skip_ws(d);
        if (*d->p == ',') {
            jd_emit(d, ',');
            d->p++;
            continue;
        }
        if (*d->p == '}') {
            jd_emit(d, '}');
            d->p++;
            return;
        }
        jd_fail(d);
        return;
    }
}

static void jd_parse_array(JsonDocBuf *d) {
    if (*d->p != '[') {
        jd_fail(d);
        return;
    }
    jd_emit(d, '[');
    d->p++;
    jd_skip_ws(d);
    if (*d->p == ']') {
        jd_emit(d, ']');
        d->p++;
        return;
    }
    for (;;) {
        jd_skip_ws(d);
        jd_parse_value(d);
        if (!d->ok) {
            return;
        }
        jd_skip_ws(d);
        if (*d->p == ',') {
            jd_emit(d, ',');
            d->p++;
            continue;
        }
        if (*d->p == ']') {
            jd_emit(d, ']');
            d->p++;
            return;
        }
        jd_fail(d);
        return;
    }
}

static void jd_parse_value(JsonDocBuf *d) {
    jd_skip_ws(d);
    if (*d->p == '"') {
        jd_parse_string(d);
        return;
    }
    if (*d->p == '{') {
        jd_parse_object(d);
        return;
    }
    if (*d->p == '[') {
        jd_parse_array(d);
        return;
    }
    if (*d->p == '-' || (*d->p >= '0' && *d->p <= '9')) {
        jd_parse_number(d);
        return;
    }
    if (strncmp(d->p, "true", 4) == 0) {
        jd_emit_n(d, "true", 4);
        d->p += 4;
        return;
    }
    if (strncmp(d->p, "false", 5) == 0) {
        jd_emit_n(d, "false", 5);
        d->p += 5;
        return;
    }
    if (strncmp(d->p, "null", 4) == 0) {
        jd_emit_n(d, "null", 4);
        d->p += 4;
        return;
    }
    jd_fail(d);
}

/* Validate + compact JSON document. Empty string on invalid input. */
char *json_parse_document(const char *input) {
    if (!input) {
        return dup_slice("", 0);
    }
    JsonDocBuf d;
    d.p = input;
    d.out = NULL;
    d.len = 0;
    d.cap = 0;
    d.ok = 1;
    jd_skip_ws(&d);
    if (!*d.p) {
        return dup_slice("", 0);
    }
    jd_parse_value(&d);
    if (d.ok) {
        jd_skip_ws(&d);
        if (*d.p) {
            jd_fail(&d);
        }
    }
    if (!d.ok) {
        free(d.out);
        return dup_slice("", 0);
    }
    jd_emit(&d, '\0');
    if (!d.ok) {
        free(d.out);
        return dup_slice("", 0);
    }
    return d.out;
}

/* Compact stringify — same semantics as parse for document text. */
char *json_stringify_document(const char *input) {
    return json_parse_document(input);
}
