#include "rt_common.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern void mem_start(const char *label);
extern void mem_end(const char *label);

static int g_alloc_track_active = 0;
static size_t g_alloc_track_bytes = 0;
static size_t g_alloc_track_notes = 0;


void alloc_track_start(const char *label) {
    g_alloc_track_active = 1;
    g_alloc_track_bytes = 0;
    g_alloc_track_notes = 0;
    mem_start(label);
}

void alloc_track_note(size_t bytes) {
    if (g_alloc_track_active) {
        g_alloc_track_bytes += bytes;
        g_alloc_track_notes += 1;
    }
}

void alloc_track_end(const char *label) {
    g_alloc_track_active = 0;
    mem_end(label);
    if (label) {
        printf("%s: alloc_track %zu notes %zu bytes (estimated)\n", label, g_alloc_track_notes,
               g_alloc_track_bytes);
    }
}
