// Advanced OS: affinity, clocks, USB, serial, signals, mqueue, HW crypto, permissions.
#if defined(__linux__) && !defined(_GNU_SOURCE)
#define _GNU_SOURCE
#endif
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if !defined(_WIN32)
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <signal.h>
#include <time.h>
#include <pthread.h>
#endif

#if defined(__linux__)
#include <sched.h>
#include <sys/mman.h>
#include <mqueue.h>
#include <dirent.h>
#include <termios.h>
#include <sys/random.h>
#elif defined(__APPLE__)
#include <mach/mach.h>
#include <mach/thread_policy.h>
#include <termios.h>
#include <sys/random.h>
#include <IOKit/IOKitLib.h>
#elif defined(_WIN32)
#include <windows.h>
#include <bcrypt.h>
#include <setupapi.h>
#include <initguid.h>
#include <devguid.h>
#endif

static char *dup_cstr(const char *s) {
    if (!s) {
        char *e = (char *)malloc(1);
        if (e) {
            e[0] = '\0';
        }
        return e;
    }
    size_t n = strlen(s);
    char *out = (char *)malloc(n + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, s, n + 1);
    return out;
}

// --- CPU affinity (current thread) ---

int32_t rt_affinity_set_thread_cpu(int32_t core_index) {
    if (core_index < 0) {
        return -1;
    }
#if defined(__linux__)
    cpu_set_t set;
    CPU_ZERO(&set);
    CPU_SET((unsigned)core_index, &set);
    return pthread_setaffinity_np(pthread_self(), sizeof(set), &set) == 0 ? 0 : -1;
#elif defined(__APPLE__)
    thread_affinity_policy_data_t policy = {(integer_t)core_index};
    thread_port_t thread = mach_thread_self();
    kern_return_t kr = thread_policy_set(thread, THREAD_AFFINITY_POLICY,
                                         (thread_policy_t)&policy,
                                         THREAD_AFFINITY_POLICY_COUNT);
    mach_port_deallocate(mach_task_self(), thread);
    return kr == KERN_SUCCESS ? 0 : -1;
#elif defined(_WIN32)
    DWORD_PTR mask = (DWORD_PTR)1 << (unsigned)core_index;
    return SetThreadAffinityMask(GetCurrentThread(), mask) != 0 ? 0 : -1;
#else
    (void)core_index;
    return -1;
#endif
}

int32_t rt_affinity_get_thread_cpu(void) {
#if defined(__linux__)
    cpu_set_t set;
    CPU_ZERO(&set);
    if (pthread_getaffinity_np(pthread_self(), sizeof(set), &set) != 0) {
        return -1;
    }
    for (int i = 0; i < CPU_SETSIZE; i++) {
        if (CPU_ISSET(i, &set)) {
            return (int32_t)i;
        }
    }
    return -1;
#elif defined(_WIN32)
    return (int32_t)GetCurrentProcessorNumber();
#else
    return -1;
#endif
}

// --- High-resolution clocks ---

int64_t rt_clock_monotonic_ns(void) {
#if defined(_WIN32)
    LARGE_INTEGER freq, counter;
    if (!QueryPerformanceFrequency(&freq) || !QueryPerformanceCounter(&counter)) {
        return -1;
    }
    return (int64_t)((counter.QuadPart * 1000000000LL) / freq.QuadPart);
#else
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) {
        return -1;
    }
    return (int64_t)ts.tv_sec * 1000000000LL + (int64_t)ts.tv_nsec;
#endif
}

int64_t rt_clock_rdtsc(void) {
#if (defined(__x86_64__) || defined(__i386__)) && (defined(__GNUC__) || defined(__clang__))
    unsigned hi, lo;
    __asm__ volatile("rdtsc" : "=a"(lo), "=d"(hi));
    return ((int64_t)hi << 32) | lo;
#elif defined(__aarch64__) && (defined(__GNUC__) || defined(__clang__))
    int64_t val;
    __asm__ volatile("mrs %0, cntvct_el0" : "=r"(val));
    return val;
#else
    return rt_clock_monotonic_ns();
#endif
}

// --- USB device enumeration ---

typedef struct {
    int32_t vid;
    int32_t pid;
    char path[128];
} UsbDev;

#if defined(__linux__)
static int usb_collect_linux(UsbDev *out, int max) {
    DIR *d = opendir("/sys/bus/usb/devices");
    if (!d) {
        return 0;
    }
    int n = 0;
    struct dirent *ent;
    while ((ent = readdir(d)) != NULL && n < max) {
        if (ent->d_name[0] == '.' || strchr(ent->d_name, ':')) {
            continue;
        }
        char vpath[256];
        snprintf(vpath, sizeof(vpath), "/sys/bus/usb/devices/%s/idVendor", ent->d_name);
        FILE *vf = fopen(vpath, "r");
        if (!vf) {
            continue;
        }
        unsigned vid = 0;
        if (fscanf(vf, "%x", &vid) != 1) {
            fclose(vf);
            continue;
        }
        fclose(vf);
        snprintf(vpath, sizeof(vpath), "/sys/bus/usb/devices/%s/idProduct", ent->d_name);
        FILE *pf = fopen(vpath, "r");
        if (!pf) {
            continue;
        }
        unsigned pid = 0;
        if (fscanf(pf, "%x", &pid) != 1) {
            fclose(pf);
            continue;
        }
        fclose(pf);
        out[n].vid = (int32_t)vid;
        out[n].pid = (int32_t)pid;
        snprintf(out[n].path, sizeof(out[n].path), "/sys/bus/usb/devices/%s", ent->d_name);
        n++;
    }
    closedir(d);
    return n;
}
#endif

static int usb_device_count_internal(void) {
#if defined(__linux__)
    UsbDev tmp[64];
    return usb_collect_linux(tmp, 64);
#elif defined(_WIN32)
    return 0;
#else
    return 0;
#endif
}

int32_t rt_usb_device_count(void) {
    return (int32_t)usb_device_count_internal();
}

int32_t rt_usb_device_vid(int32_t index) {
#if defined(__linux__)
    UsbDev tmp[64];
    int n = usb_collect_linux(tmp, 64);
    if (index < 0 || index >= n) {
        return -1;
    }
    return tmp[index].vid;
#else
    (void)index;
    return -1;
#endif
}

int32_t rt_usb_device_pid(int32_t index) {
#if defined(__linux__)
    UsbDev tmp[64];
    int n = usb_collect_linux(tmp, 64);
    if (index < 0 || index >= n) {
        return -1;
    }
    return tmp[index].pid;
#else
    (void)index;
    return -1;
#endif
}

char *rt_usb_device_path(int32_t index) {
#if defined(__linux__)
    UsbDev tmp[64];
    int n = usb_collect_linux(tmp, 64);
    if (index < 0 || index >= n) {
        return dup_cstr("");
    }
    return dup_cstr(tmp[index].path);
#else
    (void)index;
    return dup_cstr("");
#endif
}

// --- Serial ports ---

#if !defined(_WIN32)
static speed_t baud_to_speed(int32_t baud) {
    switch (baud) {
    case 9600:
        return B9600;
    case 19200:
        return B19200;
    case 38400:
        return B38400;
    case 57600:
        return B57600;
    case 115200:
        return B115200;
    default:
        return B115200;
    }
}
#endif

int32_t rt_serial_open(const char *path, int32_t baud) {
    if (!path || !path[0]) {
        return -1;
    }
#if defined(_WIN32)
    char dev[128];
    if (path[0] != '\\' && path[1] != '\\') {
        snprintf(dev, sizeof(dev), "\\\\.\\%s", path);
    } else {
        snprintf(dev, sizeof(dev), "%s", path);
    }
    HANDLE h = CreateFileA(dev, GENERIC_READ | GENERIC_WRITE, 0, NULL, OPEN_EXISTING, 0,
                           NULL);
    if (h == INVALID_HANDLE_VALUE) {
        return -1;
    }
    DCB dcb = {0};
    dcb.DCBlength = sizeof(dcb);
    if (!GetCommState(h, &dcb)) {
        CloseHandle(h);
        return -1;
    }
    dcb.BaudRate = (DWORD)baud;
    dcb.ByteSize = 8;
    dcb.Parity = NOPARITY;
    dcb.StopBits = ONESTOPBIT;
    if (!SetCommState(h, &dcb)) {
        CloseHandle(h);
        return -1;
    }
    return (int32_t)(intptr_t)h;
#else
    int fd = open(path, O_RDWR | O_NOCTTY | O_SYNC);
    if (fd < 0) {
        return -1;
    }
    struct termios tty;
    if (tcgetattr(fd, &tty) != 0) {
        close(fd);
        return -1;
    }
    cfsetospeed(&tty, baud_to_speed(baud));
    cfsetispeed(&tty, baud_to_speed(baud));
    tty.c_cflag = (tty.c_cflag & ~CSIZE) | CS8;
    tty.c_iflag &= ~IGNBRK;
    tty.c_lflag = 0;
    tty.c_oflag = 0;
    tty.c_cc[VMIN] = 0;
    tty.c_cc[VTIME] = 10;
    tty.c_iflag &= ~(IXON | IXOFF | IXANY);
    tty.c_cflag |= (CLOCAL | CREAD);
    tty.c_cflag &= ~(PARENB | PARODD);
    tty.c_cflag &= ~CSTOPB;
    tty.c_cflag &= ~CRTSCTS;
    if (tcsetattr(fd, TCSANOW, &tty) != 0) {
        close(fd);
        return -1;
    }
    return (int32_t)fd;
#endif
}

char *rt_serial_read(int32_t handle, int32_t max_bytes) {
    if (handle < 0 || max_bytes <= 0) {
        return dup_cstr("");
    }
    if (max_bytes > 65536) {
        max_bytes = 65536;
    }
    char *buf = (char *)malloc((size_t)max_bytes + 1);
    if (!buf) {
        return dup_cstr("");
    }
#if defined(_WIN32)
    DWORD nread = 0;
    if (!ReadFile((HANDLE)(intptr_t)handle, buf, (DWORD)max_bytes, &nread, NULL)) {
        free(buf);
        return dup_cstr("");
    }
    buf[nread] = '\0';
#else
    ssize_t n = read(handle, buf, (size_t)max_bytes);
    if (n < 0) {
        free(buf);
        return dup_cstr("");
    }
    buf[n] = '\0';
#endif
    return buf;
}

int32_t rt_serial_write(int32_t handle, const char *data) {
    if (handle < 0 || !data) {
        return -1;
    }
    size_t len = strlen(data);
#if defined(_WIN32)
    DWORD written = 0;
    if (!WriteFile((HANDLE)(intptr_t)handle, data, (DWORD)len, &written, NULL)) {
        return -1;
    }
    return (int32_t)written;
#else
    ssize_t n = write(handle, data, len);
    return n < 0 ? -1 : (int32_t)n;
#endif
}

int32_t rt_serial_close(int32_t handle) {
    if (handle < 0) {
        return -1;
    }
#if defined(_WIN32)
    return CloseHandle((HANDLE)(intptr_t)handle) ? 0 : -1;
#else
    return close(handle);
#endif
}

// --- Signals (poll model; handlers set flags in C) ---

#define NYRA_SIG_MAX 128

#if defined(_WIN32)
static volatile LONG nyra_sig_pending[NYRA_SIG_MAX];

static BOOL WINAPI nyra_console_ctrl_handler(DWORD ctrl_type) {
    // Map Windows console events to a small subset of POSIX-like signals.
    // SIGINT  = 2  (Ctrl+C)
    // SIGTERM = 15 (console close/shutdown/logoff)
    switch (ctrl_type) {
    case CTRL_C_EVENT:
    case CTRL_BREAK_EVENT:
        InterlockedExchange(&nyra_sig_pending[2], 1);
        return TRUE;
    case CTRL_CLOSE_EVENT:
    case CTRL_LOGOFF_EVENT:
    case CTRL_SHUTDOWN_EVENT:
        InterlockedExchange(&nyra_sig_pending[15], 1);
        return TRUE;
    default:
        return FALSE;
    }
}

// --- Windows mqueue (local IPC) via mailslots ---
// NOTE: not POSIX semantics. This is a best-effort local queue.
typedef struct {
    HANDLE h;
    char name[64];
    int32_t valid;
} WinMqSlot;

static WinMqSlot nyra_win_mq_slots[16];
static int32_t nyra_win_mq_inited = 0;

static void nyra_win_mq_init_if_needed() {
    if (nyra_win_mq_inited) {
        return;
    }
    memset(nyra_win_mq_slots, 0, sizeof(nyra_win_mq_slots));
    nyra_win_mq_inited = 1;
}
#else
static volatile sig_atomic_t nyra_sig_pending[NYRA_SIG_MAX];

static void nyra_sig_forward(int sig) {
    if (sig >= 0 && sig < NYRA_SIG_MAX) {
        if (nyra_sig_pending[sig] == 0) {
            nyra_sig_pending[sig] = 1;
        }
    }
}
#endif

int32_t rt_signal_install(int32_t sig_num) {
#if defined(_WIN32)
    // Only SIGINT/SIGTERM are supported via console control handler.
    if (sig_num != 2 && sig_num != 15) {
        return -1;
    }
    return SetConsoleCtrlHandler(nyra_console_ctrl_handler, TRUE) ? 0 : -1;
#else
    if (sig_num < 1 || sig_num >= NYRA_SIG_MAX) {
        return -1;
    }
    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_handler = nyra_sig_forward;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = SA_RESTART;
    return sigaction(sig_num, &sa, NULL) == 0 ? 0 : -1;
#endif
}

int32_t rt_signal_poll(int32_t sig_num) {
    if (sig_num < 0 || sig_num >= NYRA_SIG_MAX) {
        return -1;
    }
#if defined(_WIN32)
    if (InterlockedExchange(&nyra_sig_pending[sig_num], 0) != 0) {
        return 1;
    }
    return 0;
#else
    if (nyra_sig_pending[sig_num]) {
        nyra_sig_pending[sig_num] = 0;
        return 1;
    }
    return 0;
#endif
}

// --- Message queues / IPC ---

#if defined(__linux__)
typedef struct {
    mqd_t mq;
    int32_t valid;
} MqSlot;

#define NYRA_MQ_MAX 16
static MqSlot nyra_mq_slots[NYRA_MQ_MAX];

static int32_t mq_alloc_slot(mqd_t mq) {
    for (int i = 0; i < NYRA_MQ_MAX; i++) {
        if (!nyra_mq_slots[i].valid) {
            nyra_mq_slots[i].mq = mq;
            nyra_mq_slots[i].valid = 1;
            return i;
        }
    }
    return -1;
}

static mqd_t mq_from_slot(int32_t id) {
    if (id < 0 || id >= NYRA_MQ_MAX || !nyra_mq_slots[id].valid) {
        return (mqd_t)-1;
    }
    return nyra_mq_slots[id].mq;
}
#endif

#if defined(__APPLE__)
typedef struct {
    char *buf;
    int32_t max_msgs;
    int32_t msg_size;
    int32_t head;
    int32_t tail;
    int32_t count;
    int32_t valid;
} MqSlot;

#define NYRA_MQ_MAX 16
static MqSlot nyra_mq_slots[NYRA_MQ_MAX];

static char *nyra_mq_slot_at(MqSlot *slot, int32_t idx) {
    if (!slot || !slot->buf || idx < 0 || idx >= slot->max_msgs) {
        return NULL;
    }
    return slot->buf + ((size_t)idx * (size_t)slot->msg_size);
}
#endif

int32_t rt_mqueue_open(const char *name, int32_t max_msgs, int32_t msg_size) {
#if defined(__linux__)
    if (!name || max_msgs <= 0 || msg_size <= 0) {
        return -1;
    }
    char qname[64];
    snprintf(qname, sizeof(qname), "/%s", name[0] == '/' ? name + 1 : name);
    struct mq_attr attr;
    memset(&attr, 0, sizeof(attr));
    attr.mq_maxmsg = (long)max_msgs;
    attr.mq_msgsize = (long)msg_size;
    mqd_t mq = mq_open(qname, O_CREAT | O_RDWR, 0600, &attr);
    if (mq == (mqd_t)-1) {
        return -1;
    }
    return mq_alloc_slot(mq);
#elif defined(__APPLE__)
    if (!name || max_msgs <= 0 || msg_size <= 0) {
        return -1;
    }
    int32_t slot = -1;
    for (int32_t i = 0; i < NYRA_MQ_MAX; i++) {
        if (!nyra_mq_slots[i].valid) {
            slot = i;
            break;
        }
    }
    if (slot < 0) {
        return -1;
    }
    size_t total = (size_t)max_msgs * (size_t)msg_size;
    char *buf = (char *)malloc(total);
    if (!buf) {
        return -1;
    }
    nyra_mq_slots[slot].buf = buf;
    nyra_mq_slots[slot].max_msgs = max_msgs;
    nyra_mq_slots[slot].msg_size = msg_size;
    nyra_mq_slots[slot].head = 0;
    nyra_mq_slots[slot].tail = 0;
    nyra_mq_slots[slot].count = 0;
    nyra_mq_slots[slot].valid = 1;
    return slot;
#elif defined(_WIN32)
    // Windows implementation: Mailslot-based best-effort message queue.
    // Note: this is not POSIX mqueue semantics; it's a simple local IPC primitive.
    (void)max_msgs;
    (void)msg_size;
    if (!name) {
        return -1;
    }
    nyra_win_mq_init_if_needed();
    char slotname[128];
    snprintf(slotname, sizeof(slotname), "\\\\.?\\mailslot\\nyra_%s", name);
    // Create a mailslot for receiving.
    HANDLE h = CreateMailslotA(slotname, 0, 0, NULL);
    if (h == INVALID_HANDLE_VALUE) {
        return -1;
    }
    for (int i = 0; i < 16; i++) {
        if (!nyra_win_mq_slots[i].valid) {
            nyra_win_mq_slots[i].h = h;
            strncpy(nyra_win_mq_slots[i].name, slotname, sizeof(nyra_win_mq_slots[i].name) - 1);
            nyra_win_mq_slots[i].name[sizeof(nyra_win_mq_slots[i].name) - 1] = '\0';
            nyra_win_mq_slots[i].valid = 1;
            return i;
        }
    }
    CloseHandle(h);
    return -1;
#else
    (void)name;
    (void)max_msgs;
    (void)msg_size;
    return -1;
#endif
}

int32_t rt_mqueue_send(int32_t mq_id, const char *msg) {
#if defined(__linux__)
    mqd_t mq = mq_from_slot(mq_id);
    if (mq == (mqd_t)-1 || !msg) {
        return -1;
    }
    return mq_send(mq, msg, strlen(msg), 0) == 0 ? 0 : -1;
#elif defined(__APPLE__)
    if (mq_id < 0 || mq_id >= NYRA_MQ_MAX || !nyra_mq_slots[mq_id].valid || !msg) {
        return -1;
    }
    MqSlot *slot = &nyra_mq_slots[mq_id];
    if (slot->count >= slot->max_msgs) {
        return -1;
    }
    char *dst = nyra_mq_slot_at(slot, slot->tail);
    if (!dst) {
        return -1;
    }
    int32_t len = (int32_t)strlen(msg);
    if (slot->msg_size <= 0) {
        return -1;
    }
    if (len >= slot->msg_size) {
        len = slot->msg_size - 1;
    }
    if (len < 0) {
        len = 0;
    }
    memcpy(dst, msg, (size_t)len);
    dst[len] = '\0';
    slot->tail = (slot->tail + 1) % slot->max_msgs;
    slot->count++;
    return 0;
#elif defined(_WIN32)
    if (mq_id < 0 || mq_id >= 16 || !msg) {
        return -1;
    }
    nyra_win_mq_init_if_needed();
    if (!nyra_win_mq_slots[mq_id].valid) {
        return -1;
    }
    HANDLE wh = CreateFileA(
        nyra_win_mq_slots[mq_id].name,
        GENERIC_WRITE,
        FILE_SHARE_READ,
        NULL,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        NULL
    );
    if (wh == INVALID_HANDLE_VALUE) {
        return -1;
    }
    DWORD written = 0;
    BOOL ok = WriteFile(wh, msg, (DWORD)strlen(msg), &written, NULL);
    CloseHandle(wh);
    return ok ? 0 : -1;
#else
    (void)mq_id;
    (void)msg;
    return -1;
#endif
}

char *rt_mqueue_recv(int32_t mq_id, int32_t max_bytes) {
#if defined(__linux__)
    mqd_t mq = mq_from_slot(mq_id);
    if (mq == (mqd_t)-1 || max_bytes <= 0) {
        return dup_cstr("");
    }
    if (max_bytes > 65536) {
        max_bytes = 65536;
    }
    char *buf = (char *)malloc((size_t)max_bytes + 1);
    if (!buf) {
        return dup_cstr("");
    }
    ssize_t n = mq_receive(mq, buf, (size_t)max_bytes, NULL);
    if (n < 0) {
        free(buf);
        return dup_cstr("");
    }
    buf[n] = '\0';
    return buf;
#elif defined(__APPLE__)
    if (mq_id < 0 || mq_id >= NYRA_MQ_MAX || max_bytes <= 0 || !nyra_mq_slots[mq_id].valid) {
        return dup_cstr("");
    }
    MqSlot *slot = &nyra_mq_slots[mq_id];
    if (slot->count <= 0) {
        return dup_cstr("");
    }
    char *src = nyra_mq_slot_at(slot, slot->head);
    if (!src) {
        return dup_cstr("");
    }
    int32_t len = (int32_t)strlen(src);
    if (len < 0) {
        len = 0;
    }
    if (len > max_bytes) {
        len = max_bytes;
    }
    char *out = (char *)malloc((size_t)len + 1);
    if (!out) {
        return dup_cstr("");
    }
    memcpy(out, src, (size_t)len);
    out[len] = '\0';
    slot->head = (slot->head + 1) % slot->max_msgs;
    slot->count--;
    return out;
#elif defined(_WIN32)
    if (mq_id < 0 || mq_id >= 16 || max_bytes <= 0) {
        return dup_cstr("");
    }
    nyra_win_mq_init_if_needed();
    if (max_bytes > 65536) {
        max_bytes = 65536;
    }
    if (!nyra_win_mq_slots[mq_id].valid) {
        return dup_cstr("");
    }
    DWORD next_size = 0, msg_count = 0;
    if (!GetMailslotInfo(nyra_win_mq_slots[mq_id].h, NULL, &next_size, &msg_count, NULL)) {
        return dup_cstr("");
    }
    if (next_size == MAILSLOT_NO_MESSAGE || msg_count == 0) {
        return dup_cstr("");
    }
    if ((int32_t)next_size > max_bytes) {
        // read and truncate
        next_size = (DWORD)max_bytes;
    }
    char *buf = (char *)malloc((size_t)next_size + 1);
    if (!buf) {
        return dup_cstr("");
    }
    DWORD readn = 0;
    if (!ReadFile(nyra_win_mq_slots[mq_id].h, buf, next_size, &readn, NULL)) {
        free(buf);
        return dup_cstr("");
    }
    buf[readn] = '\0';
    return buf;
#else
    (void)mq_id;
    (void)max_bytes;
    return dup_cstr("");
#endif
}

int32_t rt_mqueue_close(int32_t mq_id) {
#if defined(__linux__)
    if (mq_id < 0 || mq_id >= NYRA_MQ_MAX || !nyra_mq_slots[mq_id].valid) {
        return -1;
    }
    int r = mq_close(nyra_mq_slots[mq_id].mq);
    nyra_mq_slots[mq_id].valid = 0;
    return r == 0 ? 0 : -1;
#elif defined(__APPLE__)
    if (mq_id < 0 || mq_id >= NYRA_MQ_MAX || !nyra_mq_slots[mq_id].valid) {
        return -1;
    }
    MqSlot *slot = &nyra_mq_slots[mq_id];
    if (slot->buf) {
        free(slot->buf);
        slot->buf = NULL;
    }
    slot->valid = 0;
    return 0;
#elif defined(_WIN32)
    if (mq_id < 0 || mq_id >= 16) {
        return -1;
    }
    nyra_win_mq_init_if_needed();
    if (!nyra_win_mq_slots[mq_id].valid) {
        return -1;
    }
    CloseHandle(nyra_win_mq_slots[mq_id].h);
    nyra_win_mq_slots[mq_id].valid = 0;
    return 0;
#else
    (void)mq_id;
    return -1;
#endif
}

// --- Hardware-backed random / secure enclave probe ---

char *rt_hw_random_bytes(int32_t count) {
    if (count <= 0 || count > 4096) {
        return dup_cstr("");
    }
    char *buf = (char *)malloc((size_t)count + 1);
    if (!buf) {
        return dup_cstr("");
    }
#if defined(__APPLE__)
    arc4random_buf(buf, (size_t)count);
    buf[count] = '\0';
    return buf;
#elif defined(__linux__)
    if (getentropy(buf, (size_t)count) == 0) {
        buf[count] = '\0';
        return buf;
    }
#endif
#if defined(_WIN32)
    if (BCryptGenRandom(NULL, (PUCHAR)buf, (ULONG)count, BCRYPT_USE_SYSTEM_PREFERRED_RNG) ==
        0) {
        buf[count] = '\0';
        return buf;
    }
#endif
    FILE *f = fopen("/dev/urandom", "rb");
    if (f && fread(buf, 1, (size_t)count, f) == (size_t)count) {
        fclose(f);
        buf[count] = '\0';
        return buf;
    }
    if (f) {
        fclose(f);
    }
    free(buf);
    return dup_cstr("");
}

int32_t rt_hw_secure_enclave_available(void) {
#if defined(__APPLE__) && defined(__aarch64__)
    return 1;
#elif defined(__linux__) && defined(__x86_64__)
    return 0;
#else
    return 0;
#endif
}

// --- Permissions ---

int32_t rt_perm_getuid(void) {
#if defined(_WIN32)
    return -1;
#else
    return (int32_t)getuid();
#endif
}

int32_t rt_perm_geteuid(void) {
#if defined(_WIN32)
    return -1;
#else
    return (int32_t)geteuid();
#endif
}

int32_t rt_perm_drop_to_uid(int32_t uid) {
#if defined(_WIN32)
    (void)uid;
    return -1;
#else
    if (setuid((uid_t)uid) != 0) {
        return -1;
    }
    return 0;
#endif
}

int32_t rt_perm_chroot(const char *path) {
#if defined(_WIN32)
    if (!path) {
        return -1;
    }
    return SetCurrentDirectoryA(path) ? 0 : -1;
#elif defined(__linux__) || defined(__APPLE__)
    if (!path) {
        return -1;
    }
    if (chroot(path) != 0) {
        return -1;
    }
    return chdir("/") == 0 ? 0 : -1;
#else
    (void)path;
    return -1;
#endif
}

int32_t rt_perm_sandbox_seatbelt_available(void) {
#if defined(__APPLE__)
    return 1;
#else
    return 0;
#endif
}
