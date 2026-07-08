#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#if defined(_WIN32)
#include <process.h>
#include <windows.h>
#else
#include <pthread.h>
#endif

typedef void (*NyraSpawnBody)(void *);

typedef struct {
    NyraSpawnBody body;
    void *data;
} NyraSpawnJob;

typedef struct NyraJoinHandle {
#if defined(_WIN32)
    HANDLE thread;
#else
    pthread_t thread;
#endif
    int joined;
} NyraJoinHandle;

#if defined(_WIN32)
static unsigned __stdcall nyra_spawn_thread(void *arg) {
    NyraSpawnJob *job = (NyraSpawnJob *)arg;
    if (job && job->body) {
        job->body(job->data);
    }
    if (job) {
        free(job->data);
        free(job);
    }
    return 0;
}
#else
static void *nyra_spawn_thread(void *arg) {
    NyraSpawnJob *job = (NyraSpawnJob *)arg;
    if (job && job->body) {
        job->body(job->data);
    }
    if (job) {
        free(job->data);
        free(job);
    }
    return NULL;
}
#endif

static void spawn_run_job_inline(NyraSpawnJob *job) {
    if (job && job->body) {
        job->body(job->data);
    }
    if (job) {
        free(job->data);
        free(job);
    }
}

void *spawn_capture(void (*body)(void *), void *data, int64_t nbytes) {
    if (!body) {
        return NULL;
    }
    NyraJoinHandle *handle = (NyraJoinHandle *)calloc(1, sizeof(NyraJoinHandle));
    if (!handle) {
        return NULL;
    }
    NyraSpawnJob *job = (NyraSpawnJob *)calloc(1, sizeof(NyraSpawnJob));
    if (!job) {
        free(handle);
        return NULL;
    }
    job->body = body;
    if (data && nbytes > 0) {
        job->data = malloc((size_t)nbytes);
        if (!job->data) {
            free(job);
            free(handle);
            return NULL;
        }
        memcpy(job->data, data, (size_t)nbytes);
    } else {
        job->data = NULL;
    }
#if defined(_WIN32)
    handle->thread =
        (HANDLE)_beginthreadex(NULL, 0, nyra_spawn_thread, job, 0, NULL);
    if (!handle->thread) {
        spawn_run_job_inline(job);
        handle->joined = 1;
        return handle;
    }
#else
    if (pthread_create(&handle->thread, NULL, nyra_spawn_thread, job) != 0) {
        free(job->data);
        free(job);
        free(handle);
        return NULL;
    }
#endif
    return handle;
}

int spawn_join(void *handle) {
    NyraJoinHandle *jh = (NyraJoinHandle *)handle;
    if (!jh || jh->joined) {
        return -1;
    }
#if defined(_WIN32)
    if (!jh->thread) {
        jh->joined = 1;
        free(jh);
        return 0;
    }
    WaitForSingleObject(jh->thread, INFINITE);
    CloseHandle(jh->thread);
#else
    pthread_join(jh->thread, NULL);
#endif
    jh->joined = 1;
    free(jh);
    return 0;
}

void spawn_handle_drop(void *handle) {
    NyraJoinHandle *jh = (NyraJoinHandle *)handle;
    if (!jh || jh->joined) {
        return;
    }
#if defined(_WIN32)
    if (jh->thread) {
        CloseHandle(jh->thread);
    }
#else
    pthread_detach(jh->thread);
#endif
    jh->joined = 1;
    free(jh);
}
