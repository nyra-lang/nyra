#include <stdlib.h>
#include <stdint.h>
#include <stddef.h>

#define NYRA_FUTURE_I32 0
#define NYRA_FUTURE_BOOL 1
#define NYRA_FUTURE_PTR 2

int async_future_done(int handle);
void *async_future_ptr_value(int handle);

#if defined(_WIN32)
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <winsock2.h>
#include <windows.h>
extern void nyra_winsock_ensure(void);

typedef struct {
    int done;
    int kind;
    intptr_t result;
    CRITICAL_SECTION cs;
    CONDITION_VARIABLE cv;
} NyraTask;

static CRITICAL_SECTION g_table_cs;
static int g_table_init = 0;

static void table_lock_init(void) {
    if (!g_table_init) {
        InitializeCriticalSection(&g_table_cs);
        g_table_init = 1;
    }
}

#define TASK_LOCK(t) EnterCriticalSection(&(t)->cs)
#define TASK_UNLOCK(t) LeaveCriticalSection(&(t)->cs)
#define TASK_WAIT(t) \
    while (!(t)->done) { \
        SleepConditionVariableCS(&(t)->cv, &(t)->cs, INFINITE); \
    }
#define TASK_SIGNAL(t) WakeAllConditionVariable(&(t)->cv)
#define TASK_INIT(t) \
    InitializeCriticalSection(&(t)->cs); \
    InitializeConditionVariable(&(t)->cv)

#define IO_LOCK() EnterCriticalSection(&g_io_cs)
#define IO_UNLOCK() LeaveCriticalSection(&g_io_cs)

static CRITICAL_SECTION g_io_cs;
static int g_io_cs_init = 0;

static void io_lock_init(void) {
    if (!g_io_cs_init) {
        InitializeCriticalSection(&g_io_cs);
        g_io_cs_init = 1;
    }
}

static int64_t nyra_now_ms(void) {
    return (int64_t)GetTickCount64();
}

#else
#include <pthread.h>
#include <time.h>

#ifdef __APPLE__
#include <sys/event.h>
#include <sys/time.h>
#elif defined(__linux__)
#include <sys/epoll.h>
#endif

typedef struct {
    int done;
    int kind;
    intptr_t result;
    pthread_mutex_t mu;
    pthread_cond_t cv;
} NyraTask;

static pthread_mutex_t g_table_mu = PTHREAD_MUTEX_INITIALIZER;

#define TASK_LOCK(t) pthread_mutex_lock(&(t)->mu)
#define TASK_UNLOCK(t) pthread_mutex_unlock(&(t)->mu)
#define TASK_WAIT(t) \
    while (!(t)->done) { \
        pthread_cond_wait(&(t)->cv, &(t)->mu); \
    }
#define TASK_SIGNAL(t) pthread_cond_broadcast(&(t)->cv)
#define TASK_INIT(t) \
    pthread_mutex_init(&(t)->mu, NULL); \
    pthread_cond_init(&(t)->cv, NULL)

#define IO_LOCK() pthread_mutex_lock(&g_io_mu)
#define IO_UNLOCK() pthread_mutex_unlock(&g_io_mu)

static int64_t nyra_now_ms(void) {
    struct timespec ts;
#if defined(CLOCK_MONOTONIC)
    clock_gettime(CLOCK_MONOTONIC, &ts);
#else
    clock_gettime(CLOCK_REALTIME, &ts);
#endif
    return (int64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

#endif

void *spawn_capture(void (*body)(void *), void *data, long long nbytes);
int spawn_join(void *handle);
void spawn_handle_drop(void *handle);
int io_wait_once(int timeout_ms);

#define NYRA_MAX_TASKS 4096
#define NYRA_MAX_TIMERS 128

static NyraTask *g_tasks[NYRA_MAX_TASKS];
static int g_next_task = 1;

static NyraTask *task_get(int id) {
    if (id <= 0 || id >= NYRA_MAX_TASKS) {
        return NULL;
    }
    return g_tasks[id];
}

int async_promise_new(void);
void async_promise_complete(int handle, int value);
int async_poll(int handle);

typedef struct {
    int task_id;
    int64_t deadline_ms;
    int value;
    int active;
} NyraTimer;

static NyraTimer g_timers[NYRA_MAX_TIMERS];

#ifdef __APPLE__
static int g_kq = -1;
static pthread_mutex_t g_io_mu = PTHREAD_MUTEX_INITIALIZER;
#elif defined(__linux__)
static int g_epoll = -1;
static pthread_mutex_t g_io_mu = PTHREAD_MUTEX_INITIALIZER;
#elif defined(_WIN32)
#define NYRA_MAX_IO 64
static struct {
    int fd;
    int task_id;
    int active;
} g_io_tab[NYRA_MAX_IO];
static int g_io_n = 0;
#endif

static void timers_lock(void) {
#if defined(_WIN32)
    table_lock_init();
    EnterCriticalSection(&g_table_cs);
#else
    pthread_mutex_lock(&g_table_mu);
#endif
}

static void timers_unlock(void) {
#if defined(_WIN32)
    LeaveCriticalSection(&g_table_cs);
#else
    pthread_mutex_unlock(&g_table_mu);
#endif
}

static int process_timers(void) {
    int64_t now = nyra_now_ms();
    int fired = 0;
    int ids[NYRA_MAX_TIMERS];
    int values[NYRA_MAX_TIMERS];
    int pending = 0;

    timers_lock();
    for (int i = 0; i < NYRA_MAX_TIMERS; i++) {
        if (!g_timers[i].active) {
            continue;
        }
        if (now >= g_timers[i].deadline_ms) {
            ids[pending] = g_timers[i].task_id;
            values[pending] = g_timers[i].value;
            pending++;
            g_timers[i].active = 0;
        }
    }
    timers_unlock();

    for (int i = 0; i < pending; i++) {
        async_promise_complete(ids[i], values[i]);
        fired++;
    }
    return fired;
}

static int register_timer(int task_id, int delay_ms) {
    if (task_id <= 0 || delay_ms < 0) {
        return -1;
    }
    int64_t deadline = nyra_now_ms() + (int64_t)delay_ms;
    timers_lock();
    for (int i = 0; i < NYRA_MAX_TIMERS; i++) {
        if (!g_timers[i].active) {
            g_timers[i].task_id = task_id;
            g_timers[i].deadline_ms = deadline;
            g_timers[i].value = delay_ms;
            g_timers[i].active = 1;
            timers_unlock();
            return 0;
        }
    }
    timers_unlock();
    return -1;
}

#if defined(_WIN32)
static void executor_yield_ms(int timeout_ms) {
    if (timeout_ms > 0) {
        Sleep((DWORD)timeout_ms);
    }
}
#else
static void executor_yield_ms(int timeout_ms) {
    if (timeout_ms <= 0) {
        return;
    }
    struct timespec ts;
    ts.tv_sec = timeout_ms / 1000;
    ts.tv_nsec = (long)(timeout_ms % 1000) * 1000000L;
    nanosleep(&ts, NULL);
}
#endif

int runtime_executor_tick(int timeout_ms) {
    int io = io_wait_once(timeout_ms);
    int timers = process_timers();
    int work = io + timers;
    /* Cooperative poll loops call tick with no registered I/O; yield so spawn
     * threads and other waiters can run (Windows CI could spin forever otherwise). */
    if (work == 0 && timeout_ms > 0) {
        executor_yield_ms(timeout_ms);
    }
    return work;
}

int runtime_executor_run_until(int handle, int timeout_ms) {
    if (handle <= 0) {
        return -1;
    }
    int64_t start = nyra_now_ms();
    for (;;) {
        int r = async_poll(handle);
        if (r >= 0) {
            return r;
        }
        if (timeout_ms >= 0 && nyra_now_ms() - start >= (int64_t)timeout_ms) {
            return -1;
        }
        int slice = 10;
        if (timeout_ms >= 0) {
            int64_t left = (int64_t)timeout_ms - (nyra_now_ms() - start);
            if (left <= 0) {
                return -1;
            }
            if (left < slice) {
                slice = (int)left;
            }
        }
        runtime_executor_tick(slice);
    }
}

int async_sleep_ms(int delay_ms) {
    int h = async_promise_new();
    if (h == 0) {
        return 0;
    }
    if (delay_ms <= 0) {
        async_promise_complete(h, 0);
        return h;
    }
    if (register_timer(h, delay_ms) != 0) {
        async_promise_complete(h, 0);
    }
    return h;
}

static int alloc_task(void) {
#if defined(_WIN32)
    table_lock_init();
#endif
    NyraTask *t = (NyraTask *)calloc(1, sizeof(NyraTask));
    if (!t) {
        return 0;
    }
    TASK_INIT(t);

#if defined(_WIN32)
    EnterCriticalSection(&g_table_cs);
#else
    pthread_mutex_lock(&g_table_mu);
#endif
    int id = g_next_task++;
    if (g_next_task >= NYRA_MAX_TASKS) {
        g_next_task = 1;
    }
    if (g_tasks[id]) {
        free(g_tasks[id]);
    }
    g_tasks[id] = t;
#if defined(_WIN32)
    LeaveCriticalSection(&g_table_cs);
#else
    pthread_mutex_unlock(&g_table_mu);
#endif
    return id;
}

int async_promise_new(void) {
    return alloc_task();
}

static void promise_complete_task(NyraTask *t, int kind, intptr_t value) {
    TASK_LOCK(t);
    t->kind = kind;
    t->result = value;
    t->done = 1;
    TASK_SIGNAL(t);
    TASK_UNLOCK(t);
}

void async_promise_complete(int handle, int value) {
    NyraTask *t = task_get(handle);
    if (!t) {
        return;
    }
    promise_complete_task(t, NYRA_FUTURE_I32, (intptr_t)value);
}

void async_promise_complete_bool(int handle, int value) {
    NyraTask *t = task_get(handle);
    if (!t) {
        return;
    }
    promise_complete_task(t, NYRA_FUTURE_BOOL, value ? 1 : 0);
}

void async_promise_complete_ptr(int handle, void *value) {
    NyraTask *t = task_get(handle);
    if (!t) {
        return;
    }
    promise_complete_task(t, NYRA_FUTURE_PTR, (intptr_t)value);
}

static int task_poll_i32(NyraTask *t) {
    if (!t->done) {
        return -1;
    }
    if (t->kind == NYRA_FUTURE_BOOL) {
        return (int)t->result;
    }
    if (t->kind == NYRA_FUTURE_PTR) {
        return -1;
    }
    return (int)t->result;
}

static int task_poll_bool(NyraTask *t) {
    if (!t->done || t->kind != NYRA_FUTURE_BOOL) {
        return -1;
    }
    return (int)t->result;
}

static void *task_poll_ptr(NyraTask *t) {
    if (!t->done || t->kind != NYRA_FUTURE_PTR) {
        return NULL;
    }
    return (void *)t->result;
}

static void task_wait(NyraTask *t) {
    TASK_LOCK(t);
    while (!t->done) {
        TASK_UNLOCK(t);
        runtime_executor_tick(10);
        TASK_LOCK(t);
        if (t->done) {
            break;
        }
#if defined(_WIN32)
        SleepConditionVariableCS(&t->cv, &t->cs, 5);
#else
        struct timespec ts;
        clock_gettime(CLOCK_REALTIME, &ts);
        ts.tv_nsec += 5000000L;
        if (ts.tv_nsec >= 1000000000L) {
            ts.tv_sec += 1;
            ts.tv_nsec -= 1000000000L;
        }
        pthread_cond_timedwait(&t->cv, &t->mu, &ts);
#endif
    }
    TASK_UNLOCK(t);
}

int async_await(int handle) {
    NyraTask *t = task_get(handle);
    if (!t) {
        return 0;
    }
    task_wait(t);
    TASK_LOCK(t);
    int r = (int)t->result;
    TASK_UNLOCK(t);
    return r;
}

int async_await_bool(int handle) {
    NyraTask *t = task_get(handle);
    if (!t) {
        return 0;
    }
    task_wait(t);
    TASK_LOCK(t);
    int r = (t->kind == NYRA_FUTURE_BOOL) ? (int)t->result : (int)t->result;
    TASK_UNLOCK(t);
    return r;
}

void *async_await_ptr(int handle) {
    NyraTask *t = task_get(handle);
    if (!t) {
        return NULL;
    }
    task_wait(t);
    TASK_LOCK(t);
    void *r = (t->kind == NYRA_FUTURE_PTR) ? (void *)t->result : (void *)(intptr_t)t->result;
    TASK_UNLOCK(t);
    return r;
}

int async_poll(int handle) {
    NyraTask *t = task_get(handle);
    if (!t) {
        return -1;
    }
    TASK_LOCK(t);
    int r = task_poll_i32(t);
    TASK_UNLOCK(t);
    return r;
}

int async_poll_bool(int handle) {
    NyraTask *t = task_get(handle);
    if (!t) {
        return -1;
    }
    TASK_LOCK(t);
    int r = task_poll_bool(t);
    TASK_UNLOCK(t);
    return r;
}

void *async_poll_ptr(int handle) {
    if (!async_future_done(handle)) {
        return NULL;
    }
    return async_future_ptr_value(handle);
}

static int select_poll_i32(int handle, int *out_index, int slot) {
    int r = async_poll(handle);
    if (r >= 0) {
        *out_index = slot;
        return r;
    }
    return -1;
}

static int select_poll_bool(int handle, int *out_index, int slot) {
    int r = async_poll_bool(handle);
    if (r >= 0) {
        *out_index = slot;
        return r;
    }
    return -1;
}

static int future_is_done(int handle) {
    NyraTask *t = task_get(handle);
    if (!t) {
        return 0;
    }
    TASK_LOCK(t);
    int done = t->done;
    TASK_UNLOCK(t);
    return done;
}

int async_future_done(int handle) {
    return future_is_done(handle);
}

void *async_future_ptr_value(int handle) {
    NyraTask *t = task_get(handle);
    if (!t) {
        return NULL;
    }
    TASK_LOCK(t);
    void *r = (t->done && t->kind == NYRA_FUTURE_PTR) ? (void *)t->result : NULL;
    TASK_UNLOCK(t);
    return r;
}

static int select_poll_ptr(int handle, int *out_index, int slot, void **out_ptr) {
    NyraTask *t = task_get(handle);
    if (!t) {
        return 0;
    }
    TASK_LOCK(t);
    if (!t->done || t->kind != NYRA_FUTURE_PTR) {
        TASK_UNLOCK(t);
        return 0;
    }
    *out_index = slot;
    *out_ptr = (void *)t->result;
    TASK_UNLOCK(t);
    return 1;
}

int async_select2_i32(int h0, int h1, int *out_index) {
    if (!out_index || h0 <= 0 || h1 <= 0) {
        return -1;
    }
    for (;;) {
        int r = select_poll_i32(h0, out_index, 0);
        if (r >= 0) {
            return r;
        }
        r = select_poll_i32(h1, out_index, 1);
        if (r >= 0) {
            return r;
        }
        runtime_executor_tick(10);
    }
}

int async_select2_bool(int h0, int h1, int *out_index) {
    if (!out_index || h0 <= 0 || h1 <= 0) {
        return -1;
    }
    for (;;) {
        int r = select_poll_bool(h0, out_index, 0);
        if (r >= 0) {
            return r;
        }
        r = select_poll_bool(h1, out_index, 1);
        if (r >= 0) {
            return r;
        }
        runtime_executor_tick(10);
    }
}

void *async_select2_ptr(int h0, int h1, int *out_index) {
    if (!out_index || h0 <= 0 || h1 <= 0) {
        return NULL;
    }
    for (;;) {
        void *r = NULL;
        if (select_poll_ptr(h0, out_index, 0, &r)) {
            return r;
        }
        if (select_poll_ptr(h1, out_index, 1, &r)) {
            return r;
        }
        runtime_executor_tick(10);
    }
}

int async_select_i32(int *handles, int count, int *out_index) {
    if (!handles || count <= 0 || !out_index) {
        return -1;
    }
    for (;;) {
        for (int i = 0; i < count; i++) {
            int r = select_poll_i32(handles[i], out_index, i);
            if (r >= 0) {
                return r;
            }
        }
        runtime_executor_tick(10);
    }
}

int async_run(int result) {
    int h = async_promise_new();
    if (h == 0) {
        return 0;
    }
    async_promise_complete(h, result);
    return h;
}

void runtime_run(void) {
    runtime_executor_tick(0);
}

int runtime_poll_io(int timeout_ms) {
    return runtime_executor_tick(timeout_ms);
}

int io_register(int fd, int task_id) {
#ifdef __APPLE__
    IO_LOCK();
    if (g_kq < 0) {
        g_kq = kqueue();
    }
    struct kevent ev;
    EV_SET(&ev, (uintptr_t)fd, EVFILT_READ, EV_ADD, 0, 0, (void *)(intptr_t)task_id);
    int ok = kevent(g_kq, &ev, 1, NULL, 0, NULL) == 0 ? 0 : -1;
    IO_UNLOCK();
    return ok;
#elif defined(__linux__)
    IO_LOCK();
    if (g_epoll < 0) {
        g_epoll = epoll_create1(0);
    }
    struct epoll_event ev;
    ev.events = EPOLLIN;
    ev.data.u32 = (uint32_t)task_id;
    int ok = epoll_ctl(g_epoll, EPOLL_CTL_ADD, fd, &ev) == 0 ? 0 : -1;
    IO_UNLOCK();
    return ok;
#elif defined(_WIN32)
    if (fd < 0 || task_id <= 0) {
        return -1;
    }
    nyra_winsock_ensure();
    io_lock_init();
    IO_LOCK();
    for (int i = 0; i < g_io_n; i++) {
        if (g_io_tab[i].fd == fd) {
            g_io_tab[i].task_id = task_id;
            g_io_tab[i].active = 1;
            IO_UNLOCK();
            return 0;
        }
    }
    if (g_io_n >= NYRA_MAX_IO) {
        IO_UNLOCK();
        return -1;
    }
    g_io_tab[g_io_n].fd = fd;
    g_io_tab[g_io_n].task_id = task_id;
    g_io_tab[g_io_n].active = 1;
    g_io_n++;
    IO_UNLOCK();
    return 0;
#else
    (void)fd;
    (void)task_id;
    return -1;
#endif
}

int io_unregister(int fd) {
#ifdef __APPLE__
    if (g_kq < 0 || fd < 0) {
        return -1;
    }
    struct kevent ev;
    EV_SET(&ev, (uintptr_t)fd, EVFILT_READ, EV_DELETE, 0, 0, NULL);
    return kevent(g_kq, &ev, 1, NULL, 0, NULL) == 0 ? 0 : -1;
#elif defined(__linux__)
    if (g_epoll < 0 || fd < 0) {
        return -1;
    }
    struct epoll_event ev;
    return epoll_ctl(g_epoll, EPOLL_CTL_DEL, fd, &ev) == 0 ? 0 : -1;
#elif defined(_WIN32)
    io_lock_init();
    IO_LOCK();
    for (int i = 0; i < g_io_n; i++) {
        if (g_io_tab[i].fd == fd) {
            g_io_tab[i].active = 0;
            IO_UNLOCK();
            return 0;
        }
    }
    IO_UNLOCK();
    return -1;
#else
    (void)fd;
    return -1;
#endif
}

int io_wait_once(int timeout_ms) {
#ifdef __APPLE__
    if (g_kq < 0) {
        return 0;
    }
    struct kevent ev;
    struct timespec ts;
    ts.tv_sec = timeout_ms / 1000;
    ts.tv_nsec = (long)(timeout_ms % 1000) * 1000000L;
    int n = kevent(g_kq, NULL, 0, &ev, 1, timeout_ms > 0 ? &ts : NULL);
    if (n <= 0) {
        return 0;
    }
    int task_id = (int)(intptr_t)ev.udata;
    async_promise_complete(task_id, (int)ev.ident);
    return 1;
#elif defined(__linux__)
    extern int io_uring_pending(void);
    extern int io_uring_wait_once(int timeout_ms);
    if (io_uring_pending() > 0) {
        int uring_fired = io_uring_wait_once(timeout_ms);
        if (uring_fired > 0) {
            return uring_fired;
        }
    }
    if (g_epoll < 0) {
        return 0;
    }
    struct epoll_event ev;
    int n = epoll_wait(g_epoll, &ev, 1, timeout_ms);
    if (n <= 0) {
        return 0;
    }
    async_promise_complete((int)ev.data.u32, 1);
    return 1;
#elif defined(_WIN32)
    io_lock_init();
    IO_LOCK();
    fd_set rfds;
    FD_ZERO(&rfds);
    SOCKET max_sock = 0;
    int count = 0;
    for (int i = 0; i < g_io_n; i++) {
        if (g_io_tab[i].active) {
            FD_SET((SOCKET)g_io_tab[i].fd, &rfds);
            if ((SOCKET)g_io_tab[i].fd > max_sock) {
                max_sock = (SOCKET)g_io_tab[i].fd;
            }
            count++;
        }
    }
    IO_UNLOCK();
    if (count == 0) {
        return 0;
    }
    nyra_winsock_ensure();
    struct timeval tv;
    struct timeval *ptv = NULL;
    if (timeout_ms > 0) {
        tv.tv_sec = timeout_ms / 1000;
        tv.tv_usec = (timeout_ms % 1000) * 1000;
        ptv = &tv;
    }
    int r = select((int)max_sock + 1, &rfds, NULL, NULL, ptv);
    if (r <= 0) {
        return 0;
    }
    IO_LOCK();
    for (int i = 0; i < g_io_n; i++) {
        if (g_io_tab[i].active && FD_ISSET((SOCKET)g_io_tab[i].fd, &rfds)) {
            int tid = g_io_tab[i].task_id;
            int ready_fd = g_io_tab[i].fd;
            g_io_tab[i].active = 0;
            IO_UNLOCK();
            async_promise_complete(tid, ready_fd);
            return 1;
        }
    }
    IO_UNLOCK();
    return 0;
#else
    (void)timeout_ms;
    return 0;
#endif
}

static void nyra_spawn_noop(void *data) {
    (void)data;
}

void spawn(void) {
    void *h = spawn_capture(nyra_spawn_noop, NULL, 0);
    if (h) {
        spawn_handle_drop(h);
    }
}
