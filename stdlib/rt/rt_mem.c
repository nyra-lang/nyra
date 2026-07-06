#include "rt_common.h"

#if defined(_WIN32)
#include <io.h>
#ifndef isatty
#define isatty _isatty
#endif
#ifndef STDOUT_FILENO
#define STDOUT_FILENO 1
#endif
#else
#include <unistd.h>
#endif

typedef struct NyraMemMarker {
    char *label;
    size_t rss_start;
    struct NyraMemMarker *next;
} NyraMemMarker;

static NyraMemMarker *nyra_mem_markers = NULL;

static NyraMemMarker *nyra_find_mem_marker(const char *label) {
    for (NyraMemMarker *m = nyra_mem_markers; m; m = m->next) {
        if (strcmp(m->label, label) == 0) {
            return m;
        }
    }
    return NULL;
}

static void nyra_print_mem_delta(const char *label, long long delta_bytes) {
    int color = isatty(STDOUT_FILENO);
    long long abs_bytes = delta_bytes < 0 ? -delta_bytes : delta_bytes;
    const char *unit;
    double value;
    char sign = delta_bytes < 0 ? '-' : '+';

    if (abs_bytes < 1024) {
        unit = "B";
        value = (double)abs_bytes;
    } else if (abs_bytes < 1024LL * 1024LL) {
        unit = "KB";
        value = (double)abs_bytes / 1024.0;
    } else if (abs_bytes < 1024LL * 1024LL * 1024LL) {
        unit = "MB";
        value = (double)abs_bytes / (1024.0 * 1024.0);
    } else {
        unit = "GB";
        value = (double)abs_bytes / (1024.0 * 1024.0 * 1024.0);
    }

    if (color) {
        printf("%s: RSS \033[1;32m%c%.3f\033[0m %s\n", label, sign, value, unit);
    } else {
        printf("%s: RSS %c%.3f %s\n", label, sign, value, unit);
    }
}

void mem_start(const char *label) {
    if (!label) {
        return;
    }
    NyraMemMarker *entry = nyra_find_mem_marker(label);
    if (!entry) {
        entry = (NyraMemMarker *)calloc(1, sizeof(NyraMemMarker));
        if (!entry) {
            return;
        }
        entry->label = strdup(label);
        if (!entry->label) {
            free(entry);
            return;
        }
        entry->next = nyra_mem_markers;
        nyra_mem_markers = entry;
    }
    entry->rss_start = nyra_current_rss_bytes();
}

void mem_end(const char *label) {
    if (!label) {
        return;
    }
    NyraMemMarker *entry = nyra_find_mem_marker(label);
    if (!entry) {
        printf("%s: (memory marker not started)\n", label);
        return;
    }
    size_t rss_end = nyra_current_rss_bytes();
    long long delta = (long long)rss_end - (long long)entry->rss_start;
    nyra_print_mem_delta(label, delta);
}
