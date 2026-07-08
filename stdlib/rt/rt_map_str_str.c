#include <stdlib.h>
#include <string.h>
#include "rt_map_handle.h"

typedef struct {
    char *key;
    char *value;
    int used;
} MapStrEntry;

typedef struct {
    MapStrEntry *entries;
    int len;
    int cap;
} NyraMapStrStr;

static unsigned hash_str(const char *s) {
    unsigned h = 5381u;
    while (*s) {
        h = ((h << 5) + h) + (unsigned char)(*s++);
    }
    return h;
}

static void map_grow(NyraMapStrStr *m) {
    int nc = m->cap * 2;
    MapStrEntry *ne = (MapStrEntry *)calloc((size_t)nc, sizeof(MapStrEntry));
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

static void map_str_str_free_inner(void *inner) {
    NyraMapStrStr *m = (NyraMapStrStr *)inner;
    if (!m) {
        return;
    }
    for (int i = 0; i < m->cap; i++) {
        if (m->entries[i].used) {
            free(m->entries[i].key);
            free(m->entries[i].value);
        }
    }
    free(m->entries);
    free(m);
}

void *map_str_str_new(void) {
    NyraMapStrStr *m = (NyraMapStrStr *)calloc(1, sizeof(NyraMapStrStr));
    if (!m) {
        return NULL;
    }
    m->cap = 16;
    m->entries = (MapStrEntry *)calloc((size_t)m->cap, sizeof(MapStrEntry));
    if (!m->entries) {
        free(m);
        return NULL;
    }
    return map_handle_wrap(m);
}

void map_str_str_insert(void *handle, const char *key, const char *value) {
    NyraMapStrStr *m = (NyraMapStrStr *)map_handle_inner(handle);
    if (!m || !key) {
        return;
    }
    if (m->len >= m->cap / 2) {
        map_grow(m);
    }
    unsigned h = hash_str(key) % (unsigned)m->cap;
    while (m->entries[h].used) {
        if (strcmp(m->entries[h].key, key) == 0) {
            free(m->entries[h].value);
            m->entries[h].value = value ? strdup(value) : strdup("");
            return;
        }
        h = (h + 1) % (unsigned)m->cap;
    }
    m->entries[h].key = strdup(key);
    m->entries[h].value = value ? strdup(value) : strdup("");
    m->entries[h].used = 1;
    m->len = m->len + 1;
}

const char *map_str_str_get(void *handle, const char *key) {
    NyraMapStrStr *m = (NyraMapStrStr *)map_handle_inner(handle);
    if (!m || !key) {
        return "";
    }
    unsigned h = hash_str(key) % (unsigned)m->cap;
    for (int i = 0; i < m->cap; i++) {
        unsigned idx = (h + (unsigned)i) % (unsigned)m->cap;
        if (!m->entries[idx].used) {
            return "";
        }
        if (strcmp(m->entries[idx].key, key) == 0) {
            return m->entries[idx].value ? m->entries[idx].value : "";
        }
    }
    return "";
}

int map_str_str_contains(void *handle, const char *key) {
    NyraMapStrStr *m = (NyraMapStrStr *)map_handle_inner(handle);
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

void map_str_str_free(void *handle) {
    map_handle_release(handle, map_str_str_free_inner);
}

void map_str_str_retain(void *handle) {
    map_handle_retain(handle);
}

extern void *vec_str_new(void);
extern void vec_str_push(void *handle, const char *value);

void *map_str_str_keys(void *handle) {
    NyraMapStrStr *m = (NyraMapStrStr *)map_handle_inner(handle);
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

int map_str_str_remove(void *handle, const char *key) {
    NyraMapStrStr *m = (NyraMapStrStr *)map_handle_inner(handle);
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
            free(m->entries[idx].value);
            m->entries[idx].key = NULL;
            m->entries[idx].value = NULL;
            m->entries[idx].used = 0;
            m->len = m->len - 1;
            return 1;
        }
    }
    return 0;
}

// [contrib-dev:map_str_str_clear:map]
void map_str_str_clear(void *handle) {
    NyraMapStrStr *m = (NyraMapStrStr *)map_handle_inner(handle);
    if (!m) {
        return;
    }
    for (int i = 0; i < m->cap; i++) {
        if (m->entries[i].used) {
            free(m->entries[i].key);
            free(m->entries[i].value);
            m->entries[i].key = NULL;
            m->entries[i].value = NULL;
            m->entries[i].used = 0;
        }
    }
    m->len = 0;
}
// [/contrib-dev:map_str_str_clear:map]

// [contrib-dev:map_str_str_len:map]
int map_str_str_len(void *handle) {
    NyraMapStrStr *m = (NyraMapStrStr *)map_handle_inner(handle);
    return m ? m->len : 0;
}
// [/contrib-dev:map_str_str_len:map]

// [contrib-dev:map_str_str_values:map]
void *map_str_str_values(void *handle) {
    NyraMapStrStr *m = (NyraMapStrStr *)map_handle_inner(handle);
    void *vec = vec_str_new();
    if (!m || !vec) {
        return vec;
    }
    for (int i = 0; i < m->cap; i++) {
        if (m->entries[i].used && m->entries[i].value) {
            vec_str_push(vec, m->entries[i].value);
        }
    }
    return vec;
}
// [/contrib-dev:map_str_str_values:map]
