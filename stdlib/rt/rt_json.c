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
