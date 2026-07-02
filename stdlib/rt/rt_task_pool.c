// Global lightweight task pool — `spawn` / `spawn:task` (Tokio-style worker pool).
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#if defined(_WIN32)
#include <windows.h>
#else
#include <pthread.h>
#endif

typedef void (*NyraSpawnBody)(void *);

int32_t cpu_count(void);

#define NYRA_TASK_QUEUE_CAP 65536
#define NYRA_TASK_MAX_WORKERS 64

typedef struct NyraTaskJob {
    NyraSpawnBody body;
    void *data;
    int state; /* 0=queued, 1=running, 2=done */
    int detached;
#if defined(_WIN32)
    CRITICAL_SECTION mu;
    CONDITION_VARIABLE done;
#else
    pthread_mutex_t mu;
    pthread_cond_t done;
#endif
} NyraTaskJob;

typedef struct NyraTaskHandle {
    NyraTaskJob *job;
    int joined;
} NyraTaskHandle;

typedef struct {
    int inited;
    int shutdown;
    int n_workers;
#if defined(_WIN32)
    HANDLE *workers;
    CRITICAL_SECTION q_mu;
    CONDITION_VARIABLE q_not_empty;
#else
    pthread_t *workers;
    pthread_mutex_t q_mu;
    pthread_cond_t q_not_empty;
#endif
    NyraTaskJob *queue[NYRA_TASK_QUEUE_CAP];
    int q_head;
    int q_tail;
    int q_count;
} TaskPool;

static TaskPool g_task_pool;

static void job_init_sync(NyraTaskJob *job) {
#if defined(_WIN32)
    InitializeCriticalSection(&job->mu);
    InitializeConditionVariable(&job->done);
#else
    pthread_mutex_init(&job->mu, NULL);
    pthread_cond_init(&job->done, NULL);
#endif
}

static void job_destroy_sync(NyraTaskJob *job) {
#if defined(_WIN32)
    DeleteCriticalSection(&job->mu);
#else
    pthread_mutex_destroy(&job->mu);
    pthread_cond_destroy(&job->done);
#endif
}

static void job_free(NyraTaskJob *job) {
    if (!job) {
        return;
    }
    job_destroy_sync(job);
    free(job);
}

static int pool_push(NyraTaskJob *job) {
#if defined(_WIN32)
    EnterCriticalSection(&g_task_pool.q_mu);
#else
    pthread_mutex_lock(&g_task_pool.q_mu);
#endif
    if (g_task_pool.q_count >= NYRA_TASK_QUEUE_CAP) {
#if defined(_WIN32)
        LeaveCriticalSection(&g_task_pool.q_mu);
#else
        pthread_mutex_unlock(&g_task_pool.q_mu);
#endif
        return -1;
    }
    g_task_pool.queue[g_task_pool.q_tail] = job;
    g_task_pool.q_tail = (g_task_pool.q_tail + 1) % NYRA_TASK_QUEUE_CAP;
    g_task_pool.q_count++;
#if defined(_WIN32)
    WakeAllConditionVariable(&g_task_pool.q_not_empty);
    LeaveCriticalSection(&g_task_pool.q_mu);
#else
    pthread_cond_broadcast(&g_task_pool.q_not_empty);
    pthread_mutex_unlock(&g_task_pool.q_mu);
#endif
    return 0;
}

static NyraTaskJob *pool_pop(void) {
#if defined(_WIN32)
    EnterCriticalSection(&g_task_pool.q_mu);
    while (g_task_pool.q_count == 0 && !g_task_pool.shutdown) {
        SleepConditionVariableCS(&g_task_pool.q_not_empty, &g_task_pool.q_mu, INFINITE);
    }
    if (g_task_pool.q_count == 0) {
        LeaveCriticalSection(&g_task_pool.q_mu);
        return NULL;
    }
#else
    pthread_mutex_lock(&g_task_pool.q_mu);
    while (g_task_pool.q_count == 0 && !g_task_pool.shutdown) {
        pthread_cond_wait(&g_task_pool.q_not_empty, &g_task_pool.q_mu);
    }
    if (g_task_pool.q_count == 0) {
        pthread_mutex_unlock(&g_task_pool.q_mu);
        return NULL;
    }
#endif
    NyraTaskJob *job = g_task_pool.queue[g_task_pool.q_head];
    g_task_pool.q_head = (g_task_pool.q_head + 1) % NYRA_TASK_QUEUE_CAP;
    g_task_pool.q_count--;
#if defined(_WIN32)
    LeaveCriticalSection(&g_task_pool.q_mu);
#else
    pthread_mutex_unlock(&g_task_pool.q_mu);
#endif
    return job;
}

static void run_job(NyraTaskJob *job) {
    if (!job) {
        return;
    }
    job->state = 1;
    if (job->body) {
        job->body(job->data);
    }
    if (job->data) {
        free(job->data);
        job->data = NULL;
    }
#if defined(_WIN32)
    EnterCriticalSection(&job->mu);
    job->state = 2;
    WakeAllConditionVariable(&job->done);
    int detached = job->detached;
    LeaveCriticalSection(&job->mu);
#else
    pthread_mutex_lock(&job->mu);
    job->state = 2;
    pthread_cond_broadcast(&job->done);
    int detached = job->detached;
    pthread_mutex_unlock(&job->mu);
#endif
    if (detached) {
        job_free(job);
    }
}

#if defined(_WIN32)
static DWORD WINAPI task_worker_main(LPVOID arg) {
    (void)arg;
#else
static void *task_worker_main(void *arg) {
    (void)arg;
#endif
    for (;;) {
        NyraTaskJob *job = pool_pop();
        if (!job) {
            break;
        }
        run_job(job);
    }
#if defined(_WIN32)
    return 0;
#else
    return NULL;
#endif
}

static void task_pool_init(void) {
    if (g_task_pool.inited) {
        return;
    }
    memset(&g_task_pool, 0, sizeof(g_task_pool));
    int workers = (int)cpu_count();
    if (workers <= 0) {
        workers = 4;
    }
    if (workers > NYRA_TASK_MAX_WORKERS) {
        workers = NYRA_TASK_MAX_WORKERS;
    }
    g_task_pool.n_workers = workers;
#if defined(_WIN32)
    InitializeCriticalSection(&g_task_pool.q_mu);
    InitializeConditionVariable(&g_task_pool.q_not_empty);
    g_task_pool.workers = (HANDLE *)calloc((size_t)workers, sizeof(HANDLE));
#else
    pthread_mutex_init(&g_task_pool.q_mu, NULL);
    pthread_cond_init(&g_task_pool.q_not_empty, NULL);
    g_task_pool.workers = (pthread_t *)calloc((size_t)workers, sizeof(pthread_t));
#endif
    if (!g_task_pool.workers) {
        return;
    }
    for (int i = 0; i < workers; i++) {
#if defined(_WIN32)
        g_task_pool.workers[i] =
            CreateThread(NULL, 0, task_worker_main, NULL, 0, NULL);
        if (!g_task_pool.workers[i]) {
            g_task_pool.shutdown = 1;
            return;
        }
#else
        if (pthread_create(&g_task_pool.workers[i], NULL, task_worker_main, NULL) != 0) {
            g_task_pool.shutdown = 1;
            return;
        }
#endif
    }
    g_task_pool.inited = 1;
}

void *spawn_task_capture(void (*body)(void *), void *data, int64_t nbytes) {
    if (!body) {
        return NULL;
    }
    task_pool_init();
    if (!g_task_pool.inited) {
        return NULL;
    }
    NyraTaskJob *job = (NyraTaskJob *)calloc(1, sizeof(NyraTaskJob));
    if (!job) {
        return NULL;
    }
    job_init_sync(job);
    job->body = body;
    if (data && nbytes > 0) {
        job->data = malloc((size_t)nbytes);
        if (!job->data) {
            job_free(job);
            return NULL;
        }
        memcpy(job->data, data, (size_t)nbytes);
    }
    NyraTaskHandle *handle = (NyraTaskHandle *)calloc(1, sizeof(NyraTaskHandle));
    if (!handle) {
        job_free(job);
        return NULL;
    }
    handle->job = job;
    if (pool_push(job) != 0) {
        job_free(job);
        free(handle);
        return NULL;
    }
    return handle;
}

int spawn_task_join(void *handle) {
    NyraTaskHandle *th = (NyraTaskHandle *)handle;
    if (!th || th->joined || !th->job) {
        return -1;
    }
    NyraTaskJob *job = th->job;
#if defined(_WIN32)
    EnterCriticalSection(&job->mu);
    while (job->state != 2) {
        SleepConditionVariableCS(&job->done, &job->mu, INFINITE);
    }
    LeaveCriticalSection(&job->mu);
#else
    pthread_mutex_lock(&job->mu);
    while (job->state != 2) {
        pthread_cond_wait(&job->done, &job->mu);
    }
    pthread_mutex_unlock(&job->mu);
#endif
    th->joined = 1;
    th->job = NULL;
    job_free(job);
    free(th);
    return 0;
}

void spawn_task_handle_drop(void *handle) {
    NyraTaskHandle *th = (NyraTaskHandle *)handle;
    if (!th || th->joined) {
        return;
    }
    if (th->job) {
#if defined(_WIN32)
        EnterCriticalSection(&th->job->mu);
        th->job->detached = 1;
        int done = th->job->state == 2;
        LeaveCriticalSection(&th->job->mu);
#else
        pthread_mutex_lock(&th->job->mu);
        th->job->detached = 1;
        int done = th->job->state == 2;
        pthread_mutex_unlock(&th->job->mu);
#endif
        if (done) {
            job_free(th->job);
        }
    }
    th->joined = 1;
    th->job = NULL;
    free(th);
}
