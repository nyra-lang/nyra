#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int str_len(const char *s) {
    if (!s) {
        return 0;
    }
    return (int)strlen(s);
}

char *str_cat(const char *a, const char *b) {
    size_t la = strlen(a);
    size_t lb = strlen(b);
    char *out = (char *)malloc(la + lb + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, a, la);
    memcpy(out + la, b, lb + 1);
    return out;
}

char *i32_to_string(int n) {
    char buf[32];
    int len = snprintf(buf, sizeof(buf), "%d", n);
    if (len < 0) {
        return NULL;
    }
    char *out = (char *)malloc((size_t)len + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, buf, (size_t)len + 1);
    return out;
}

char *i64_to_string(long long n) {
    char buf[32];
    int len = snprintf(buf, sizeof(buf), "%lld", n);
    if (len < 0) {
        return NULL;
    }
    char *out = (char *)malloc((size_t)len + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, buf, (size_t)len + 1);
    return out;
}

int str_cmp(const char *a, const char *b) {
    return strcmp(a, b);
}

int char_at(const char *s, int i) {
    if (!s || i < 0) {
        return 0;
    }
    size_t len = strlen(s);
    if ((size_t)i >= len) {
        return 0;
    }
    return (unsigned char)s[i];
}

char *substring(const char *s, int start, int len) {
    if (!s || start < 0 || len < 0) {
        return NULL;
    }
    size_t slen = strlen(s);
    if ((size_t)start >= slen) {
        char *empty = (char *)malloc(1);
        if (empty) {
            empty[0] = '\0';
        }
        return empty;
    }
    if ((size_t)(start + len) > slen) {
        len = (int)(slen - (size_t)start);
    }
    char *out = (char *)malloc((size_t)len + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, s + start, (size_t)len);
    out[len] = '\0';
    return out;
}

int strstr_pos(const char *hay, const char *needle) {
    if (!hay || !needle) {
        return -1;
    }
    const char *p = strstr(hay, needle);
    if (!p) {
        return -1;
    }
    return (int)(p - hay);
}

static char *nyra_str_transform(const char *s, int to_upper) {
    if (!s) {
        return NULL;
    }
    size_t len = strlen(s);
    char *out = (char *)malloc(len + 1);
    if (!out) {
        return NULL;
    }
    for (size_t i = 0; i < len; i++) {
        unsigned char c = (unsigned char)s[i];
        if (to_upper && c >= 'a' && c <= 'z') {
            out[i] = (char)(c - ('a' - 'A'));
        } else if (!to_upper && c >= 'A' && c <= 'Z') {
            out[i] = (char)(c + ('a' - 'A'));
        } else {
            out[i] = (char)c;
        }
    }
    out[len] = '\0';
    return out;
}

char *str_to_upper(const char *s) {
    return nyra_str_transform(s, 1);
}

char *str_to_lower(const char *s) {
    return nyra_str_transform(s, 0);
}

char *str_trim(const char *s) {
    if (!s) {
        return NULL;
    }
    while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r') {
        s++;
    }
    size_t len = strlen(s);
    while (len > 0) {
        char c = s[len - 1];
        if (c != ' ' && c != '\t' && c != '\n' && c != '\r') {
            break;
        }
        len--;
    }
    char *out = (char *)malloc(len + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, s, len);
    out[len] = '\0';
    return out;
}

int str_contains(const char *hay, const char *needle) {
    return strstr_pos(hay, needle) >= 0 ? 1 : 0;
}

int str_starts_with(const char *s, const char *prefix) {
    if (!s || !prefix) {
        return 0;
    }
    size_t plen = strlen(prefix);
    if (plen == 0) {
        return 1;
    }
    return strncmp(s, prefix, plen) == 0 ? 1 : 0;
}

int str_ends_with(const char *s, const char *suffix) {
    if (!s || !suffix) {
        return 0;
    }
    size_t slen = strlen(s);
    size_t suflen = strlen(suffix);
    if (suflen > slen) {
        return 0;
    }
    return strcmp(s + slen - suflen, suffix) == 0 ? 1 : 0;
}

char *str_dup(const char *s) {
    if (!s) {
        return NULL;
    }
    size_t n = strlen(s);
    char *copy = (char *)malloc(n + 1);
    if (!copy) {
        return NULL;
    }
    memcpy(copy, s, n + 1);
    return copy;
}

char *str_replacen(const char *s, const char *from, const char *to, int count) {
    if (!s || !from || !to) {
        return NULL;
    }
    if (count == 0) {
        return str_dup(s);
    }
    size_t flen = strlen(from);
    if (flen == 0) {
        return str_dup(s);
    }
    size_t tlen = strlen(to);
    size_t slen = strlen(s);
    int max = count < 0 ? INT_MAX : count;

    size_t n = 0;
    const char *scan = s;
    while (n < (size_t)max && (scan = strstr(scan, from)) != NULL) {
        n++;
        scan += flen;
    }
    if (n == 0) {
        return str_dup(s);
    }

    size_t out_len = slen + n * (tlen - flen);
    char *out = (char *)malloc(out_len + 1);
    if (!out) {
        return NULL;
    }

    const char *src = s;
    char *dst = out;
    size_t replaced = 0;
    while (replaced < n) {
        const char *found = strstr(src, from);
        if (!found) {
            size_t rest = strlen(src);
            memcpy(dst, src, rest + 1);
            return out;
        }
        size_t prefix_len = (size_t)(found - src);
        memcpy(dst, src, prefix_len);
        dst += prefix_len;
        memcpy(dst, to, tlen);
        dst += tlen;
        src = found + flen;
        replaced++;
    }
    size_t rest = strlen(src);
    memcpy(dst, src, rest + 1);
    return out;
}

char *str_replace(const char *s, const char *from, const char *to) {
    return str_replacen(s, from, to, -1);
}

void *vec_str_new(void);
void vec_str_push(void *handle, const char *value);

int str_to_i32(const char *s) {
    if (!s || s[0] == '\0') {
        return 0;
    }
    return (int)strtol(s, NULL, 10);
}

double str_to_f64(const char *s) {
    if (!s || s[0] == '\0') {
        return 0.0;
    }
    return strtod(s, NULL);
}

char *f64_to_string(double n) {
    char buf[64];
    int len = snprintf(buf, sizeof(buf), "%.17g", n);
    if (len < 0) {
        return NULL;
    }
    char *out = (char *)malloc((size_t)len + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, buf, (size_t)len + 1);
    return out;
}

char *str_push_char(const char *s, int ch) {
    size_t len = s ? strlen(s) : 0;
    char *out = (char *)malloc(len + 2);
    if (!out) {
        return NULL;
    }
    if (len > 0) {
        memcpy(out, s, len);
    }
    out[len] = (char)ch;
    out[len + 1] = '\0';
    return out;
}

char *str_pop(const char *s) {
    if (!s || s[0] == '\0') {
        char *empty = (char *)malloc(1);
        if (empty) {
            empty[0] = '\0';
        }
        return empty;
    }
    size_t len = strlen(s);
    char *out = (char *)malloc(len);
    if (!out) {
        return NULL;
    }
    if (len == 1) {
        out[0] = '\0';
        return out;
    }
    memcpy(out, s, len - 1);
    out[len - 1] = '\0';
    return out;
}

char *str_strip_ansi(const char *input) {
    if (!input) {
        char *empty = (char *)malloc(1);
        if (empty) {
            empty[0] = '\0';
        }
        return empty;
    }
    size_t in_len = strlen(input);
    char *out = (char *)malloc(in_len + 1);
    if (!out) {
        return NULL;
    }
    size_t j = 0;
    for (size_t i = 0; i < in_len; i++) {
        unsigned char c = (unsigned char)input[i];
        if (c == 0x1b && i + 1 < in_len && input[i + 1] == '[') {
            i += 2;
            while (i < in_len && input[i] != 'm' && input[i] != 'H' && input[i] != 'J' && input[i] != 'K') {
                i++;
            }
            continue;
        }
        if (c == '\r') {
            continue;
        }
        if (c >= 32 || c == '\n' || c == '\t') {
            out[j++] = (char)c;
        }
    }
    out[j] = '\0';
    return out;
}

void *str_split(const char *s, const char *sep) {
    void *vec = vec_str_new();
    if (!vec) {
        return NULL;
    }
    if (!s) {
        vec_str_push(vec, "");
        return vec;
    }
    if (!sep || sep[0] == '\0') {
        vec_str_push(vec, s);
        return vec;
    }
    size_t seplen = strlen(sep);
    const char *start = s;
    const char *p = s;
    while (*p) {
        if (strncmp(p, sep, seplen) == 0) {
            size_t chunk = (size_t)(p - start);
            char *part = (char *)malloc(chunk + 1);
            if (!part) {
                break;
            }
            memcpy(part, start, chunk);
            part[chunk] = '\0';
            vec_str_push(vec, part);
            free(part);
            p += seplen;
            start = p;
        } else {
            p++;
        }
    }
    vec_str_push(vec, start);
    return vec;
}

// [builtin-dev:strip_suffix:string]
char *str_strip_suffix(const char *s, const char *suffix) {
    if (!s) return NULL;
    if (!suffix) return str_dup(s);

    size_t slen = strlen(s);
    size_t suflen = strlen(suffix);

    if (suflen > slen) {
        return str_dup(s);
    }

    const char *end_ptr = s + (slen - suflen);
    if (strcmp(end_ptr, suffix) == 0) {
        size_t new_len = slen - suflen;
        char *out = (char *)malloc(new_len + 1);
        if (!out) return NULL;
        
        memcpy(out, s, new_len);
        out[new_len] = '\0';
        return out;
    }

    return str_dup(s);
}
// [/builtin-dev:strip_suffix:string]
