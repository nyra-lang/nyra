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



