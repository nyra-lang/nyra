#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if !defined(_WIN32)
#include <execinfo.h>
#endif

char *error_stack_trace(void) {
#if defined(_WIN32)
    const char *msg = "stack trace unavailable on this target";
    char *out = (char *)malloc(strlen(msg) + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, msg, strlen(msg) + 1);
    return out;
#else
    void *frames[32];
    int n = backtrace(frames, 32);
    char **symbols = backtrace_symbols(frames, n);
    if (!symbols || n <= 0) {
        const char *msg = "stack trace unavailable";
        char *out = (char *)malloc(strlen(msg) + 1);
        if (out) {
            memcpy(out, msg, strlen(msg) + 1);
        }
        free(symbols);
        return out;
    }

    size_t len = 1;
    for (int i = 0; i < n; i++) {
        len += strlen(symbols[i]) + 1;
    }
    char *out = (char *)malloc(len);
    if (!out) {
        free(symbols);
        return NULL;
    }
    out[0] = '\0';
    for (int i = 0; i < n; i++) {
        strcat(out, symbols[i]);
        strcat(out, "\n");
    }
    free(symbols);
    return out;
#endif
}
