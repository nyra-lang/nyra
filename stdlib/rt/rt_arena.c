#include <stddef.h>
#include <stdlib.h>
#include <string.h>

typedef struct Arena {
    char *base;
    size_t capacity;
    size_t offset;
    struct Arena *next;
} Arena;

void *arena_new(long long capacity) {
    if (capacity <= 0) {
        capacity = 65536;
    }
    Arena *a = (Arena *)calloc(1, sizeof(Arena));
    if (!a) {
        return NULL;
    }
    a->base = (char *)malloc((size_t)capacity);
    if (!a->base) {
        free(a);
        return NULL;
    }
    a->capacity = (size_t)capacity;
    a->offset = 0;
    return a;
}

void *arena_alloc(void *arena, long long nbytes) {
    Arena *a = (Arena *)arena;
    if (!a || nbytes <= 0) {
        return NULL;
    }
    size_t need = (size_t)nbytes;
    size_t align = 8;
    size_t off = (a->offset + align - 1) / align * align;
    if (off + need <= a->capacity) {
        void *p = a->base + off;
        a->offset = off + need;
        return p;
    }
    size_t cap = a->capacity;
    while (cap < need + align) {
        cap *= 2;
    }
    Arena *chunk = (Arena *)calloc(1, sizeof(Arena));
    if (!chunk) {
        return NULL;
    }
    chunk->base = (char *)malloc(cap);
    if (!chunk->base) {
        free(chunk);
        return NULL;
    }
    chunk->capacity = cap;
    chunk->offset = need;
    chunk->next = a->next;
    a->next = chunk;
    return chunk->base;
}

void arena_reset(void *arena) {
    Arena *a = (Arena *)arena;
    while (a) {
        a->offset = 0;
        a = a->next;
    }
}

void arena_free(void *arena) {
    Arena *a = (Arena *)arena;
    while (a) {
        Arena *next = a->next;
        free(a->base);
        free(a);
        a = next;
    }
}
