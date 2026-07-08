// Pseudo-terminal spawn + I/O. POSIX + Windows ConPTY.
#if defined(_WIN32)
#include "rt_pty_win.inc.c"
#else

#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <sys/ioctl.h>
#include <sys/select.h>
#include <sys/wait.h>
#include <unistd.h>

#if defined(__APPLE__)
#include <util.h>
#else
#include <pty.h>
#endif

#define PTY_MAX_TRACKED 8

struct pty_child_entry {
    int master;
    pid_t pid;
};

static struct pty_child_entry g_pty_children[PTY_MAX_TRACKED];

static void pty_children_init(void) {
    static int initialized = 0;
    if (initialized) {
        return;
    }
    for (int i = 0; i < PTY_MAX_TRACKED; i++) {
        g_pty_children[i].master = -1;
        g_pty_children[i].pid = -1;
    }
    initialized = 1;
}

static void pty_ignore_sigpipe_once(void) {
    static int done = 0;
    if (!done) {
        signal(SIGPIPE, SIG_IGN);
        done = 1;
    }
}

static void pty_track_child(int master, pid_t pid) {
    pty_children_init();
    for (int i = 0; i < PTY_MAX_TRACKED; i++) {
        if (g_pty_children[i].master < 0) {
            g_pty_children[i].master = master;
            g_pty_children[i].pid = pid;
            return;
        }
    }
}

static pid_t pty_find_child(int master) {
    pty_children_init();
    for (int i = 0; i < PTY_MAX_TRACKED; i++) {
        if (g_pty_children[i].master == master) {
            return g_pty_children[i].pid;
        }
    }
    return -1;
}

static void pty_untrack_child(int master) {
    pty_children_init();
    for (int i = 0; i < PTY_MAX_TRACKED; i++) {
        if (g_pty_children[i].master == master) {
            g_pty_children[i].master = -1;
            g_pty_children[i].pid = -1;
            return;
        }
    }
}

static char *pty_empty_string(void) {
    char *empty = (char *)malloc(1);
    if (empty) {
        empty[0] = '\0';
    }
    return empty;
}

static char *pty_strip_ansi_copy(const char *input) {
    if (!input) {
        return pty_empty_string();
    }
    size_t in_len = strlen(input);
    char *out = (char *)malloc(in_len + 1);
    if (!out) {
        return pty_empty_string();
    }
    size_t j = 0;
    for (size_t i = 0; i < in_len; i++) {
        unsigned char c = (unsigned char)input[i];
        if (c == 0x1b && i + 1 < in_len && input[i + 1] == '[') {
            i += 2;
            while (i < in_len && input[i] != 'm' && input[i] != 'H' && input[i] != 'J' && input[i] != 'K') {
                i++;
            }
            continue;
        }
        if (c == '\r') {
            continue;
        }
        if (c >= 32 || c == '\n' || c == '\t') {
            out[j++] = (char)c;
        }
    }
    out[j] = '\0';
    return out;
}

static char *pty_strip_owned(char *raw) {
    if (!raw) {
        return pty_empty_string();
    }
    char *out = pty_strip_ansi_copy(raw);
    free(raw);
    if (!out) {
        return pty_empty_string();
    }
    return out;
}

static void pty_set_winsize(int master, int rows, int cols) {
    struct winsize ws;
    ws.ws_row = (unsigned short)(rows > 0 ? rows : 24);
    ws.ws_col = (unsigned short)(cols > 0 ? cols : 80);
    ws.ws_xpixel = 0;
    ws.ws_ypixel = 0;
    ioctl(master, TIOCSWINSZ, &ws);
}

int pty_spawn(const char *shell, int rows, int cols) {
    pty_ignore_sigpipe_once();
    int master = -1;
    pid_t pid = forkpty(&master, NULL, NULL, NULL);
    if (pid < 0) {
        return -1;
    }
    if (pid == 0) {
        setenv("TERM", "xterm-256color", 1);
        setenv("COLORTERM", "truecolor", 1);
        if (shell && shell[0] != '\0') {
            if (strstr(shell, "bash") != NULL) {
                execl(shell, "bash", "--noprofile", "--norc", (char *)NULL);
            } else {
                execl(shell, shell, (char *)NULL);
            }
        } else {
            execl("/bin/bash", "bash", "--noprofile", "--norc", (char *)NULL);
        }
        _exit(127);
    }
    pty_track_child(master, pid);
    pty_set_winsize(master, rows, cols);
    int flags = fcntl(master, F_GETFL, 0);
    if (flags >= 0) {
        fcntl(master, F_SETFL, flags | O_NONBLOCK);
    }
    return master;
}

int pty_write(int master, const char *data) {
    if (master < 0 || !data) {
        return -1;
    }
    ssize_t n = write(master, data, strlen(data));
    if (n < 0 && (errno == EPIPE || errno == EIO)) {
        return -1;
    }
    return (int)n;
}

char *pty_read(int master, int max_bytes) {
    if (master < 0) {
        return pty_empty_string();
    }
    if (max_bytes <= 0) {
        max_bytes = 4096;
    }
    char *buf = (char *)malloc((size_t)max_bytes + 1);
    if (!buf) {
        return pty_empty_string();
    }
    ssize_t n = read(master, buf, (size_t)max_bytes);
    if (n < 0) {
        if (errno == EAGAIN || errno == EWOULDBLOCK) {
            buf[0] = '\0';
            return buf;
        }
        free(buf);
        return pty_empty_string();
    }
    if (n == 0) {
        buf[0] = '\0';
        return buf;
    }
    buf[n] = '\0';
    return buf;
}

char *pty_drain(int master, int max_bytes) {
    if (master < 0) {
        return pty_empty_string();
    }
    if (max_bytes <= 0) {
        max_bytes = 4096;
    }
    char *buf = (char *)malloc((size_t)max_bytes + 1);
    if (!buf) {
        return pty_empty_string();
    }
    size_t total = 0;
    while (total < (size_t)max_bytes) {
        ssize_t n = read(master, buf + total, (size_t)max_bytes - total);
        if (n > 0) {
            total += (size_t)n;
            continue;
        }
        if (n < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) {
            break;
        }
        break;
    }
    buf[total] = '\0';
    return pty_strip_owned(buf);
}

static char *pty_normalize_raw_owned(char *raw) {
    if (!raw) {
        return pty_empty_string();
    }
    size_t len = strlen(raw);
    char *out = (char *)malloc(len + 1);
    if (!out) {
        free(raw);
        return pty_empty_string();
    }
    size_t j = 0;
    for (size_t i = 0; i < len; i++) {
        unsigned char c = (unsigned char)raw[i];
        if (c == '\r') {
            continue;
        }
        out[j++] = (char)c;
    }
    out[j] = '\0';
    free(raw);
    return out;
}

char *pty_drain_raw(int master, int max_bytes) {
    if (master < 0) {
        return pty_empty_string();
    }
    if (max_bytes <= 0) {
        max_bytes = 4096;
    }
    char *buf = (char *)malloc((size_t)max_bytes + 1);
    if (!buf) {
        return pty_empty_string();
    }
    size_t total = 0;
    while (total < (size_t)max_bytes) {
        ssize_t n = read(master, buf + total, (size_t)max_bytes - total);
        if (n > 0) {
            total += (size_t)n;
            continue;
        }
        if (n < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) {
            break;
        }
        break;
    }
    buf[total] = '\0';
    return pty_normalize_raw_owned(buf);
}

int pty_poll(int master) {
    if (master < 0) {
        return 0;
    }
    fd_set fds;
    struct timeval tv;
    FD_ZERO(&fds);
    FD_SET(master, &fds);
    tv.tv_sec = 0;
    tv.tv_usec = 0;
    int rc = select(master + 1, &fds, NULL, NULL, &tv);
    if (rc > 0 && FD_ISSET(master, &fds)) {
        return 1;
    }
    return 0;
}

void pty_resize(int master, int rows, int cols) {
    pty_set_winsize(master, rows, cols);
}

void pty_close(int master) {
    if (master >= 0) {
        pty_untrack_child(master);
        close(master);
    }
}

int pty_wait(int master) {
    pid_t pid = pty_find_child(master);
    if (pid <= 0) {
        return 0;
    }
    int status = 0;
    if (waitpid(pid, &status, WNOHANG) > 0) {
        pty_untrack_child(master);
        return 1;
    }
    return 0;
}

char *pty_read_wait(int master, int max_bytes, int timeout_ms) {
    if (master < 0) {
        return pty_empty_string();
    }
    if (max_bytes <= 0) {
        max_bytes = 4096;
    }
    fd_set fds;
    struct timeval tv;
    FD_ZERO(&fds);
    FD_SET(master, &fds);
    tv.tv_sec = timeout_ms / 1000;
    tv.tv_usec = (timeout_ms % 1000) * 1000;
    int rc = select(master + 1, &fds, NULL, NULL, &tv);
    if (rc <= 0 || !FD_ISSET(master, &fds)) {
        return pty_empty_string();
    }
    char *raw = pty_read(master, max_bytes);
    if (!raw) {
        return pty_empty_string();
    }
    return pty_strip_owned(raw);
}

char *pty_read_wait_raw(int master, int max_bytes, int timeout_ms) {
    if (master < 0) {
        return pty_empty_string();
    }
    if (max_bytes <= 0) {
        max_bytes = 4096;
    }
    fd_set fds;
    struct timeval tv;
    FD_ZERO(&fds);
    FD_SET(master, &fds);
    tv.tv_sec = timeout_ms / 1000;
    tv.tv_usec = (timeout_ms % 1000) * 1000;
    int rc = select(master + 1, &fds, NULL, NULL, &tv);
    if (rc <= 0 || !FD_ISSET(master, &fds)) {
        return pty_empty_string();
    }
    char *raw = pty_read(master, max_bytes);
    if (!raw) {
        return pty_empty_string();
    }
    return pty_normalize_raw_owned(raw);
}

void pty_flush_stdout(int master, int max_bytes, int timeout_ms) {
    if (master < 0) {
        return;
    }
    if (max_bytes <= 0) {
        max_bytes = 4096;
    }

    int budget_ms = timeout_ms > 0 ? timeout_ms : 0;
    int got_any = 0;

    for (;;) {
        char *chunk = pty_drain(master, max_bytes);
        if (chunk && chunk[0] != '\0') {
            fputs(chunk, stdout);
            fflush(stdout);
            free(chunk);
            got_any = 1;
            budget_ms = 250;
            continue;
        }
        free(chunk);

        if (budget_ms <= 0) {
            break;
        }

        fd_set fds;
        struct timeval tv;
        FD_ZERO(&fds);
        FD_SET(master, &fds);
        int slice = budget_ms > 50 ? 50 : budget_ms;
        tv.tv_sec = slice / 1000;
        tv.tv_usec = (slice % 1000) * 1000;
        int rc = select(master + 1, &fds, NULL, NULL, &tv);
        budget_ms -= slice;
        if (rc > 0 && FD_ISSET(master, &fds)) {
            continue;
        }
        if (got_any) {
            break;
        }
        if (rc < 0 && errno != EINTR) {
            break;
        }
    }
}

#endif /* !_WIN32 */
