#include <stdlib.h>
#include <string.h>

#if defined(_WIN32)
#define nyra_rt_strdup _strdup
#else
#define nyra_rt_strdup strdup
#endif

static int g_argc = 0;
static char **g_argv = NULL;

void rt_args_init(int argc, char **argv) {
    g_argc = argc;
    g_argv = argv;
}

int os_arg_count(void) {
    return g_argc;
}

char *os_arg_at(int index) {
    if (!g_argv || index < 0 || index >= g_argc) {
        return nyra_rt_strdup("");
    }
    const char *s = g_argv[index];
    if (!s) {
        return nyra_rt_strdup("");
    }
    char *out = nyra_rt_strdup(s);
    return out ? out : nyra_rt_strdup("");
}

void process_exit(int code) {
    exit(code);
}

void *vec_str_from_argv(int start_index) {
    extern void *vec_str_new(void);
    extern void vec_str_push(void *v, const char *value);
    void *vec = vec_str_new();
    if (!vec) {
        return NULL;
    }
    if (!g_argv || start_index < 0) {
        return vec;
    }
    for (int i = start_index; i < g_argc; i++) {
        const char *s = g_argv[i];
        vec_str_push(vec, s ? s : "");
    }
    return vec;
}
