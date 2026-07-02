#include <stdint.h>
#include <stdlib.h>

#if defined(_WIN32)
#include <windows.h>
#else
#include <pthread.h>
#include <unistd.h>
#endif

#if !defined(_WIN32)
#include <stdatomic.h>
#endif

typedef void (*NyraParBody)(int32_t index, void *ctx);
typedef int32_t (*NyraParPred)(int32_t index, void *ctx);

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

typedef struct {
    int32_t start;
    int32_t end;
    NyraParPred pred;
    void *ctx;
    volatile int32_t *stop;
    volatile int32_t *found_index;
    int32_t search_op; /* 0=any, 1=find, 2=all */
} NyraParSearchChunk;

#if defined(_WIN32)
static int32_t nyra_atomic_load(volatile int32_t *p) {
    return (int32_t)InterlockedCompareExchange((volatile LONG *)p, 0, 0);
}

static void nyra_atomic_store(volatile int32_t *p, int32_t v) {
    InterlockedExchange((volatile LONG *)p, (LONG)v);
}

static int nyra_atomic_cas_index(volatile int32_t *p, int32_t expected, int32_t desired) {
    return InterlockedCompareExchange((volatile LONG *)p, (LONG)desired, (LONG)expected) ==
           (LONG)expected;
}
#else
static int32_t nyra_atomic_load(volatile int32_t *p) {
    return atomic_load((atomic_int_least32_t *)p);
}

static void nyra_atomic_store(volatile int32_t *p, int32_t v) {
    atomic_store((atomic_int_least32_t *)p, v);
}

static int nyra_atomic_cas_index(volatile int32_t *p, int32_t expected, int32_t desired) {
    int32_t exp = expected;
    return atomic_compare_exchange_strong((atomic_int_least32_t *)p, &exp, desired);
}
#endif

static void nyra_par_search_chunk(NyraParSearchChunk *c) {
    for (int32_t i = c->start; i < c->end; i++) {
        if (nyra_atomic_load(c->stop)) {
            break;
        }
        int32_t ok = c->pred(i, c->ctx);
        if (c->search_op == 0) {
            if (ok) {
                nyra_atomic_store(c->stop, 1);
                break;
            }
        } else if (c->search_op == 1) {
            if (ok) {
                (void)nyra_atomic_cas_index(c->found_index, -1, i);
                nyra_atomic_store(c->stop, 1);
                break;
            }
        } else {
            if (!ok) {
                nyra_atomic_store(c->stop, 1);
                break;
            }
        }
    }
}

#if defined(_WIN32)
static DWORD WINAPI nyra_par_search_worker(LPVOID arg) {
    nyra_par_search_chunk((NyraParSearchChunk *)arg);
    return 0;
}
#else
static void *nyra_par_search_worker(void *arg) {
    nyra_par_search_chunk((NyraParSearchChunk *)arg);
    return NULL;
}
#endif

static void nyra_par_search_chunk_task(void *arg) {
    nyra_par_search_chunk((NyraParSearchChunk *)arg);
}

static int32_t parallel_search_range(int32_t start, int32_t end, NyraParPred pred, void *ctx,
                                     int32_t max_workers, int32_t exact_workers, int32_t mode,
                                     int32_t cpu_percent, int32_t backend, int32_t search_op) {
    int32_t count = end - start;
    if (count <= 0 || !pred) {
        if (search_op == 2) {
            return 1;
        }
        if (search_op == 1) {
            return -1;
        }
        return 0;
    }

    volatile int32_t stop = 0;
    volatile int32_t found_index = -1;

    int32_t workers =
        resolve_workers(count, max_workers, exact_workers, mode, cpu_percent);
    workers = clamp_workers(workers, count);

    if (workers <= 1) {
        NyraParSearchChunk chunk = {start, end, pred, ctx, &stop, &found_index, search_op};
        nyra_par_search_chunk(&chunk);
    } else if (backend == NYRA_PAR_BACKEND_THREAD) {
        int32_t chunk_sz = (count + workers - 1) / workers;
#if defined(_WIN32)
        HANDLE *ths = (HANDLE *)calloc((size_t)workers, sizeof(HANDLE));
#else
        pthread_t *ths = (pthread_t *)calloc((size_t)workers, sizeof(pthread_t));
#endif
        NyraParSearchChunk *chunks =
            (NyraParSearchChunk *)calloc((size_t)workers, sizeof(NyraParSearchChunk));
        if (!ths || !chunks) {
            NyraParSearchChunk chunk = {start, end, pred, ctx, &stop, &found_index, search_op};
            nyra_par_search_chunk(&chunk);
            free(ths);
            free(chunks);
        } else {
            int32_t launched = 0;
            int32_t pos = start;
            while (pos < end && launched < workers) {
                int32_t hi = pos + chunk_sz;
                if (hi > end) {
                    hi = end;
                }
                chunks[launched].start = pos;
                chunks[launched].end = hi;
                chunks[launched].pred = pred;
                chunks[launched].ctx = ctx;
                chunks[launched].stop = &stop;
                chunks[launched].found_index = &found_index;
                chunks[launched].search_op = search_op;
#if defined(_WIN32)
                ths[launched] =
                    CreateThread(NULL, 0, nyra_par_search_worker, &chunks[launched], 0, NULL);
                if (!ths[launched]) {
                    for (int32_t i = pos; i < end; i++) {
                        if (nyra_atomic_load(&stop)) {
                            break;
                        }
                        int32_t ok = pred(i, ctx);
                        if (search_op == 0 && ok) {
                            nyra_atomic_store(&stop, 1);
                            break;
                        }
                        if (search_op == 1 && ok) {
                            (void)nyra_atomic_cas_index(&found_index, -1, i);
                            nyra_atomic_store(&stop, 1);
                            break;
                        }
                        if (search_op == 2 && !ok) {
                            nyra_atomic_store(&stop, 1);
                            break;
                        }
                    }
                    free(ths);
                    free(chunks);
                    goto search_done;
                }
#else
                if (pthread_create(&ths[launched], NULL, nyra_par_search_worker,
                                   &chunks[launched]) != 0) {
                    for (int32_t i = pos; i < end; i++) {
                        if (nyra_atomic_load(&stop)) {
                            break;
                        }
                        int32_t ok = pred(i, ctx);
                        if (search_op == 0 && ok) {
                            nyra_atomic_store(&stop, 1);
                            break;
                        }
                        if (search_op == 1 && ok) {
                            (void)nyra_atomic_cas_index(&found_index, -1, i);
                            nyra_atomic_store(&stop, 1);
                            break;
                        }
                        if (search_op == 2 && !ok) {
                            nyra_atomic_store(&stop, 1);
                            break;
                        }
                    }
                    free(ths);
                    free(chunks);
                    goto search_done;
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
    } else {
        int32_t chunk_sz = (count + workers - 1) / workers;
        void **handles = (void **)calloc((size_t)workers, sizeof(void *));
        if (!handles) {
            NyraParSearchChunk chunk = {start, end, pred, ctx, &stop, &found_index, search_op};
            nyra_par_search_chunk(&chunk);
        } else {
            int32_t launched = 0;
            int32_t pos = start;
            while (pos < end && launched < workers) {
                if (nyra_atomic_load(&stop)) {
                    break;
                }
                int32_t hi = pos + chunk_sz;
                if (hi > end) {
                    hi = end;
                }
                NyraParSearchChunk chunk = {pos, hi, pred, ctx, &stop, &found_index, search_op};
                void *handle =
                    spawn_task_capture(nyra_par_search_chunk_task, &chunk, (int64_t)sizeof(chunk));
                if (!handle) {
                    nyra_par_search_chunk(&chunk);
                    for (int32_t j = 0; j < launched; j++) {
                        spawn_task_join(handles[j]);
                    }
                    free(handles);
                    goto search_done;
                }
                handles[launched++] = handle;
                pos = hi;
            }
            for (int32_t i = 0; i < launched; i++) {
                spawn_task_join(handles[i]);
            }
            free(handles);
        }
    }

search_done:
    if (search_op == 1) {
        return (int32_t)nyra_atomic_load(&found_index);
    }
    if (search_op == 2) {
        return nyra_atomic_load(&stop) ? 0 : 1;
    }
    return nyra_atomic_load(&stop) ? 1 : 0;
}

int32_t parallel_any_range(int32_t start, int32_t end, NyraParPred pred, void *ctx,
                           int32_t max_workers, int32_t exact_workers, int32_t mode,
                           int32_t cpu_percent, int32_t backend) {
    return parallel_search_range(start, end, pred, ctx, max_workers, exact_workers, mode,
                                 cpu_percent, backend, 0);
}

int32_t parallel_find_range(int32_t start, int32_t end, NyraParPred pred, void *ctx,
                            int32_t max_workers, int32_t exact_workers, int32_t mode,
                            int32_t cpu_percent, int32_t backend) {
    return parallel_search_range(start, end, pred, ctx, max_workers, exact_workers, mode,
                                 cpu_percent, backend, 1);
}

int32_t parallel_all_range(int32_t start, int32_t end, NyraParPred pred, void *ctx,
                           int32_t max_workers, int32_t exact_workers, int32_t mode,
                           int32_t cpu_percent, int32_t backend) {
    return parallel_search_range(start, end, pred, ctx, max_workers, exact_workers, mode,
                                 cpu_percent, backend, 2);
}
