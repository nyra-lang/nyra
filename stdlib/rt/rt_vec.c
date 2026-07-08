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

