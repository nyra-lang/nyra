// Linux io_uring fast path — poll-add registrations with epoll fallback elsewhere.
#include <stdint.h>

#if defined(__linux__)
#include <errno.h>
#include <linux/io_uring.h>
#include <poll.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <unistd.h>

#ifndef __NR_io_uring_setup
#if defined(__x86_64__)
#define __NR_io_uring_setup 425
#define __NR_io_uring_enter 426
#elif defined(__aarch64__)
#define __NR_io_uring_setup 425
#define __NR_io_uring_enter 426
#endif
#endif

#define NYRA_URING_ENTRIES 64
#define NYRA_URING_MAX_FD 128

typedef struct {
    int ring_fd;
    int ready;
    int pending;
    struct io_uring_params params;
    void *sq_ring_ptr;
    void *cq_ring_ptr;
    struct io_uring_sqe *sqes;
    unsigned *sq_khead;
    unsigned *sq_ktail;
    unsigned *sq_kring_mask;
    unsigned *sq_kring_entries;
    unsigned *sq_array;
    unsigned *cq_khead;
    unsigned *cq_ktail;
    unsigned *cq_kring_mask;
    unsigned *cq_koverflow;
    struct io_uring_cqe *cqes;
} NyraUring;

typedef struct {
    int fd;
    int promise;
    int active;
} NyraUringFd;

static int g_uring_available = -1;
static NyraUring g_uring;
static NyraUringFd g_uring_fds[NYRA_URING_MAX_FD];

static int probe_io_uring(void) {
    if (g_uring_available >= 0) {
        return g_uring_available;
    }
#if defined(__NR_io_uring_setup)
    struct io_uring_params params;
    memset(&params, 0, sizeof(params));
    int fd = (int)syscall(__NR_io_uring_setup, 1, &params);
    if (fd >= 0) {
        close(fd);
        g_uring_available = 1;
    } else {
        g_uring_available = 0;
    }
#else
    g_uring_available = 0;
#endif
    return g_uring_available;
}

static int uring_map_rings(void) {
    size_t sq_ring_sz = g_uring.params.sq_off.array + g_uring.params.sq_entries * sizeof(unsigned);
    size_t cq_ring_sz = g_uring.params.cq_off.cqes + g_uring.params.cq_entries * sizeof(struct io_uring_cqe);

    g_uring.sq_ring_ptr = mmap(NULL, sq_ring_sz, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_POPULATE, g_uring.ring_fd,
                               IORING_OFF_SQ_RING);
    if (g_uring.sq_ring_ptr == MAP_FAILED) {
        return -1;
    }
    g_uring.cq_ring_ptr = mmap(NULL, cq_ring_sz, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_POPULATE, g_uring.ring_fd,
                               IORING_OFF_CQ_RING);
    if (g_uring.cq_ring_ptr == MAP_FAILED) {
        munmap(g_uring.sq_ring_ptr, sq_ring_sz);
        g_uring.sq_ring_ptr = NULL;
        return -1;
    }
    g_uring.sqes = mmap(NULL, g_uring.params.sq_entries * sizeof(struct io_uring_sqe), PROT_READ | PROT_WRITE,
                        MAP_SHARED | MAP_POPULATE, g_uring.ring_fd, IORING_OFF_SQES);
    if (g_uring.sqes == MAP_FAILED) {
        munmap(g_uring.sq_ring_ptr, sq_ring_sz);
        munmap(g_uring.cq_ring_ptr, cq_ring_sz);
        g_uring.sq_ring_ptr = NULL;
        g_uring.cq_ring_ptr = NULL;
        return -1;
    }

    char *sq_base = (char *)g_uring.sq_ring_ptr;
    char *cq_base = (char *)g_uring.cq_ring_ptr;
    g_uring.sq_khead = (unsigned *)(sq_base + g_uring.params.sq_off.head);
    g_uring.sq_ktail = (unsigned *)(sq_base + g_uring.params.sq_off.tail);
    g_uring.sq_kring_mask = (unsigned *)(sq_base + g_uring.params.sq_off.ring_mask);
    g_uring.sq_kring_entries = (unsigned *)(sq_base + g_uring.params.sq_off.ring_entries);
    g_uring.sq_array = (unsigned *)(sq_base + g_uring.params.sq_off.array);
    g_uring.cq_khead = (unsigned *)(cq_base + g_uring.params.cq_off.head);
    g_uring.cq_ktail = (unsigned *)(cq_base + g_uring.params.cq_off.tail);
    g_uring.cq_kring_mask = (unsigned *)(cq_base + g_uring.params.cq_off.ring_mask);
    g_uring.cq_koverflow = (unsigned *)(cq_base + g_uring.params.cq_off.overflow);
    g_uring.cqes = (struct io_uring_cqe *)(cq_base + g_uring.params.cq_off.cqes);
    return 0;
}

static int uring_ensure_ready(void) {
    if (g_uring.ready) {
        return 0;
    }
    if (!probe_io_uring()) {
        return -1;
    }
    memset(&g_uring.params, 0, sizeof(g_uring.params));
    g_uring.ring_fd = (int)syscall(__NR_io_uring_setup, NYRA_URING_ENTRIES, &g_uring.params);
    if (g_uring.ring_fd < 0) {
        return -1;
    }
    if (uring_map_rings() != 0) {
        close(g_uring.ring_fd);
        memset(&g_uring, 0, sizeof(g_uring));
        return -1;
    }
    g_uring.ready = 1;
    return 0;
}

static NyraUringFd *uring_fd_slot(int fd, int create) {
    for (int i = 0; i < NYRA_URING_MAX_FD; i++) {
        if (g_uring_fds[i].active && g_uring_fds[i].fd == fd) {
            return &g_uring_fds[i];
        }
    }
    if (!create) {
        return NULL;
    }
    for (int i = 0; i < NYRA_URING_MAX_FD; i++) {
        if (!g_uring_fds[i].active) {
            return &g_uring_fds[i];
        }
    }
    return NULL;
}

static struct io_uring_sqe *uring_get_sqe(void) {
    unsigned head = *g_uring.sq_khead;
    unsigned tail = *g_uring.sq_ktail;
    if (tail + 1 - head > *g_uring.sq_kring_entries) {
        return NULL;
    }
    unsigned index = tail & *g_uring.sq_kring_mask;
    struct io_uring_sqe *sqe = &g_uring.sqes[g_uring.sq_array[index]];
    memset(sqe, 0, sizeof(*sqe));
    return sqe;
}

static int uring_flush_submit(void) {
    unsigned tail = *g_uring.sq_ktail;
    unsigned head = *g_uring.sq_khead;
    unsigned to_submit = tail - head;
    if (to_submit == 0) {
        return 0;
    }
    int rc = (int)syscall(__NR_io_uring_enter, g_uring.ring_fd, to_submit, 0, IORING_ENTER_GETEVENTS, NULL, 0);
    return rc < 0 ? -1 : 0;
}

static int uring_process_cq(void) {
    unsigned head = *g_uring.cq_khead;
    unsigned tail = *g_uring.cq_ktail;
    int fired = 0;
    while (head != tail) {
        struct io_uring_cqe *cqe = &g_uring.cqes[head & *g_uring.cq_kring_mask];
        int promise = (int)cqe->user_data;
        if (promise > 0 && cqe->res >= 0) {
            extern void async_promise_complete(int handle, int value);
            async_promise_complete(promise, 1);
            fired++;
            for (int i = 0; i < NYRA_URING_MAX_FD; i++) {
                if (g_uring_fds[i].active && g_uring_fds[i].promise == promise) {
                    g_uring_fds[i].active = 0;
                    if (g_uring.pending > 0) {
                        g_uring.pending--;
                    }
                    break;
                }
            }
        }
        head++;
    }
    *g_uring.cq_khead = head;
    return fired;
}

int io_uring_pending(void) {
    if (!g_uring.ready) {
        return 0;
    }
    return g_uring.pending;
}

int io_uring_wait_once(int timeout_ms) {
    if (!g_uring.ready || g_uring.pending <= 0) {
        return 0;
    }
    if (timeout_ms > 0) {
        struct pollfd pfd;
        pfd.fd = g_uring.ring_fd;
        pfd.events = POLLIN;
        pfd.revents = 0;
        int prc = poll(&pfd, 1, timeout_ms);
        if (prc <= 0) {
            return 0;
        }
    }
    (void)syscall(__NR_io_uring_enter, g_uring.ring_fd, 0, 1, IORING_ENTER_GETEVENTS, NULL, 0);
    return uring_process_cq();
}
#endif

int32_t io_uring_available(void) {
#if defined(__linux__)
    return probe_io_uring();
#else
    return 0;
#endif
}

int32_t io_uring_register_read(int32_t fd, int32_t promise) {
#if defined(__linux__)
    if (fd < 0 || promise <= 0) {
        return -1;
    }
    if (!probe_io_uring() || uring_ensure_ready() != 0) {
        extern int io_register(int fd, int task_id);
        return io_register(fd, promise);
    }
    NyraUringFd *slot = uring_fd_slot(fd, 1);
    if (!slot) {
        return -1;
    }
    int was_active = slot->active;
    slot->fd = fd;
    slot->promise = promise;
    slot->active = 1;
    if (!was_active) {
        g_uring.pending++;
    }

    struct io_uring_sqe *sqe = uring_get_sqe();
    if (!sqe) {
        if (!was_active && g_uring.pending > 0) {
            g_uring.pending--;
        }
        slot->active = 0;
        return -1;
    }
    sqe->opcode = IORING_OP_POLL_ADD;
    sqe->fd = fd;
    sqe->user_data = (uint64_t)(uint32_t)promise;
    (*g_uring.sq_ktail)++;
    return uring_flush_submit();
#else
    (void)fd;
    (void)promise;
    return -1;
#endif
}

int32_t io_uring_unregister_read(int32_t fd) {
#if defined(__linux__)
    if (fd < 0) {
        return -1;
    }
    NyraUringFd *slot = uring_fd_slot(fd, 0);
    if (!slot) {
        return -1;
    }
    slot->active = 0;
    if (g_uring.pending > 0) {
        g_uring.pending--;
    }
    return 0;
#else
    (void)fd;
    return -1;
#endif
}
