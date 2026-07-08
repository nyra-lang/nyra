// Dedicated I/O thread pool — blocking poll/read off the main event loop thread.
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <limits.h>

#if defined(_WIN32)
#include <io.h>
#include <windows.h>
#else
#include <pthread.h>
#include <poll.h>
#include <unistd.h>
#endif

void async_promise_complete(int handle, int value);
void io_pool_shutdown(int32_t pool);

#define NYRA_IO_POOL_MAX 8
#define NYRA_IO_JOB_CAP 256

typedef enum {
    IO_JOB_WAIT_READABLE = 1,
    IO_JOB_READ = 2,
} IoJobKind;

typedef struct {
    int active;
    int kind;
    int fd;
    int promise_id;
    void *buf;
    int64_t nbytes;
} IoJob;

typedef struct {
    int in_use;
    int shutdown;
    int n_workers;
#if defined(_WIN32)
    HANDLE *threads;
    CRITICAL_SECTION mu;
    CONDITION_VARIABLE cv;
#else
    pthread_t *threads;
    pthread_mutex_t mu;
    pthread_cond_t cv;
#endif
    IoJob jobs[NYRA_IO_JOB_CAP];
    int q_head;
    int q_tail;
    int q_count;
} IoPool;

static IoPool g_pools[NYRA_IO_POOL_MAX];

static IoPool *pool_get(int id) {
    if (id < 0 || id >= NYRA_IO_POOL_MAX || !g_pools[id].in_use) {
        return NULL;
    }
    return &g_pools[id];
}

static int pool_push(IoPool *p, IoJob job) {
#if defined(_WIN32)
    EnterCriticalSection(&p->mu);
#else
    pthread_mutex_lock(&p->mu);
#endif
    if (p->q_count >= NYRA_IO_JOB_CAP) {
#if defined(_WIN32)
        LeaveCriticalSection(&p->mu);
#else
        pthread_mutex_unlock(&p->mu);
#endif
        return -1;
    }
    p->jobs[p->q_tail] = job;
    p->q_tail = (p->q_tail + 1) % NYRA_IO_JOB_CAP;
    p->q_count++;
#if defined(_WIN32)
    WakeAllConditionVariable(&p->cv);
    LeaveCriticalSection(&p->mu);
#else
    pthread_cond_broadcast(&p->cv);
    pthread_mutex_unlock(&p->mu);
#endif
    return 0;
}

static int pool_pop(IoPool *p, IoJob *out) {
#if defined(_WIN32)
    EnterCriticalSection(&p->mu);
    while (p->q_count == 0 && !p->shutdown) {
        SleepConditionVariableCS(&p->cv, &p->mu, INFINITE);
    }
    if (p->q_count == 0) {
        LeaveCriticalSection(&p->mu);
        return 0;
    }
#else
    pthread_mutex_lock(&p->mu);
    while (p->q_count == 0 && !p->shutdown) {
        pthread_cond_wait(&p->cv, &p->mu);
    }
    if (p->q_count == 0) {
        pthread_mutex_unlock(&p->mu);
        return 0;
    }
#endif
    *out = p->jobs[p->q_head];
    p->q_head = (p->q_head + 1) % NYRA_IO_JOB_CAP;
    p->q_count--;
#if defined(_WIN32)
    LeaveCriticalSection(&p->mu);
#else
    pthread_mutex_unlock(&p->mu);
#endif
    return 1;
}

static void run_job(IoJob *job) {
    if (!job || !job->active || job->promise_id <= 0 || job->fd < 0) {
        return;
    }
    if (job->kind == IO_JOB_WAIT_READABLE) {
#if defined(_WIN32)
        async_promise_complete(job->promise_id, job->fd);
#else
        struct pollfd pfd;
        pfd.fd = job->fd;
        pfd.events = POLLIN;
        pfd.revents = 0;
        int rc = poll(&pfd, 1, -1);
        if (rc > 0 && (pfd.revents & POLLIN)) {
            async_promise_complete(job->promise_id, job->fd);
        } else {
            async_promise_complete(job->promise_id, -1);
        }
#endif
    } else if (job->kind == IO_JOB_READ && job->buf && job->nbytes > 0) {
#if defined(_WIN32)
        unsigned int count = job->nbytes > INT_MAX ? (unsigned int)INT_MAX : (unsigned int)job->nbytes;
        int n = _read(job->fd, job->buf, count);
        async_promise_complete(job->promise_id, n);
#else
        ssize_t n = read(job->fd, job->buf, (size_t)job->nbytes);
        async_promise_complete(job->promise_id, (int)n);
#endif
    }
}

#if defined(_WIN32)
static DWORD WINAPI io_worker_main(LPVOID arg) {
#else
static void *io_worker_main(void *arg) {
#endif
    int pool_id = (int)(intptr_t)arg;
    IoPool *p = pool_get(pool_id);
    if (!p) {
#if defined(_WIN32)
        return 0;
#else
        return NULL;
#endif
    }
    for (;;) {
        IoJob job = {0};
        if (!pool_pop(p, &job)) {
            if (p->shutdown) {
                break;
            }
            continue;
        }
        run_job(&job);
    }
#if defined(_WIN32)
    return 0;
#else
    return NULL;
#endif
}

int32_t io_pool_create(int32_t workers) {
    if (workers <= 0) {
        workers = 2;
    }
    if (workers > 16) {
        workers = 16;
    }
    int slot = -1;
    for (int i = 0; i < NYRA_IO_POOL_MAX; i++) {
        if (!g_pools[i].in_use) {
            slot = i;
            break;
        }
    }
    if (slot < 0) {
        return -1;
    }
    IoPool *p = &g_pools[slot];
    memset(p, 0, sizeof(*p));
    p->in_use = 1;
    p->n_workers = workers;
#if defined(_WIN32)
    InitializeCriticalSection(&p->mu);
    InitializeConditionVariable(&p->cv);
    p->threads = (HANDLE *)calloc((size_t)workers, sizeof(HANDLE));
#else
    pthread_mutex_init(&p->mu, NULL);
    pthread_cond_init(&p->cv, NULL);
    p->threads = (pthread_t *)calloc((size_t)workers, sizeof(pthread_t));
#endif
    if (!p->threads) {
        p->in_use = 0;
        return -1;
    }
    for (int i = 0; i < workers; i++) {
#if defined(_WIN32)
        p->threads[i] = CreateThread(NULL, 0, io_worker_main, (LPVOID)(intptr_t)slot, 0, NULL);
        if (!p->threads[i]) {
            io_pool_shutdown((int32_t)slot);
            return -1;
        }
#else
        if (pthread_create(&p->threads[i], NULL, io_worker_main, (void *)(intptr_t)slot) != 0) {
            io_pool_shutdown((int32_t)slot);
            return -1;
        }
#endif
    }
    return (int32_t)slot;
}

void io_pool_shutdown(int32_t pool) {
    IoPool *p = pool_get(pool);
    if (!p) {
        return;
    }
#if defined(_WIN32)
    EnterCriticalSection(&p->mu);
#else
    pthread_mutex_lock(&p->mu);
#endif
    p->shutdown = 1;
#if defined(_WIN32)
    WakeAllConditionVariable(&p->cv);
    LeaveCriticalSection(&p->mu);
    for (int i = 0; i < p->n_workers; i++) {
        if (p->threads[i]) {
            WaitForSingleObject(p->threads[i], INFINITE);
            CloseHandle(p->threads[i]);
        }
    }
    DeleteCriticalSection(&p->mu);
#else
    pthread_cond_broadcast(&p->cv);
    pthread_mutex_unlock(&p->mu);
    for (int i = 0; i < p->n_workers; i++) {
        pthread_join(p->threads[i], NULL);
    }
    pthread_mutex_destroy(&p->mu);
    pthread_cond_destroy(&p->cv);
#endif
    free(p->threads);
    memset(p, 0, sizeof(*p));
}

int32_t io_pool_submit_wait_readable(int32_t pool, int32_t fd, int32_t promise) {
    IoPool *p = pool_get(pool);
    if (!p || fd < 0 || promise <= 0) {
        return -1;
    }
    IoJob job = {0};
    job.active = 1;
    job.kind = IO_JOB_WAIT_READABLE;
    job.fd = fd;
    job.promise_id = promise;
    return pool_push(p, job);
}

int32_t io_pool_submit_read(int32_t pool, int32_t fd, void *buf, int64_t nbytes, int32_t promise) {
    IoPool *p = pool_get(pool);
    if (!p || fd < 0 || promise <= 0 || !buf || nbytes <= 0) {
        return -1;
    }
    IoJob job = {0};
    job.active = 1;
    job.kind = IO_JOB_READ;
    job.fd = fd;
    job.promise_id = promise;
    job.buf = buf;
    job.nbytes = nbytes;
    return pool_push(p, job);
}

int32_t io_pool_queue_depth(int32_t pool) {
    IoPool *p = pool_get(pool);
    if (!p) {
        return -1;
    }
#if defined(_WIN32)
    EnterCriticalSection(&p->mu);
    int n = p->q_count;
    LeaveCriticalSection(&p->mu);
#else
    pthread_mutex_lock(&p->mu);
    int n = p->q_count;
    pthread_mutex_unlock(&p->mu);
#endif
    return n;
}
