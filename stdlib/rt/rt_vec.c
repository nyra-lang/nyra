#include <stdlib.h>
#include <string.h>

typedef struct {
    int elem_size;
    int len;
    int cap;
    void *data;
} NyraVec;

void *vec_i32_new(void) {
    NyraVec *v = (NyraVec *)calloc(1, sizeof(NyraVec));
    if (!v) {
        return NULL;
    }
    v->elem_size = (int)sizeof(int);
    v->cap = 8;
    v->data = malloc((size_t)v->cap * (size_t)v->elem_size);
    if (!v->data) {
        free(v);
        return NULL;
    }
    return v;
}

void vec_i32_push(void *handle, int value) {
    NyraVec *v = (NyraVec *)handle;
    if (!v) {
        return;
    }
    if (v->len >= v->cap) {
        int nc = v->cap * 2;
        void *nd = realloc(v->data, (size_t)nc * (size_t)v->elem_size);
        if (!nd) {
            return;
        }
        v->data = nd;
        v->cap = nc;
    }
    ((int *)v->data)[v->len++] = value;
}

int vec_i32_get(void *handle, int index) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || index < 0 || index >= v->len) {
        return 0;
    }
    return ((int *)v->data)[index];
}

void vec_i32_set(void *handle, int index, int value) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || index < 0 || index >= v->len) {
        return;
    }
    ((int *)v->data)[index] = value;
}

int vec_i32_len(void *handle) {
    NyraVec *v = (NyraVec *)handle;
    return v ? v->len : 0;
}

int vec_i32_pop(void *handle) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || v->len <= 0) {
        return 0;
    }
    return ((int *)v->data)[--v->len];
}

void vec_i32_free(void *handle) {
    NyraVec *v = (NyraVec *)handle;
    if (!v) {
        return;
    }
    free(v->data);
    free(v);
}

static char *nyra_strdup(const char *s) {
    if (!s) {
        return NULL;
    }
    size_t n = strlen(s);
    char *out = (char *)malloc(n + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, s, n + 1);
    return out;
}

void *vec_str_new(void) {
    NyraVec *v = (NyraVec *)calloc(1, sizeof(NyraVec));
    if (!v) {
        return NULL;
    }
    v->elem_size = (int)sizeof(char *);
    v->cap = 4;
    v->data = malloc((size_t)v->cap * (size_t)v->elem_size);
    if (!v->data) {
        free(v);
        return NULL;
    }
    return v;
}

void vec_str_push(void *handle, const char *value) {
    NyraVec *v = (NyraVec *)handle;
    if (!v) {
        return;
    }
    if (v->len >= v->cap) {
        int nc = v->cap * 2;
        void *nd = realloc(v->data, (size_t)nc * (size_t)v->elem_size);
        if (!nd) {
            return;
        }
        v->data = nd;
        v->cap = nc;
    }
    ((char **)v->data)[v->len++] = nyra_strdup(value ? value : "");
}

const char *vec_str_get(void *handle, int index) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || index < 0 || index >= v->len) {
        return "";
    }
    const char *s = ((char **)v->data)[index];
    return s ? s : "";
}

int vec_str_len(void *handle) {
    NyraVec *v = (NyraVec *)handle;
    return v ? v->len : 0;
}

void vec_str_free(void *handle) {
    NyraVec *v = (NyraVec *)handle;
    if (!v) {
        return;
    }
    for (int i = 0; i < v->len; i++) {
        free(((char **)v->data)[i]);
    }
    free(v->data);
    free(v);
}

// POD element vector (memcpy push/get) for monomorph Vec<CopyStruct>.
void *vec_bytes_new(int elem_size) {
    if (elem_size <= 0) {
        return NULL;
    }
    NyraVec *v = (NyraVec *)calloc(1, sizeof(NyraVec));
    if (!v) {
        return NULL;
    }
    v->elem_size = elem_size;
    v->cap = 4;
    v->data = malloc((size_t)v->cap * (size_t)v->elem_size);
    if (!v->data) {
        free(v);
        return NULL;
    }
    return v;
}

void vec_bytes_push(void *handle, void *elem) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || !elem || v->elem_size <= 0) {
        return;
    }
    if (v->len >= v->cap) {
        int nc = v->cap * 2;
        void *nd = realloc(v->data, (size_t)nc * (size_t)v->elem_size);
        if (!nd) {
            return;
        }
        v->data = nd;
        v->cap = nc;
    }
    memcpy((char *)v->data + (size_t)v->len * (size_t)v->elem_size, elem,
           (size_t)v->elem_size);
    v->len++;
}

void vec_bytes_get(void *handle, int index, void *out) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || !out || index < 0 || index >= v->len || v->elem_size <= 0) {
        return;
    }
    memcpy(out, (char *)v->data + (size_t)index * (size_t)v->elem_size,
           (size_t)v->elem_size);
}

int vec_bytes_len(void *handle) {
    NyraVec *v = (NyraVec *)handle;
    return v ? v->len : 0;
}

void vec_bytes_free(void *handle) {
    NyraVec *v = (NyraVec *)handle;
    if (!v) {
        return;
    }
    free(v->data);
    free(v);
}

void vec_bytes_push_ptr(void *handle, void *elem) {
    vec_bytes_push(handle, &elem);
}

void *vec_bytes_get_ptr(void *handle, int index) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || index < 0 || index >= v->len || v->elem_size != (int)sizeof(void *)) {
        return NULL;
    }
    return ((void **)v->data)[index];
}
// [contrib-dev:vec_i32_clear:vec]
void vec_i32_clear(void *handle) {
    NyraVec *v = (NyraVec *)handle;
    if (!v) {
        return;
    }
    v->len = 0;
}
// [/contrib-dev:vec_i32_clear:vec]

// [contrib-dev:vec_i32_insert:vec]
void vec_i32_insert(void *handle, int index, int value) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || index < 0 || index > v->len) {
        return;
    }
    if (v->len >= v->cap) {
        int nc = v->cap * 2;
        void *nd = realloc(v->data, (size_t)nc * (size_t)v->elem_size);
        if (!nd) {
            return;
        }
        v->data = nd;
        v->cap = nc;
    }
    int *data = (int *)v->data;
    memmove(data + index + 1, data + index, (size_t)(v->len - index) * sizeof(int));
    data[index] = value;
    v->len++;
}
// [/contrib-dev:vec_i32_insert:vec]

// [contrib-dev:vec_i32_remove_at:vec]
int vec_i32_remove_at(void *handle, int index) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || index < 0 || index >= v->len) {
        return 0;
    }
    int *data = (int *)v->data;
    int removed = data[index];
    memmove(data + index, data + index + 1, (size_t)(v->len - index - 1) * sizeof(int));
    v->len--;
    return removed;
}
// [/contrib-dev:vec_i32_remove_at:vec]

// [contrib-dev:vec_i32_reverse:vec]
void vec_i32_reverse(void *handle) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || v->len <= 1) {
        return;
    }
    int *data = (int *)v->data;
    int lo = 0;
    int hi = v->len - 1;
    while (lo < hi) {
        int tmp = data[lo];
        data[lo] = data[hi];
        data[hi] = tmp;
        lo++;
        hi--;
    }
}
// [/contrib-dev:vec_i32_reverse:vec]

static int vec_i32_cmp(const void *a, const void *b) {
    int av = *(const int *)a;
    int bv = *(const int *)b;
    if (av < bv) {
        return -1;
    }
    if (av > bv) {
        return 1;
    }
    return 0;
}

// [contrib-dev:vec_i32_sort:vec]
void vec_i32_sort(void *handle) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || v->len <= 1) {
        return;
    }
    qsort(v->data, (size_t)v->len, sizeof(int), vec_i32_cmp);
}
// [/contrib-dev:vec_i32_sort:vec]

// [contrib-dev:vec_str_clear:vec_str]
void vec_str_clear(void * handle) {
    NyraVec *v = (NyraVec *)handle;
    if (!v) return;
    for (int i = 0; i < v->len; i++) free(((char **)v->data)[i]);
    v->len = 0;
}
// [/contrib-dev:vec_str_clear:vec_str]

// [contrib-dev:vec_str_pop:vec_str]
char * vec_str_pop(void * handle) {
    extern char *str_dup(const char *s);
    NyraVec *v = (NyraVec *)handle;
    if (!v || v->len <= 0) return str_dup("");
    char *top = ((char **)v->data)[--v->len];
    char *out = top ? str_dup(top) : str_dup("");
    free(top);
    return out;
}
// [/contrib-dev:vec_str_pop:vec_str]

// [contrib-dev:vec_str_reverse:vec_str]
void vec_str_reverse(void * handle) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || v->len <= 1) return;
    char **data = (char **)v->data;
    int lo = 0, hi = v->len - 1;
    while (lo < hi) {
        char *tmp = data[lo];
        data[lo] = data[hi];
        data[hi] = tmp;
        lo++; hi--;
    }
}
// [/contrib-dev:vec_str_reverse:vec_str]

// [contrib-dev:vec_str_extend:vec_str]
void vec_str_extend(void * dst, void * src) {
    extern void vec_str_push(void *handle, const char *value);
    NyraVec *d = (NyraVec *)dst;
    NyraVec *s = (NyraVec *)src;
    if (!d || !s) return;
    for (int i = 0; i < s->len; i++) {
        const char *item = ((char **)s->data)[i];
        vec_str_push(d, item ? item : "");
    }
}
// [/contrib-dev:vec_str_extend:vec_str]

// [contrib-dev:vec_str_insert:vec_str]
void vec_str_insert(void * handle, int index, const char * value) {
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
    v->len++;
}
// [/contrib-dev:vec_str_insert:vec_str]

// [contrib-dev:vec_str_remove_at:vec_str]
char * vec_str_remove_at(void * handle, int index) {
    extern char *str_dup(const char *s);
    NyraVec *v = (NyraVec *)handle;
    if (!v || index < 0 || index >= v->len) return str_dup("");
    char **data = (char **)v->data;
    char *removed = data[index];
    memmove(data + index, data + index + 1, (size_t)(v->len - index - 1) * sizeof(char *));
    v->len--;
    char *out = removed ? str_dup(removed) : str_dup("");
    free(removed);
    return out;
}
// [/contrib-dev:vec_str_remove_at:vec_str]

// [contrib-dev:vec_str_set:vec_str]
void vec_str_set(void * handle, int index, const char * value) {
    extern char *str_dup(const char *s);
    NyraVec *v = (NyraVec *)handle;
    if (!v || index < 0 || index >= v->len) return;
    char **data = (char **)v->data;
    free(data[index]);
    data[index] = str_dup(value ? value : "");
}
// [/contrib-dev:vec_str_set:vec_str]

// [contrib-dev:vec_str_swap:vec_str]
void vec_str_swap(void * handle, int i, int j) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || i < 0 || j < 0 || i >= v->len || j >= v->len) return;
    char **data = (char **)v->data;
    char *tmp = data[i];
    data[i] = data[j];
    data[j] = tmp;
}
// [/contrib-dev:vec_str_swap:vec_str]

// [contrib-dev:vec_i32_extend:vec]
void vec_i32_extend(void * dst, void * src) {
    extern void vec_i32_push(void *handle, int value);
    NyraVec *d = (NyraVec *)dst;
    NyraVec *s = (NyraVec *)src;
    if (!d || !s) return;
    for (int i = 0; i < s->len; i++) {
        vec_i32_push(d, ((int *)s->data)[i]);
    }
}
// [/contrib-dev:vec_i32_extend:vec]

// [contrib-dev:vec_i32_swap:vec]
void vec_i32_swap(void * handle, int i, int j) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || i < 0 || j < 0 || i >= v->len || j >= v->len) return;
    int *data = (int *)v->data;
    int tmp = data[i];
    data[i] = data[j];
    data[j] = tmp;
}
// [/contrib-dev:vec_i32_swap:vec]

// [contrib-dev:vec_i32_capacity:vec]
int vec_i32_capacity(void * handle) {
    NyraVec *v = (NyraVec *)handle;
    return v ? v->cap : 0;
}
// [/contrib-dev:vec_i32_capacity:vec]

// [contrib-dev:vec_i32_fill:vec]
void vec_i32_fill(void * handle, int value) {
    NyraVec *v = (NyraVec *)handle;
    if (!v) return;
    int *data = (int *)v->data;
    for (int i = 0; i < v->len; i++) data[i] = value;
}
// [/contrib-dev:vec_i32_fill:vec]

// [contrib-dev:vec_i32_reserve:vec]
void vec_i32_reserve(void * handle, int min_cap) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || min_cap <= v->cap) return;
    int nc = v->cap;
    while (nc < min_cap) nc = nc < 1 ? 8 : nc * 2;
    void *nd = realloc(v->data, (size_t)nc * (size_t)v->elem_size);
    if (!nd) return;
    v->data = nd;
    v->cap = nc;
}
// [/contrib-dev:vec_i32_reserve:vec]

// [contrib-dev:vec_i32_swap_remove:vec]
int vec_i32_swap_remove(void * handle, int index) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || index < 0 || index >= v->len) return 0;
    int *data = (int *)v->data;
    int removed = data[index];
    data[index] = data[v->len - 1];
    v->len--;
    return removed;
}
// [/contrib-dev:vec_i32_swap_remove:vec]

// [contrib-dev:vec_i32_truncate:vec]
void vec_i32_truncate(void * handle, int len) {
    NyraVec *v = (NyraVec *)handle;
    if (!v || len < 0) return;
    if (len < v->len) v->len = len;
}
// [/contrib-dev:vec_i32_truncate:vec]

