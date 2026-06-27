#ifndef NYRA_RT_COMMON_H
#define NYRA_RT_COMMON_H

/* clock_gettime requires POSIX visibility before any libc headers on Linux. */
#if defined(__linux__) && !defined(_POSIX_C_SOURCE) && !defined(_DEFAULT_SOURCE) \
    && !defined(_GNU_SOURCE)
#define _DEFAULT_SOURCE
#endif

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#if defined(__APPLE__)
#include <mach/mach.h>
#include <mach/mach_time.h>
#else
#include <time.h>
#endif

typedef uint64_t NyraTimeStamp;

static inline NyraTimeStamp nyra_now(void) {
#if defined(__APPLE__)
    return mach_absolute_time();
#else
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec;
#endif
}

static inline double nyra_elapsed_seconds(NyraTimeStamp start, NyraTimeStamp end) {
#if defined(__APPLE__)
    static mach_timebase_info_data_t timebase;
    if (timebase.denom == 0) {
        mach_timebase_info(&timebase);
    }
    uint64_t elapsed = end - start;
    return (double)elapsed * (double)timebase.numer / (double)timebase.denom / 1e9;
#else
    return (double)(end - start) / 1e9;
#endif
}

static inline size_t nyra_current_rss_bytes(void) {
#if defined(__APPLE__)
    struct task_basic_info info;
    mach_msg_type_number_t count = TASK_BASIC_INFO_COUNT;
    if (task_info(mach_task_self(), TASK_BASIC_INFO, (task_info_t)&info, &count) != KERN_SUCCESS) {
        return 0;
    }
    return (size_t)info.resident_size;
#elif defined(__linux__)
    FILE *f = fopen("/proc/self/statm", "r");
    if (!f) {
        return 0;
    }
    unsigned long resident_pages = 0;
    if (fscanf(f, "%*u %lu", &resident_pages) != 1) {
        fclose(f);
        return 0;
    }
    fclose(f);
    long page_size = sysconf(_SC_PAGESIZE);
    if (page_size <= 0) {
        page_size = 4096;
    }
    return (size_t)resident_pages * (size_t)page_size;
#else
    return 0;
#endif
}

#endif
