#include <stdlib.h>
#include <string.h>
#include "rt_map_handle.h"

typedef struct {
    char *key;
    int value;
    int used;
} MapEntry;

typedef struct {
    MapEntry *entries;
    int len;
    int cap;
} NyraMapStrI32;

static unsigned hash_str(const char *s) {
    unsigned h = 5381u;
    while (*s) {
        h = ((h << 5) + h) + (unsigned char)(*s++);
    }
    return h;
}

static void map_grow(NyraMapStrI32 *m) {
    int nc = m->cap * 2;
    MapEntry *ne = (MapEntry *)calloc((size_t)nc, sizeof(MapEntry));
    if (!ne) {
        return;
    }
    for (int i = 0; i < m->cap; i++) {
        if (m->entries[i].used) {
            unsigned h = hash_str(m->entries[i].key) % (unsigned)nc;
            while (ne[h].used) {
                h = (h + 1) % (unsigned)nc;
            }
            ne[h] = m->entries[i];
        }
    }
    free(m->entries);
    m->entries = ne;
    m->cap = nc;
}

static void map_str_i32_free_inner(void *inner) {
    NyraMapStrI32 *m = (NyraMapStrI32 *)inner;
    if (!m) {
        return;
    }
    for (int i = 0; i < m->cap; i++) {
        if (m->entries[i].used) {
            free(m->entries[i].key);
        }
    }
    free(m->entries);
    free(m);
}

void *map_str_i32_new(void) {
    NyraMapStrI32 *m = (NyraMapStrI32 *)calloc(1, sizeof(NyraMapStrI32));
    if (!m) {
        return NULL;
    }
    m->cap = 16;
    m->entries = (MapEntry *)calloc((size_t)m->cap, sizeof(MapEntry));
    if (!m->entries) {
        free(m);
        return NULL;
    }
    return map_handle_wrap(m);
}

void map_str_i32_insert(void *handle, const char *key, int value) {
    NyraMapStrI32 *m = (NyraMapStrI32 *)map_handle_inner(handle);
    if (!m || !key) {
        return;
    }
    if (m->len >= m->cap / 2) {
        map_grow(m);
    }
    unsigned h = hash_str(key) % (unsigned)m->cap;
    while (m->entries[h].used) {
        if (strcmp(m->entries[h].key, key) == 0) {
            m->entries[h].value = value;
            return;
        }
        h = (h + 1) % (unsigned)m->cap;
    }
    m->entries[h].key = strdup(key);
    m->entries[h].value = value;
    m->entries[h].used = 1;
    m->len = m->len + 1;
}

int map_str_i32_get(void *handle, const char *key) {
    NyraMapStrI32 *m = (NyraMapStrI32 *)map_handle_inner(handle);
    if (!m || !key) {
        return 0;
    }
    unsigned h = hash_str(key) % (unsigned)m->cap;
    for (int i = 0; i < m->cap; i++) {
        unsigned idx = (h + (unsigned)i) % (unsigned)m->cap;
        if (!m->entries[idx].used) {
            return 0;
        }
        if (strcmp(m->entries[idx].key, key) == 0) {
            return m->entries[idx].value;
        }
    }
    return 0;
}

int map_str_i32_contains(void *handle, const char *key) {
    NyraMapStrI32 *m = (NyraMapStrI32 *)map_handle_inner(handle);
    if (!m || !key) {
        return 0;
    }
    unsigned h = hash_str(key) % (unsigned)m->cap;
    for (int i = 0; i < m->cap; i++) {
        unsigned idx = (h + (unsigned)i) % (unsigned)m->cap;
        if (!m->entries[idx].used) {
            return 0;
        }
        if (strcmp(m->entries[idx].key, key) == 0) {
            return 1;
        }
    }
    return 0;
}

void map_str_i32_free(void *handle) {
    map_handle_release(handle, map_str_i32_free_inner);
}

void map_str_i32_retain(void *handle) {
    map_handle_retain(handle);
}

extern void *vec_str_new(void);
extern void vec_str_push(void *handle, const char *value);

void *map_str_i32_keys(void *handle) {
    NyraMapStrI32 *m = (NyraMapStrI32 *)map_handle_inner(handle);
    void *vec = vec_str_new();
    if (!m || !vec) {
        return vec;
    }
    for (int i = 0; i < m->cap; i++) {
        if (m->entries[i].used && m->entries[i].key) {
            vec_str_push(vec, m->entries[i].key);
        }
    }
    return vec;
}

int map_str_i32_remove(void *handle, const char *key) {
    NyraMapStrI32 *m = (NyraMapStrI32 *)map_handle_inner(handle);
    if (!m || !key) {
        return 0;
    }
    unsigned h = hash_str(key) % (unsigned)m->cap;
    for (int i = 0; i < m->cap; i++) {
        unsigned idx = (h + (unsigned)i) % (unsigned)m->cap;
        if (!m->entries[idx].used) {
            return 0;
        }
        if (strcmp(m->entries[idx].key, key) == 0) {
            free(m->entries[idx].key);
            m->entries[idx].key = NULL;
            m->entries[idx].used = 0;
            m->len = m->len - 1;
            return 1;
        }
    }
    return 0;
}

/* ── map[int]int (i32 keys — fair comparison with Go map[int]int) ─────────── */

typedef struct {
    int key;
    int value;
    int used;
} MapEntryI32;

typedef struct {
    MapEntryI32 *entries;
    int len;
    int cap;
} NyraMapI32I32;

static unsigned hash_i32(int k) {
    return (unsigned)k * 2654435761u;
}

static void map_i32_grow(NyraMapI32I32 *m) {
    int nc = m->cap * 2;
    MapEntryI32 *ne = (MapEntryI32 *)calloc((size_t)nc, sizeof(MapEntryI32));
    if (!ne) {
        return;
    }
    for (int i = 0; i < m->cap; i++) {
        if (m->entries[i].used) {
            unsigned h = hash_i32(m->entries[i].key) % (unsigned)nc;
            while (ne[h].used) {
                h = (h + 1) % (unsigned)nc;
            }
            ne[h] = m->entries[i];
        }
    }
    free(m->entries);
    m->entries = ne;
    m->cap = nc;
}

static void map_i32_i32_free_inner(void *inner) {
    NyraMapI32I32 *m = (NyraMapI32I32 *)inner;
    if (!m) {
        return;
    }
    free(m->entries);
    free(m);
}

void *map_i32_i32_new(void) {
    NyraMapI32I32 *m = (NyraMapI32I32 *)calloc(1, sizeof(NyraMapI32I32));
    if (!m) {
        return NULL;
    }
    m->cap = 16;
    m->entries = (MapEntryI32 *)calloc((size_t)m->cap, sizeof(MapEntryI32));
    if (!m->entries) {
        free(m);
        return NULL;
    }
    return map_handle_wrap(m);
}

void map_i32_i32_insert(void *handle, int key, int value) {
    NyraMapI32I32 *m = (NyraMapI32I32 *)map_handle_inner(handle);
    if (!m) {
        return;
    }
    if (m->len >= m->cap / 2) {
        map_i32_grow(m);
    }
    unsigned h = hash_i32(key) % (unsigned)m->cap;
    while (m->entries[h].used) {
        if (m->entries[h].key == key) {
            m->entries[h].value = value;
            return;
        }
        h = (h + 1) % (unsigned)m->cap;
    }
    m->entries[h].key = key;
    m->entries[h].value = value;
    m->entries[h].used = 1;
    m->len = m->len + 1;
}

int map_i32_i32_get(void *handle, int key) {
    NyraMapI32I32 *m = (NyraMapI32I32 *)map_handle_inner(handle);
    if (!m) {
        return 0;
    }
    unsigned h = hash_i32(key) % (unsigned)m->cap;
    for (int i = 0; i < m->cap; i++) {
        unsigned idx = (h + (unsigned)i) % (unsigned)m->cap;
        if (!m->entries[idx].used) {
            return 0;
        }
        if (m->entries[idx].key == key) {
            return m->entries[idx].value;
        }
    }
    return 0;
}

int map_i32_i32_contains(void *handle, int key) {
    NyraMapI32I32 *m = (NyraMapI32I32 *)map_handle_inner(handle);
    if (!m) {
        return 0;
    }
    unsigned h = hash_i32(key) % (unsigned)m->cap;
    for (int i = 0; i < m->cap; i++) {
        unsigned idx = (h + (unsigned)i) % (unsigned)m->cap;
        if (!m->entries[idx].used) {
            return 0;
        }
        if (m->entries[idx].key == key) {
            return 1;
        }
    }
    return 0;
}

void map_i32_i32_free(void *handle) {
    map_handle_release(handle, map_i32_i32_free_inner);
}

void map_i32_i32_retain(void *handle) {
    map_handle_retain(handle);
}
// [contrib-dev:map_str_i32_clear:map]
void map_str_i32_clear(void *handle) {
    NyraMapStrI32 *m = (NyraMapStrI32 *)map_handle_inner(handle);
    if (!m) {
        return;
    }
    for (int i = 0; i < m->cap; i++) {
        if (m->entries[i].used) {
            free(m->entries[i].key);
            m->entries[i].key = NULL;
            m->entries[i].used = 0;
        }
    }
    m->len = 0;
}
// [/contrib-dev:map_str_i32_clear:map]

// [contrib-dev:map_str_i32_len:map]
int map_str_i32_len(void *handle) {
    NyraMapStrI32 *m = (NyraMapStrI32 *)map_handle_inner(handle);
    return m ? m->len : 0;
}
// [/contrib-dev:map_str_i32_len:map]

extern void *vec_i32_new(void);
extern void vec_i32_push(void *handle, int value);

// [contrib-dev:map_str_i32_values:map]
void *map_str_i32_values(void *handle) {
    NyraMapStrI32 *m = (NyraMapStrI32 *)map_handle_inner(handle);
    void *vec = vec_i32_new();
    if (!m || !vec) {
        return vec;
    }
    for (int i = 0; i < m->cap; i++) {
        if (m->entries[i].used) {
            vec_i32_push(vec, m->entries[i].value);
        }
    }
    return vec;
}
// [/contrib-dev:map_str_i32_values:map]

