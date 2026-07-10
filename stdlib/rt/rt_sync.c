#include <stdlib.h>
#include <string.h>

#if defined(_WIN32)
#include <windows.h>

typedef struct {
    CRITICAL_SECTION cs;
} NyraMutex;

typedef struct {
    SRWLOCK lock;
    int readers;
} NyraRwLock;

typedef struct {
    CRITICAL_SECTION cs;
    CONDITION_VARIABLE cv;
    int count;
} NyraWaitGroup;

void *mutex_new(void) {
    NyraMutex *m = (NyraMutex *)calloc(1, sizeof(NyraMutex));
    if (!m) {
        return NULL;
    }
    InitializeCriticalSection(&m->cs);
    return m;
}

void mutex_lock(void *m) {
    if (m) {
        EnterCriticalSection(&((NyraMutex *)m)->cs);
    }
}

void mutex_unlock(void *m) {
    if (m) {
        LeaveCriticalSection(&((NyraMutex *)m)->cs);
    }
}

void mutex_free(void *m) {
    if (!m) {
        return;
    }
    DeleteCriticalSection(&((NyraMutex *)m)->cs);
    free(m);
}

void *rwlock_new(void) {
    NyraRwLock *r = (NyraRwLock *)calloc(1, sizeof(NyraRwLock));
    if (!r) {
        return NULL;
    }
    InitializeSRWLock(&r->lock);
    return r;
}

void rwlock_rlock(void *r) {
    if (r) {
        AcquireSRWLockShared(&((NyraRwLock *)r)->lock);
    }
}

void rwlock_wlock(void *r) {
    if (r) {
        AcquireSRWLockExclusive(&((NyraRwLock *)r)->lock);
    }
}

void rwlock_unlock(void *r) {
    if (!r) {
        return;
    }
    NyraRwLock *rw = (NyraRwLock *)r;
    if (rw->readers < 0) {
        ReleaseSRWLockExclusive(&rw->lock);
        rw->readers = 0;
    } else {
        ReleaseSRWLockShared(&rw->lock);
    }
}

void rwlock_free(void *r) {
    free(r);
}

void *waitgroup_new(void) {
    NyraWaitGroup *wg = (NyraWaitGroup *)calloc(1, sizeof(NyraWaitGroup));
    if (!wg) {
        return NULL;
    }
    InitializeCriticalSection(&wg->cs);
    InitializeConditionVariable(&wg->cv);
    return wg;
}

void waitgroup_add(void *wg, int delta) {
    if (!wg) {
        return;
    }
    NyraWaitGroup *w = (NyraWaitGroup *)wg;
    EnterCriticalSection(&w->cs);
    w->count += delta;
    if (w->count < 0) {
        w->count = 0;
    }
    LeaveCriticalSection(&w->cs);
}

void waitgroup_done(void *wg) {
    if (!wg) {
        return;
    }
    NyraWaitGroup *w = (NyraWaitGroup *)wg;
    EnterCriticalSection(&w->cs);
    if (w->count > 0) {
        w->count--;
    }
    if (w->count == 0) {
        WakeAllConditionVariable(&w->cv);
    }
    LeaveCriticalSection(&w->cs);
}

void waitgroup_wait(void *wg) {
    if (!wg) {
        return;
    }
    NyraWaitGroup *w = (NyraWaitGroup *)wg;
    EnterCriticalSection(&w->cs);
    while (w->count > 0) {
        SleepConditionVariableCS(&w->cv, &w->cs, INFINITE);
    }
    LeaveCriticalSection(&w->cs);
}

void waitgroup_free(void *wg) {
    if (!wg) {
        return;
    }
    DeleteCriticalSection(&((NyraWaitGroup *)wg)->cs);
    free(wg);
}

int atomic_load_i32(int *p) {
    if (!p) {
        return 0;
    }
    return (int)InterlockedOr((volatile LONG *)p, 0);
}

void atomic_store_i32(int *p, int v) {
    if (p) {
        InterlockedExchange((volatile LONG *)p, (LONG)v);
    }
}

int atomic_add_i32(int *p, int delta) {
    if (!p) {
        return 0;
    }
    return (int)InterlockedAdd((volatile LONG *)p, (LONG)delta);
}

int atomic_cas_i32(int *p, int expected, int desired) {
    if (!p) {
        return 0;
    }
    LONG prev = InterlockedCompareExchange((volatile LONG *)p, (LONG)desired, (LONG)expected);
    return prev == (LONG)expected ? 1 : 0;
}

void *atomic_i32_new(int initial) {
    int *p = (int *)malloc(sizeof(int));
    if (p) {
        *p = initial;
    }
    return p;
}

void atomic_i32_free(void *p) {
    free(p);
}

#else
#include <pthread.h>

typedef struct {
    pthread_mutex_t mu;
} NyraMutex;

typedef struct {
    pthread_rwlock_t rw;
    int writer;
} NyraRwLock;

typedef struct {
    pthread_mutex_t mu;
    pthread_cond_t cv;
    int count;
} NyraWaitGroup;

void *mutex_new(void) {
    NyraMutex *m = (NyraMutex *)calloc(1, sizeof(NyraMutex));
    if (!m) {
        return NULL;
    }
    pthread_mutex_init(&m->mu, NULL);
    return m;
}

void mutex_lock(void *m) {
    if (m) {
        pthread_mutex_lock(&((NyraMutex *)m)->mu);
    }
}

void mutex_unlock(void *m) {
    if (m) {
        pthread_mutex_unlock(&((NyraMutex *)m)->mu);
    }
}

void mutex_free(void *m) {
    if (!m) {
        return;
    }
    pthread_mutex_destroy(&((NyraMutex *)m)->mu);
    free(m);
}

void *rwlock_new(void) {
    NyraRwLock *r = (NyraRwLock *)calloc(1, sizeof(NyraRwLock));
    if (!r) {
        return NULL;
    }
    pthread_rwlock_init(&r->rw, NULL);
    return r;
}

void rwlock_rlock(void *r) {
    if (r) {
        pthread_rwlock_rdlock(&((NyraRwLock *)r)->rw);
        ((NyraRwLock *)r)->writer = 0;
    }
}

void rwlock_wlock(void *r) {
    if (r) {
        pthread_rwlock_wrlock(&((NyraRwLock *)r)->rw);
        ((NyraRwLock *)r)->writer = 1;
    }
}

void rwlock_unlock(void *r) {
    if (r) {
        pthread_rwlock_unlock(&((NyraRwLock *)r)->rw);
    }
}

void rwlock_free(void *r) {
    if (!r) {
        return;
    }
    pthread_rwlock_destroy(&((NyraRwLock *)r)->rw);
    free(r);
}

void *waitgroup_new(void) {
    NyraWaitGroup *wg = (NyraWaitGroup *)calloc(1, sizeof(NyraWaitGroup));
    if (!wg) {
        return NULL;
    }
    pthread_mutex_init(&wg->mu, NULL);
    pthread_cond_init(&wg->cv, NULL);
    return wg;
}

void waitgroup_add(void *wg, int delta) {
    if (!wg) {
        return;
    }
    NyraWaitGroup *w = (NyraWaitGroup *)wg;
    pthread_mutex_lock(&w->mu);
    w->count += delta;
    if (w->count < 0) {
        w->count = 0;
    }
    pthread_mutex_unlock(&w->mu);
}

void waitgroup_done(void *wg) {
    if (!wg) {
        return;
    }
    NyraWaitGroup *w = (NyraWaitGroup *)wg;
    pthread_mutex_lock(&w->mu);
    if (w->count > 0) {
        w->count--;
    }
    if (w->count == 0) {
        pthread_cond_broadcast(&w->cv);
    }
    pthread_mutex_unlock(&w->mu);
}

void waitgroup_wait(void *wg) {
    if (!wg) {
        return;
    }
    NyraWaitGroup *w = (NyraWaitGroup *)wg;
    pthread_mutex_lock(&w->mu);
    while (w->count > 0) {
        pthread_cond_wait(&w->cv, &w->mu);
    }
    pthread_mutex_unlock(&w->mu);
}

void waitgroup_free(void *wg) {
    if (!wg) {
        return;
    }
    NyraWaitGroup *w = (NyraWaitGroup *)wg;
    pthread_mutex_destroy(&w->mu);
    pthread_cond_destroy(&w->cv);
    free(w);
}

int atomic_load_i32(int *p) {
    if (!p) {
        return 0;
    }
    return __atomic_load_n(p, __ATOMIC_SEQ_CST);
}

void atomic_store_i32(int *p, int v) {
    if (p) {
        __atomic_store_n(p, v, __ATOMIC_SEQ_CST);
    }
}

int atomic_add_i32(int *p, int delta) {
    if (!p) {
        return 0;
    }
    return __atomic_add_fetch(p, delta, __ATOMIC_SEQ_CST);
}

int atomic_cas_i32(int *p, int expected, int desired) {
    if (!p) {
        return 0;
    }
    return __atomic_compare_exchange_n(p, &expected, desired, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST) ? 1 : 0;
}

void *atomic_i32_new(int initial) {
    int *p = (int *)malloc(sizeof(int));
    if (p) {
        *p = initial;
    }
    return p;
}

void atomic_i32_free(void *p) {
    free(p);
}

#endif
// [contrib-dev:atomic_sub_i32:sync_atomic]
int atomic_sub_i32(void * p, int delta) {
    int *cell = (int *)p;
    if (!cell) return 0;
    return __atomic_sub_fetch(cell, delta, __ATOMIC_SEQ_CST);
}
// [/contrib-dev:atomic_sub_i32:sync_atomic]

// [contrib-dev:atomic_xor_i32:sync_atomic]
int atomic_xor_i32(void * p, int mask) {
    int *cell = (int *)p;
    if (!cell) return 0;
    return __atomic_xor_fetch(cell, mask, __ATOMIC_SEQ_CST);
}
// [/contrib-dev:atomic_xor_i32:sync_atomic]

// [contrib-dev:atomic_and_i32:sync_atomic]
int atomic_and_i32(void * p, int mask) {
    int *cell = (int *)p;
    if (!cell) return 0;
    return __atomic_and_fetch(cell, mask, __ATOMIC_SEQ_CST);
}
// [/contrib-dev:atomic_and_i32:sync_atomic]

// [contrib-dev:atomic_or_i32:sync_atomic]
int atomic_or_i32(void * p, int mask) {
    int *cell = (int *)p;
    if (!cell) return 0;
    return __atomic_or_fetch(cell, mask, __ATOMIC_SEQ_CST);
}
// [/contrib-dev:atomic_or_i32:sync_atomic]

