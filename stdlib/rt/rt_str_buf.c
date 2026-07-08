#include <stdlib.h>
#include <string.h>

typedef struct {
    char *data;
    size_t len;
    size_t cap;
} NyraStrBuf;

static void str_buf_ensure(NyraStrBuf *b, size_t need) {
    if (!b) {
        return;
    }
    if (b->cap >= need) {
        return;
    }
    size_t nc = b->cap ? b->cap * 2 : 16;
    while (nc < need) {
        nc *= 2;
    }
    char *nd = (char *)realloc(b->data, nc);
    if (!nd) {
        return;
    }
    b->data = nd;
    b->cap = nc;
}

void *str_buf_new(void) {
    NyraStrBuf *b = (NyraStrBuf *)calloc(1, sizeof(NyraStrBuf));
    if (!b) {
        return NULL;
    }
    str_buf_ensure(b, 1);
    if (!b->data) {
        free(b);
        return NULL;
    }
    b->data[0] = '\0';
    return b;
}

void str_buf_drop(void *handle) {
    NyraStrBuf *b = (NyraStrBuf *)handle;
    if (!b) {
        return;
    }
    free(b->data);
    free(b);
}

void str_buf_append(void *handle, const char *piece) {
    NyraStrBuf *b = (NyraStrBuf *)handle;
    if (!b || !piece) {
        return;
    }
    size_t plen = strlen(piece);
    str_buf_ensure(b, b->len + plen + 1);
    if (!b->data) {
        return;
    }
    memcpy(b->data + b->len, piece, plen);
    b->len += plen;
    b->data[b->len] = '\0';
}

void str_buf_append_char(void *handle, int ch) {
    NyraStrBuf *b = (NyraStrBuf *)handle;
    if (!b) {
        return;
    }
    unsigned char c = (unsigned char)ch;
    str_buf_ensure(b, b->len + 2);
    if (!b->data) {
        return;
    }
    b->data[b->len] = (char)c;
    b->len += 1;
    b->data[b->len] = '\0';
}

char *str_buf_build(void *handle) {
    NyraStrBuf *b = (NyraStrBuf *)handle;
    if (!b) {
        char *empty = (char *)malloc(1);
        if (empty) {
            empty[0] = '\0';
        }
        return empty;
    }
    char *out = b->data;
    if (!out) {
        out = (char *)malloc(1);
        if (out) {
            out[0] = '\0';
        }
    }
    b->data = NULL;
    b->len = 0;
    b->cap = 0;
    return out;
}
