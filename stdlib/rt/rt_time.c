#include "rt_common.h"
#include <time.h>
#if defined(_WIN32)
#include <io.h>
#include <windows.h>
#endif

typedef struct NyraTimer {
    char *label;
    NyraTimeStamp start;
    struct NyraTimer *next;
} NyraTimer;

static NyraTimer *nyra_timers = NULL;

static NyraTimer *nyra_find_timer(const char *label) {
    for (NyraTimer *t = nyra_timers; t; t = t->next) {
        if (strcmp(t->label, label) == 0) {
            return t;
        }
    }
    return NULL;
}

static int nyra_stdout_is_tty(void) {
#if defined(_WIN32)
    return _isatty(_fileno(stdout));
#else
    return isatty(STDOUT_FILENO);
#endif
}

static void nyra_print_elapsed(const char *label, double elapsed_s) {
    const char *unit;
    double value;
    int color = nyra_stdout_is_tty();

    if (elapsed_s >= 1.0) {
        unit = "s";
        value = elapsed_s;
    } else if (elapsed_s >= 1e-3) {
        unit = "ms";
        value = elapsed_s * 1e3;
    } else if (elapsed_s >= 1e-6) {
        unit = "\xc2\xb5s";
        value = elapsed_s * 1e6;
    } else if (elapsed_s >= 1e-9) {
        unit = "ns";
        value = elapsed_s * 1e9;
    } else if (elapsed_s >= 1e-12) {
        unit = "ps";
        value = elapsed_s * 1e12;
    } else if (elapsed_s >= 1e-15) {
        unit = "fs";
        value = elapsed_s * 1e15;
    } else {
        unit = "as";
        value = elapsed_s * 1e18;
    }

    if (color) {
        printf("%s: \033[1;32m%.6f\033[0m %s\n", label, value, unit);
    } else {
        printf("%s: %.6f %s\n", label, value, unit);
    }
}

void time_start(const char *label) {
    if (!label) {
        return;
    }
    NyraTimer *entry = nyra_find_timer(label);
    if (!entry) {
        entry = (NyraTimer *)calloc(1, sizeof(NyraTimer));
        if (!entry) {
            return;
        }
        entry->label = nyra_rt_strdup(label);
        if (!entry->label) {
            free(entry);
            return;
        }
        entry->next = nyra_timers;
        nyra_timers = entry;
    }
    entry->start = nyra_now();
}

void time_end(const char *label) {
    if (!label) {
        return;
    }
    NyraTimer *entry = nyra_find_timer(label);
    if (!entry) {
        printf("%s: (timer not started)\n", label);
        return;
    }
    nyra_print_elapsed(label, nyra_elapsed_seconds(entry->start, nyra_now()));
}

int64_t instant_now(void) {
    return (int64_t)nyra_now();
}

int instant_elapsed_ms(int64_t start) {
    double s = nyra_elapsed_seconds((NyraTimeStamp)start, nyra_now());
    return (int)(s * 1000.0);
}

void sleep_ms(int ms) {
    if (ms <= 0) {
        return;
    }
#if defined(__APPLE__) || defined(__linux__)
    struct timespec ts;
    ts.tv_sec = ms / 1000;
    ts.tv_nsec = (long)(ms % 1000) * 1000000L;
    nanosleep(&ts, NULL);
#elif defined(_WIN32)
    Sleep((DWORD)ms);
#else
    (void)ms;
#endif
}

void date_now(int *out) {
    if (!out) {
        return;
    }
    for (int i = 0; i < 8; i++) {
        out[i] = 0;
    }
#if defined(_WIN32)
    SYSTEMTIME st;
    GetLocalTime(&st);
    out[0] = (int)st.wYear;
    out[1] = (int)st.wMonth;
    out[2] = (int)st.wDay;
    out[3] = (int)st.wHour;
    out[4] = (int)st.wMinute;
    out[5] = (int)st.wSecond;
    out[6] = (int)st.wDayOfWeek;
    out[7] = (int)st.wMilliseconds;
#else
    struct timespec ts;
    struct tm tm_buf;
    clock_gettime(CLOCK_REALTIME, &ts);
    time_t sec = ts.tv_sec;
    struct tm *tm = localtime_r(&sec, &tm_buf);
    if (!tm) {
        return;
    }
    out[0] = tm->tm_year + 1900;
    out[1] = tm->tm_mon + 1;
    out[2] = tm->tm_mday;
    out[3] = tm->tm_hour;
    out[4] = tm->tm_min;
    out[5] = tm->tm_sec;
    out[6] = tm->tm_wday;
    out[7] = (int)(ts.tv_nsec / 1000000L);
#endif
}
