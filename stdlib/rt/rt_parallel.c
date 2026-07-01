#include <stdint.h>
#include <stdlib.h>

#if defined(_WIN32)
#include <windows.h>
#else
#include <pthread.h>
#include <unistd.h>
#endif

typedef void (*NyraParBody)(int32_t index, void *ctx);

/* Task pool backend (rt_task_pool.c) */
void *spawn_task_capture(void (*body)(void *), void *data, int64_t nbytes);
int spawn_task_join(void *handle);

#define NYRA_PAR_BACKEND_TASK 0
#define NYRA_PAR_BACKEND_THREAD 1

static int32_t nyra_ncpus(void) {
#if defined(_WIN32)
    SYSTEM_INFO si;
    GetSystemInfo(&si);
    int32_t n = (int32_t)si.dwNumberOfProcessors;
    return n > 0 ? n : 4;
#else
    long n = sysconf(_SC_NPROCESSORS_ONLN);
    return n > 0 ? (int32_t)n : 4;
#endif
}

int32_t cpu_count(void) {
    return nyra_ncpus();
}

typedef struct {
    int32_t start;
    int32_t end;
    NyraParBody body;
    void *ctx;
} NyraParChunk;

#if defined(_WIN32)
static DWORD WINAPI nyra_par_worker(LPVOID arg) {
    NyraParChunk *c = (NyraParChunk *)arg;
    for (int32_t i = c->start; i < c->end; i++) {
        c->body(i, c->ctx);
    }
    return 0;
}
#else
static void *nyra_par_worker(void *arg) {
    NyraParChunk *c = (NyraParChunk *)arg;
    for (int32_t i = c->start; i < c->end; i++) {
        c->body(i, c->ctx);
    }
    return NULL;
}
#endif

static void nyra_par_chunk_task(void *arg) {
    NyraParChunk *c = (NyraParChunk *)arg;
    for (int32_t i = c->start; i < c->end; i++) {
        c->body(i, c->ctx);
    }
}

static int32_t clamp_workers(int32_t workers, int32_t count) {
    if (workers < 1) {
        workers = 1;
    }
    if (workers > count) {
        workers = count;
    }
    return workers;
}

static int32_t resolve_workers(int32_t count, int32_t max_workers, int32_t exact_workers,
                               int32_t mode, int32_t cpu_percent) {
    if (exact_workers > 0) {
        return clamp_workers(exact_workers, count);
    }

    int32_t cpus = nyra_ncpus();
    int32_t workers;

    if (cpu_percent > 0) {
        if (cpu_percent > 100) {
            cpu_percent = 100;
        }
        workers = (cpus * cpu_percent + 99) / 100;
        if (workers < 1) {
            workers = 1;
        }
    } else {
        switch (mode) {
        case 1: /* balanced */
            workers = cpus > 1 ? cpus - 1 : 1;
            break;
        case 2: /* max_performance */
            workers = cpus;
            break;
        case 3: /* background */
            workers = cpus / 2;
            if (workers < 1) {
                workers = 1;
            }
            break;
        default: /* auto */
            workers = cpus;
            break;
        }
    }

    if (max_workers > 0 && workers > max_workers) {
        workers = max_workers;
    }
    return clamp_workers(workers, count);
}

static void parallel_for_range_impl(int32_t start, int32_t end, NyraParBody body, void *ctx,
                                    int32_t workers) {
    if (!body || end <= start) {
        return;
    }
    int32_t count = end - start;
    workers = clamp_workers(workers, count);
    if (workers <= 1) {
        for (int32_t i = start; i < end; i++) {
            body(i, ctx);
        }
        return;
    }

    int32_t chunk_sz = (count + workers - 1) / workers;
#if defined(_WIN32)
    HANDLE *ths = (HANDLE *)calloc((size_t)workers, sizeof(HANDLE));
#else
    pthread_t *ths = (pthread_t *)calloc((size_t)workers, sizeof(pthread_t));
#endif
    NyraParChunk *chunks = (NyraParChunk *)calloc((size_t)workers, sizeof(NyraParChunk));
    if (!ths || !chunks) {
        for (int32_t i = start; i < end; i++) {
            body(i, ctx);
        }
        free(ths);
        free(chunks);
        return;
    }

    int32_t launched = 0;
    int32_t pos = start;
    while (pos < end && launched < workers) {
        int32_t hi = pos + chunk_sz;
        if (hi > end) {
            hi = end;
        }
        chunks[launched].start = pos;
        chunks[launched].end = hi;
        chunks[launched].body = body;
        chunks[launched].ctx = ctx;
#if defined(_WIN32)
        ths[launched] = CreateThread(NULL, 0, nyra_par_worker, &chunks[launched], 0, NULL);
        if (!ths[launched]) {
            for (int32_t i = pos; i < end; i++) {
                body(i, ctx);
            }
            free(ths);
            free(chunks);
            return;
        }
#else
        if (pthread_create(&ths[launched], NULL, nyra_par_worker, &chunks[launched]) != 0) {
            for (int32_t i = pos; i < end; i++) {
                body(i, ctx);
            }
            free(ths);
            free(chunks);
            return;
        }
#endif
        pos = hi;
        launched++;
    }

#if defined(_WIN32)
    WaitForMultipleObjects((DWORD)launched, ths, TRUE, INFINITE);
    for (int32_t i = 0; i < launched; i++) {
        if (ths[i]) {
            CloseHandle(ths[i]);
        }
    }
#else
    for (int32_t i = 0; i < launched; i++) {
        pthread_join(ths[i], NULL);
    }
#endif

    free(ths);
    free(chunks);
}

static void parallel_for_range_task(int32_t start, int32_t end, NyraParBody body, void *ctx,
                                    int32_t workers) {
    if (!body || end <= start) {
        return;
    }
    int32_t count = end - start;
    workers = clamp_workers(workers, count);
    if (workers <= 1) {
        for (int32_t i = start; i < end; i++) {
            body(i, ctx);
        }
        return;
    }

    int32_t chunk_sz = (count + workers - 1) / workers;
    void **handles = (void **)calloc((size_t)workers, sizeof(void *));
    if (!handles) {
        for (int32_t i = start; i < end; i++) {
            body(i, ctx);
        }
        return;
    }

    int32_t launched = 0;
    int32_t pos = start;
    while (pos < end && launched < workers) {
        int32_t hi = pos + chunk_sz;
        if (hi > end) {
            hi = end;
        }
        NyraParChunk chunk = {pos, hi, body, ctx};
        void *handle = spawn_task_capture(nyra_par_chunk_task, &chunk, (int64_t)sizeof(chunk));
        if (!handle) {
            for (int32_t j = 0; j < launched; j++) {
                spawn_task_join(handles[j]);
            }
            for (int32_t i = pos; i < end; i++) {
                body(i, ctx);
            }
            free(handles);
            return;
        }
        handles[launched++] = handle;
        pos = hi;
    }

    for (int32_t i = 0; i < launched; i++) {
        spawn_task_join(handles[i]);
    }
    free(handles);
}

void parallel_for_range(int32_t start, int32_t end, NyraParBody body, void *ctx,
                        int32_t max_workers, int32_t exact_workers, int32_t mode,
                        int32_t cpu_percent, int32_t backend) {
    int32_t count = end - start;
    if (count <= 0 || !body) {
        return;
    }
    int32_t workers =
        resolve_workers(count, max_workers, exact_workers, mode, cpu_percent);
    if (backend == NYRA_PAR_BACKEND_THREAD) {
        parallel_for_range_impl(start, end, body, ctx, workers);
    } else {
        parallel_for_range_task(start, end, body, ctx, workers);
    }
}
