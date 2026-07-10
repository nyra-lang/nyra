#include <ctype.h>
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
// [builtin-dev:to_snake_case:string]
char *str_to_snake_case(const char *s) {
    if (s == NULL) return NULL;

    size_t len = strlen(s);
    // Allocate double the size to accommodate inserted underscores (CamelCase).
    char *result = (char *)malloc(len * 2 + 1);
    if (!result) return NULL;

    int i = 0, j = 0;
    while (s[i] != '\0') {
        if (isspace((unsigned char)s[i])) {
            if (j > 0 && result[j - 1] != '_') {
                result[j++] = '_';
            }
        } else if (isupper((unsigned char)s[i])) {
            if (j > 0 && result[j - 1] != '_') {
                result[j++] = '_';
            }
            result[j++] = (char)tolower((unsigned char)s[i]);
        } else {
            result[j++] = s[i];
        }
        i++;
    }

    result[j] = '\0';
    return result;
}
// [/builtin-dev:to_snake_case:string]


/* ---- shared helpers for case-conversion builtins (not method-specific) ---- */

/* Split `s` into lowercased words. Boundaries are runs of non-alphanumeric
 * characters (space, _, -, ., …) and lower/digit → upper transitions so that
 * "fooBar" splits into ["foo", "bar"]. Returns a malloc'd array of `*count`
 * malloc'd strings; free with nyra_free_words. */
static char **nyra_split_words(const char *s, int *count) {
    *count = 0;
    if (!s) {
        return NULL;
    }
    size_t len = strlen(s);
    char **words = (char **)malloc((len + 1) * sizeof(char *));
    char *buf = (char *)malloc(len + 1);
    if (!words || !buf) {
        free(words);
        free(buf);
        return NULL;
    }
    int n = 0;
    size_t bi = 0;
    for (size_t i = 0; i < len; i++) {
        unsigned char c = (unsigned char)s[i];
        if (!isalnum(c)) {
            if (bi > 0) {
                buf[bi] = '\0';
                words[n++] = str_dup(buf);
                bi = 0;
            }
            continue;
        }
        if (bi > 0 && isupper(c)) {
            unsigned char prev = (unsigned char)s[i - 1];
            if (islower(prev) || isdigit(prev)) {
                buf[bi] = '\0';
                words[n++] = str_dup(buf);
                bi = 0;
            }
        }
        buf[bi++] = (char)tolower(c);
    }
    if (bi > 0) {
        buf[bi] = '\0';
        words[n++] = str_dup(buf);
    }
    free(buf);
    *count = n;
    return words;
}

static void nyra_free_words(char **words, int count) {
    if (!words) {
        return;
    }
    for (int i = 0; i < count; i++) {
        free(words[i]);
    }
    free(words);
}

/* Join lowercased words. sep = 0 means no separator. cap mode:
 *   0 = keep lower, 1 = UPPER, 2 = Capitalize each, 3 = camel (first word
 *   lower, later words capitalized). */
static char *nyra_join_words(char **words, int count, char sep, int cap) {
    size_t total = 1;
    for (int i = 0; i < count; i++) {
        total += strlen(words[i]);
    }
    if (sep && count > 1) {
        total += (size_t)(count - 1);
    }
    char *out = (char *)malloc(total);
    if (!out) {
        return NULL;
    }
    size_t oi = 0;
    for (int i = 0; i < count; i++) {
        if (sep && i > 0) {
            out[oi++] = sep;
        }
        const char *w = words[i];
        for (size_t k = 0; w[k]; k++) {
            unsigned char ch = (unsigned char)w[k];
            if (cap == 1) {
                ch = (unsigned char)toupper(ch);
            } else if (cap == 2 && k == 0) {
                ch = (unsigned char)toupper(ch);
            } else if (cap == 3 && i > 0 && k == 0) {
                ch = (unsigned char)toupper(ch);
            }
            out[oi++] = (char)ch;
        }
    }
    out[oi] = '\0';
    return out;
}

/* Tokenize `s` then re-join with the given separator + capitalization mode. */
static char *nyra_case_join(const char *s, char sep, int cap) {
    if (!s) {
        return NULL;
    }
    int count = 0;
    char **words = nyra_split_words(s, &count);
    char *out = nyra_join_words(words, count, sep, cap);
    nyra_free_words(words, count);
    return out ? out : str_dup("");
}


// [builtin-dev:to_lowercase:string]
char *str_to_lowercase(const char *s) {
    if (!s) {
        return NULL;
    }
    char *out = str_dup(s);
    if (!out) {
        return NULL;
    }
    for (size_t i = 0; out[i]; i++) {
        out[i] = (char)tolower((unsigned char)out[i]);
    }
    return out;
}
// [/builtin-dev:to_lowercase:string]


// [builtin-dev:to_titlecase:string]
char *str_to_titlecase(const char *s) {
    if (!s) {
        return NULL;
    }
    char *out = str_dup(s);
    if (!out) {
        return NULL;
    }
    int at_word_start = 1;
    for (size_t i = 0; out[i]; i++) {
        unsigned char c = (unsigned char)out[i];
        if (isalpha(c)) {
            out[i] = (char)(at_word_start ? toupper(c) : tolower(c));
            at_word_start = 0;
        } else {
            at_word_start = 1;
        }
    }
    return out;
}
// [/builtin-dev:to_titlecase:string]


// [builtin-dev:to_capitalize:string]
char *str_to_capitalize(const char *s) {
    if (!s) {
        return NULL;
    }
    char *out = str_dup(s);
    if (!out) {
        return NULL;
    }
    int first_alpha = 1;
    for (size_t i = 0; out[i]; i++) {
        unsigned char c = (unsigned char)out[i];
        if (first_alpha && isalpha(c)) {
            out[i] = (char)toupper(c);
            first_alpha = 0;
        } else {
            out[i] = (char)tolower(c);
        }
    }
    return out;
}
// [/builtin-dev:to_capitalize:string]


// [builtin-dev:to_camel_case:string]
char *str_to_camel_case(const char *s) {
    return nyra_case_join(s, 0, 3);
}
// [/builtin-dev:to_camel_case:string]


// [builtin-dev:to_kebab_case:string]
char *str_to_kebab_case(const char *s) {
    return nyra_case_join(s, '-', 0);
}
// [/builtin-dev:to_kebab_case:string]


// [builtin-dev:to_pascal_case:string]
char *str_to_pascal_case(const char *s) {
    return nyra_case_join(s, 0, 2);
}
// [/builtin-dev:to_pascal_case:string]


// [builtin-dev:to_screaming_snake_case:string]
char *str_to_screaming_snake_case(const char *s) {
    return nyra_case_join(s, '_', 1);
}
// [/builtin-dev:to_screaming_snake_case:string]


// [builtin-dev:to_train_case:string]
char *str_to_train_case(const char *s) {
    return nyra_case_join(s, '-', 2);
}
// [/builtin-dev:to_train_case:string]


// [builtin-dev:to_dot_case:string]
char *str_to_dot_case(const char *s) {
    return nyra_case_join(s, '.', 0);
}
// [/builtin-dev:to_dot_case:string]




// [builtin-dev:strip_prefix:string]
char *str_strip_prefix(const char *s, const char *prefix) {
    if (!s) return NULL;
    if (!prefix || prefix[0] == '\0') return str_dup(s);
    size_t plen = strlen(prefix);
    size_t slen = strlen(s);
    if (plen > slen) return str_dup(s);
    if (strncmp(s, prefix, plen) != 0) return str_dup(s);
    return str_dup(s + plen);
}
// [/builtin-dev:strip_prefix:string]


// [builtin-dev:index:string]
int str_index(const char *s, const char *needle) {
    if (!s || !needle) return -1;
    int pos = strstr_pos(s, needle);
    return pos;
}
// [/builtin-dev:index:string]


// [builtin-dev:is_empty:string]
int str_is_empty(const char *s) {
    if (!s) return 1;
    return s[0] == '\0' ? 1 : 0;
}
// [/builtin-dev:is_empty:string]


// [builtin-dev:last_index:string]
int str_last_index(const char *s, const char *needle) {
    if (!s || !needle || needle[0] == '\0') return -1;
    size_t nlen = strlen(needle);
    size_t slen = strlen(s);
    if (nlen > slen) return -1;
    for (size_t i = slen - nlen; i != (size_t)-1; i--) {
        if (strncmp(s + i, needle, nlen) == 0) {
            return (int)i;
        }
        if (i == 0) break;
    }
    return -1;
}
// [/builtin-dev:last_index:string]


// [builtin-dev:repeat:string]
char *str_repeat(const char *s, int count) {
    if (!s || count <= 0) return str_dup("");
    size_t slen = strlen(s);
    size_t total = (size_t)count * slen;
    char *out = (char *)malloc(total + 1);
    if (!out) return NULL;
    char *p = out;
    for (int i = 0; i < count; i++) {
        memcpy(p, s, slen);
        p += slen;
    }
    out[total] = '\0';
    return out;
}
// [/builtin-dev:repeat:string]


// [builtin-dev:trim_end:string]
char *str_trim_end(const char *s) {
    if (!s) return NULL;
    size_t len = strlen(s);
    while (len > 0) {
        char c = s[len - 1];
        if (c != ' ' && c != '\t' && c != '\n' && c != '\r') break;
        len--;
    }
    char *out = (char *)malloc(len + 1);
    if (!out) return NULL;
    memcpy(out, s, len);
    out[len] = '\0';
    return out;
}
// [/builtin-dev:trim_end:string]


// [builtin-dev:trim_start:string]
char *str_trim_start(const char *s) {
    if (!s) return NULL;
    while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r') s++;
    return str_dup(s);
}
// [/builtin-dev:trim_start:string]


// [builtin-dev:splitn:string]
void *str_splitn(const char *s, const char *sep, int n) {
    void *vec = vec_str_new();
    if (!vec) {
        return NULL;
    }
    if (!s) {
        vec_str_push(vec, "");
        return vec;
    }
    if (n <= 0) {
        vec_str_push(vec, s);
        return vec;
    }
    if (!sep || sep[0] == '\0') {
        vec_str_push(vec, s);
        return vec;
    }
    size_t seplen = strlen(sep);
    const char *start = s;
    const char *p = s;
    int parts = 1;
    while (*p && parts < n) {
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
            parts++;
        } else {
            p++;
        }
    }
    vec_str_push(vec, start);
    return vec;
}
// [/builtin-dev:splitn:string]


// [builtin-dev:count:string]
int str_count(const char *s, const char *needle) {
    if (!s || !needle || needle[0] == '\0') {
        return 0;
    }
    size_t nlen = strlen(needle);
    int count = 0;
    const char *p = s;
    while (*p) {
        if (strncmp(p, needle, nlen) == 0) {
            count++;
            p += nlen;
        } else {
            p++;
        }
    }
    return count;
}
// [/builtin-dev:count:string]


// [builtin-dev:fields:string]
static int str_is_field_sep(char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r';
}

void *str_fields(const char *s) {
    void *vec = vec_str_new();
    if (!vec) {
        return NULL;
    }
    if (!s) {
        return vec;
    }
    const char *p = s;
    while (*p) {
        while (*p && str_is_field_sep(*p)) {
            p++;
        }
        if (!*p) {
            break;
        }
        const char *start = p;
        while (*p && !str_is_field_sep(*p)) {
            p++;
        }
        size_t chunk = (size_t)(p - start);
        char *part = (char *)malloc(chunk + 1);
        if (!part) {
            break;
        }
        memcpy(part, start, chunk);
        part[chunk] = '\0';
        vec_str_push(vec, part);
        free(part);
    }
    return vec;
}
// [/builtin-dev:fields:string]


// [builtin-dev:pad_end:string]
char *str_pad_end(const char *s, int width, const char *pad) {
    if (!s) {
        return NULL;
    }
    size_t slen = strlen(s);
    if (width <= (int)slen) {
        return str_dup(s);
    }
    const char *p = (pad && pad[0] != '\0') ? pad : " ";
    size_t plen = strlen(p);
    size_t need = (size_t)width - slen;
    char *out = (char *)malloc((size_t)width + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, s, slen);
    size_t oi = slen;
    size_t pi = 0;
    while (oi < (size_t)width) {
        out[oi++] = p[pi++];
        if (pi >= plen) {
            pi = 0;
        }
    }
    out[width] = '\0';
    return out;
}
// [/builtin-dev:pad_end:string]


// [builtin-dev:pad_start:string]
char *str_pad_start(const char *s, int width, const char *pad) {
    if (!s) {
        return NULL;
    }
    size_t slen = strlen(s);
    if (width <= (int)slen) {
        return str_dup(s);
    }
    const char *p = (pad && pad[0] != '\0') ? pad : " ";
    size_t plen = strlen(p);
    size_t need = (size_t)width - slen;
    char *out = (char *)malloc((size_t)width + 1);
    if (!out) {
        return NULL;
    }
    size_t oi = 0;
    size_t pi = 0;
    while (oi < need) {
        out[oi++] = p[pi++];
        if (pi >= plen) {
            pi = 0;
        }
    }
    memcpy(out + need, s, slen + 1);
    return out;
}
// [/builtin-dev:pad_start:string]


// [builtin-dev:split_once:string]
char *str_before_sep(const char *s, const char *sep) {
    if (!s) {
        return NULL;
    }
    if (!sep || sep[0] == '\0') {
        return str_dup(s);
    }
    const char *p = strstr(s, sep);
    if (!p) {
        return str_dup(s);
    }
    size_t len = (size_t)(p - s);
    char *out = (char *)malloc(len + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, s, len);
    out[len] = '\0';
    return out;
}
// [/builtin-dev:split_once:string]

// [contrib-dev:hex_decode:encoding_mod]
static int hex_nibble(char c) {
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return c - 'a' + 10;
    }
    if (c >= 'A' && c <= 'F') {
        return c - 'A' + 10;
    }
    return -1;
}

char *hex_decode(const char *hex) {
    if (!hex) {
        return str_dup("");
    }
    size_t i = 0;
    if (hex[0] == '0' && (hex[1] == 'x' || hex[1] == 'X')) {
        i = 2;
    }
    size_t cap = strlen(hex) / 2 + 1;
    char *out = (char *)malloc(cap);
    if (!out) {
        return NULL;
    }
    size_t oi = 0;
    int hi = -1;
    for (; hex[i] != '\0'; i++) {
        if (hex[i] == ' ' || hex[i] == '\t') {
            continue;
        }
        int n = hex_nibble(hex[i]);
        if (n < 0) {
            out[oi] = '\0';
            return out;
        }
        if (hi < 0) {
            hi = n;
        } else {
            out[oi++] = (char)((hi << 4) | n);
            hi = -1;
        }
    }
    out[oi] = '\0';
    return out;
}
// [/contrib-dev:hex_decode:encoding_mod]

// [contrib-dev:str_to_bool:strconv_mod]
int str_to_bool(const char *s) {
    if (!s) {
        return 0;
    }
    if (strcmp(s, "true") == 0 || strcmp(s, "1") == 0 || strcmp(s, "yes") == 0) {
        return 1;
    }
    return 0;
}
// [/contrib-dev:str_to_bool:strconv_mod]


// [builtin-dev:compare:string]
int str_compare(const char *s, const char * other) {
    if (!s && !other) return 0;
    if (!s) return -1;
    if (!other) return 1;
    return strcmp(s, other);
}
// [/builtin-dev:compare:string]


// [builtin-dev:equal_fold:string]
int str_equal_fold(const char *s, const char * other) {
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
    return (s[i] == '\0' && other[i] == '\0') ? 1 : 0;
}
// [/builtin-dev:equal_fold:string]


// [builtin-dev:index_byte:string]
int str_index_byte(const char *s, int byte) {
    if (!s) return -1;
    unsigned char ch = (unsigned char)byte;
    for (int i = 0; s[i]; i++) {
        if ((unsigned char)s[i] == ch) return i;
    }
    return -1;
}
// [/builtin-dev:index_byte:string]


// [builtin-dev:last_index_byte:string]
int str_last_index_byte(const char *s, int byte) {
    if (!s) return -1;
    unsigned char ch = (unsigned char)byte;
    int last = -1;
    for (int i = 0; s[i]; i++) {
        if ((unsigned char)s[i] == ch) last = i;
    }
    return last;
}
// [/builtin-dev:last_index_byte:string]

// [contrib-dev:f64_to_string_prec:strconv_mod]
char * f64_to_string_prec(double n, int prec) {
    extern char *str_dup(const char *s);
    char buf[64];
    if (prec < 0) prec = 0;
    if (prec > 12) prec = 12;
    snprintf(buf, sizeof(buf), "%.*f", prec, n);
    return str_dup(buf);
}
// [/contrib-dev:f64_to_string_prec:strconv_mod]

// [contrib-dev:parse_int_base:strconv_mod]
int parse_int_base(const char * s, int base) {
    if (!s) return 0;
    return (int)strtol(s, NULL, base);
}
// [/contrib-dev:parse_int_base:strconv_mod]

// [contrib-dev:str_to_i64:strconv_mod]
long long str_to_i64(const char * s) {
    if (!s) return 0;
    return (long long)strtoll(s, NULL, 10);
}
// [/contrib-dev:str_to_i64:strconv_mod]

// [contrib-dev:format_i32_pad:strconv_mod]
char * format_i32_pad(int n, int width) {
    extern char *str_dup(const char *s);
    char buf[32];
    if (width < 0) width = 0;
    if (width > 20) width = 20;
    snprintf(buf, sizeof(buf), "%0*d", width, n);
    return str_dup(buf);
}
// [/contrib-dev:format_i32_pad:strconv_mod]


// [builtin-dev:after_sep:string]
char * str_after_sep(const char *s, const char * sep) {
    if (!s) return str_dup("");
    if (!sep || sep[0] == '\0') return str_dup(s);
    const char *p = strstr(s, sep);
    if (!p) return str_dup("");
    return str_dup(p + strlen(sep));
}
// [/builtin-dev:after_sep:string]

// [contrib-dev:format_i32_hex:strconv_mod]
char * format_i32_hex(int n) {
    extern char *str_dup(const char *s);
    char buf[16];
    snprintf(buf, sizeof(buf), "%x", n);
    return str_dup(buf);
}
// [/contrib-dev:format_i32_hex:strconv_mod]

// [contrib-dev:parse_uint_base:strconv_mod]
int parse_uint_base(const char * s, int base) {
    if (!s) return 0;
    return (int)strtoul(s, NULL, base);
}
// [/contrib-dev:parse_uint_base:strconv_mod]

// [contrib-dev:str_to_u64:strconv_mod]
long long str_to_u64(const char * s) {
    if (!s) return 0;
    return (long long)strtoull(s, NULL, 10);
}
// [/contrib-dev:str_to_u64:strconv_mod]

// [contrib-dev:format_bool:strconv_mod]
char * format_bool(int b) {
    extern char *str_dup(const char *s);
    return str_dup(b ? "true" : "false");
}
// [/contrib-dev:format_bool:strconv_mod]

// [contrib-dev:format_f64_pad:strconv_mod]
char * format_f64_pad(double n, int width, int prec) {
    extern char *str_dup(const char *s);
    char buf[64];
    if (width < 0) width = 0;
    if (prec < 0) prec = 0;
    if (prec > 12) prec = 12;
    snprintf(buf, sizeof(buf), "%*.*f", width, prec, n);
    return str_dup(buf);
}
// [/contrib-dev:format_f64_pad:strconv_mod]

// [contrib-dev:format_i32_hex_pad:strconv_mod]
char * format_i32_hex_pad(int n, int width) {
    extern char *str_dup(const char *s);
    char buf[24];
    if (width < 0) width = 0;
    if (width > 16) width = 16;
    snprintf(buf, sizeof(buf), "%0*x", width, n);
    return str_dup(buf);
}
// [/contrib-dev:format_i32_hex_pad:strconv_mod]

// [contrib-dev:format_i64_pad:strconv_mod]
char * format_i64_pad(long long n, int width) {
    extern char *str_dup(const char *s);
    char buf[48];
    if (width < 0) width = 0;
    if (width > 24) width = 24;
    snprintf(buf, sizeof(buf), "%0*lld", width, n);
    return str_dup(buf);
}
// [/contrib-dev:format_i64_pad:strconv_mod]


// [builtin-dev:collapse_ws:string]
char * str_collapse_ws(const char *s) {
    extern char *str_dup(const char *s);
    if (!s) return str_dup("");
    size_t cap = strlen(s) + 1;
    char *out = (char *)malloc(cap);
    if (!out) return NULL;
    size_t oi = 0;
    int in_ws = 0;
    for (size_t i = 0; s[i]; i++) {
        char c = s[i];
        if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
            if (!in_ws && oi > 0) { out[oi++] = ' '; in_ws = 1; }
        } else {
            out[oi++] = c;
            in_ws = 0;
        }
    }
    while (oi > 0 && out[oi - 1] == ' ') oi--;
    out[oi] = '\0';
    return out;
}
// [/builtin-dev:collapse_ws:string]


// [builtin-dev:is_ascii:string]
int str_is_ascii(const char *s) {
    if (!s) return 1;
    for (int i = 0; s[i]; i++) {
        if ((unsigned char)s[i] > 127) return 0;
    }
    return 1;
}
// [/builtin-dev:is_ascii:string]

// [contrib-dev:hex_encode:encoding_mod]
char * hex_encode(const char * data) {
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
    out[n * 2] = '\0';
    return out;
}
// [/contrib-dev:hex_encode:encoding_mod]

// [contrib-dev:hex_encode_upper:encoding_mod]
char * hex_encode_upper(const char * data) {
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
    out[n * 2] = '\0';
    return out;
}
// [/contrib-dev:hex_encode_upper:encoding_mod]

// [contrib-dev:i32_to_string_radix:strconv_mod]
char * i32_to_string_radix(int n, int base) {
    extern char *str_dup(const char *s);
    char buf[34];
    if (base < 2) base = 10;
    if (base > 36) base = 36;
    if (base == 10) snprintf(buf, sizeof(buf), "%d", n);
    else snprintf(buf, sizeof(buf), "%x", n);
    return str_dup(buf);
}
// [/contrib-dev:i32_to_string_radix:strconv_mod]

// [contrib-dev:parse_i64_base:strconv_mod]
long long parse_i64_base(const char * s, int base) {
    if (!s) return 0;
    return (long long)strtoll(s, NULL, base);
}
// [/contrib-dev:parse_i64_base:strconv_mod]

// [contrib-dev:str_to_f32:strconv_mod]
double str_to_f32(const char * s) {
    if (!s) return 0.0;
    return (double)strtof(s, NULL);
}
// [/contrib-dev:str_to_f32:strconv_mod]

// [contrib-dev:u64_to_string:strconv_mod]
char * u64_to_string(long long n) {
    extern char *str_dup(const char *s);
    char buf[32];
    snprintf(buf, sizeof(buf), "%llu", (unsigned long long)n);
    return str_dup(buf);
}
// [/contrib-dev:u64_to_string:strconv_mod]

// [contrib-dev:format_i32_bin:strconv_mod]
char * format_i32_bin(int n) {
    extern char *str_dup(const char *s);
    char tmp[33];
    int pos = 32;
    unsigned u = (unsigned)n;
    tmp[pos] = '\0';
    if (u == 0) return str_dup("0");
    while (u > 0 && pos > 0) {
        tmp[--pos] = (char)('0' + (u & 1));
        u >>= 1;
    }
    return str_dup(tmp + pos);
}
// [/contrib-dev:format_i32_bin:strconv_mod]

// [contrib-dev:format_i32_oct:strconv_mod]
char * format_i32_oct(int n) {
    extern char *str_dup(const char *s);
    char buf[16];
    snprintf(buf, sizeof(buf), "%o", n);
    return str_dup(buf);
}
// [/contrib-dev:format_i32_oct:strconv_mod]

// [contrib-dev:format_i64_hex:strconv_mod]
char * format_i64_hex(long long n) {
    extern char *str_dup(const char *s);
    char buf[24];
    snprintf(buf, sizeof(buf), "%llx", (long long)n);
    return str_dup(buf);
}
// [/contrib-dev:format_i64_hex:strconv_mod]

// [contrib-dev:format_u64_pad:strconv_mod]
char * format_u64_pad(long long n, int width) {
    extern char *str_dup(const char *s);
    char buf[48];
    if (width < 0) width = 0;
    if (width > 24) width = 24;
    snprintf(buf, sizeof(buf), "%0*llu", width, (unsigned long long)n);
    return str_dup(buf);
}
// [/contrib-dev:format_u64_pad:strconv_mod]


// [builtin-dev:common_prefix_len:string]
int str_common_prefix_len(const char *s, const char * other) {
    if (!s || !other) return 0;
    int i = 0;
    while (s[i] && other[i] && s[i] == other[i]) i++;
    return i;
}
// [/builtin-dev:common_prefix_len:string]


// [builtin-dev:is_alnum:string]
int str_is_alnum(const char *s) {
    if (!s || s[0] == '\0') return 0;
    for (int i = 0; s[i]; i++) {
        unsigned char c = (unsigned char)s[i];
        if (!((c >= '0' && c <= '9') || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')))
            return 0;
    }
    return 1;
}
// [/builtin-dev:is_alnum:string]


// [builtin-dev:is_alpha:string]
int str_is_alpha(const char *s) {
    if (!s || s[0] == '\0') return 0;
    for (int i = 0; s[i]; i++) {
        unsigned char c = (unsigned char)s[i];
        if (!((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z'))) return 0;
    }
    return 1;
}
// [/builtin-dev:is_alpha:string]


// [builtin-dev:is_digit:string]
int str_is_digit(const char *s) {
    if (!s || s[0] == '\0') return 0;
    for (int i = 0; s[i]; i++) {
        if (s[i] < '0' || s[i] > '9') return 0;
    }
    return 1;
}
// [/builtin-dev:is_digit:string]


// [builtin-dev:pad_center:string]
char * str_pad_center(const char *s, int width, const char * pad) {
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
    tmp[oi] = '\0';
    return tmp;
}
// [/builtin-dev:pad_center:string]


// [builtin-dev:reverse:string]
char * str_reverse(const char *s) {
    extern char *str_dup(const char *s);
    if (!s) return str_dup("");
    size_t n = strlen(s);
    char *out = (char *)malloc(n + 1);
    if (!out) return NULL;
    for (size_t i = 0; i < n; i++) out[i] = s[n - 1 - i];
    out[n] = '\0';
    return out;
}
// [/builtin-dev:reverse:string]

// [contrib-dev:format_i64_bin:strconv_mod]
char * format_i64_bin(long long n) {
    extern char *str_dup(const char *s);
    char tmp[65];
    int pos = 64;
    unsigned long long u = (unsigned long long)n;
    tmp[pos] = '\0';
    if (u == 0) return str_dup("0");
    while (u > 0 && pos > 0) {
        tmp[--pos] = (char)('0' + (u & 1));
        u >>= 1;
    }
    return str_dup(tmp + pos);
}
// [/contrib-dev:format_i64_bin:strconv_mod]

// [contrib-dev:format_quote:strconv_mod]
char * format_quote(const char * s) {
    extern char *str_dup(const char *s);
    if (!s) return str_dup("\"\"");
    size_t n = strlen(s);
    char *out = (char *)malloc(n * 2 + 3);
    if (!out) return NULL;
    int oi = 0;
    out[oi++] = '"';
    for (size_t i = 0; i < n; i++) {
        char c = s[i];
        if (c == '"' || c == '\\') out[oi++] = '\\';
        out[oi++] = c;
    }
    out[oi++] = '"';
    out[oi] = '\0';
    return out;
}
// [/contrib-dev:format_quote:strconv_mod]


// [builtin-dev:escape_json:string]
char * str_escape_json(const char *s) {
    extern char *str_dup(const char *s);
    if (!s) return str_dup("");
    size_t n = strlen(s);
    char *out = (char *)malloc(n * 2 + 1);
    if (!out) return NULL;
    size_t oi = 0;
    for (size_t i = 0; i < n; i++) {
        char c = s[i];
        if (c == '"' || c == '\\' || c == '\n' || c == '\r' || c == '\t') {
            out[oi++] = '\\';
            if (c == '\n') out[oi++] = 'n';
            else if (c == '\r') out[oi++] = 'r';
            else if (c == '\t') out[oi++] = 't';
            else out[oi++] = c;
        } else {
            out[oi++] = c;
        }
    }
    out[oi] = '\0';
    return out;
}
// [/builtin-dev:escape_json:string]


// [builtin-dev:split_after:string]
char * str_split_after(const char *s, const char * sep) {
    extern char *str_dup(const char *s);
    if (!s) return str_dup("");
    if (!sep || sep[0] == '\0') return str_dup(s);
    const char *p = strstr(s, sep);
    if (!p) return str_dup(s);
    p += strlen(sep);
    return str_dup(p);
}
// [/builtin-dev:split_after:string]


// [builtin-dev:truncate:string]
char * str_truncate(const char *s, int max_len) {
    extern char *str_dup(const char *s);
    if (!s) return str_dup("");
    if (max_len <= 0) return str_dup("");
    size_t n = strlen(s);
    size_t take = (size_t)max_len;
    if (take > n) take = n;
    char *out = (char *)malloc(take + 1);
    if (!out) return NULL;
    memcpy(out, s, take);
    out[take] = '\0';
    return out;
}
// [/builtin-dev:truncate:string]

