#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(_WIN32)
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <winsock2.h>
#include <ws2tcpip.h>

void nyra_winsock_ensure(void) {
    static int inited = 0;
    if (!inited) {
        WSADATA wsa;
        if (WSAStartup(MAKEWORD(2, 2), &wsa) == 0) {
            inited = 1;
        }
    }
}

static int nyra_sock_close(int fd) {
    if (fd < 0) {
        return 0;
    }
    return closesocket((SOCKET)fd) == 0 ? 0 : -1;
}

static int nyra_sock_set_nonblock(int fd) {
    if (fd < 0) {
        return -1;
    }
    u_long mode = 1;
    return ioctlsocket((SOCKET)fd, FIONBIO, &mode) == 0 ? 0 : -1;
}

#else
#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <netdb.h>
#include <pthread.h>
#include <sys/select.h>
#include <unistd.h>
#include <sys/socket.h>

void nyra_winsock_ensure(void) {}

static int nyra_sock_close(int fd) {
    if (fd >= 0) {
        close(fd);
    }
    return 0;
}

static int nyra_sock_set_nonblock(int fd) {
    if (fd < 0) {
        return -1;
    }
    int flags = fcntl(fd, F_GETFL, 0);
    if (flags < 0) {
        return -1;
    }
    return fcntl(fd, F_SETFL, flags | O_NONBLOCK) == 0 ? 0 : -1;
}

#endif

extern int async_promise_new(void);
extern void async_promise_complete(int handle, int value);

static int set_reuse(int fd) {
    int one = 1;
    return setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, (const char *)&one, sizeof(one));
}

static int host_is_passive(const char *host) {
    return host == NULL || host[0] == '\0' ||
           strcmp(host, "0.0.0.0") == 0 || strcmp(host, "::") == 0;
}

int rt_tcp_listen(const char *host, int port) {
#if defined(_WIN32)
    nyra_winsock_ensure();
#endif
    struct addrinfo hints;
    struct addrinfo *res = NULL;
    char portbuf[16];
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    if (host_is_passive(host)) {
        hints.ai_flags = AI_PASSIVE;
        host = NULL;
    }
    snprintf(portbuf, sizeof(portbuf), "%d", port);
    if (getaddrinfo(host, portbuf, &hints, &res) != 0 || !res) {
        return -1;
    }
    int fd = -1;
    for (struct addrinfo *p = res; p; p = p->ai_next) {
        fd = (int)socket(p->ai_family, p->ai_socktype, p->ai_protocol);
        if (fd < 0) {
            continue;
        }
        set_reuse(fd);
        if (bind(fd, p->ai_addr, (int)p->ai_addrlen) == 0 &&
            listen(fd, SOMAXCONN) == 0) {
            break;
        }
        nyra_sock_close(fd);
        fd = -1;
    }
    freeaddrinfo(res);
    return fd;
}

int rt_tcp_accept(int listener_fd) {
    if (listener_fd < 0) {
        return -1;
    }
#if defined(_WIN32)
    nyra_winsock_ensure();
#endif
    return (int)accept(listener_fd, NULL, NULL);
}

typedef struct {
    int listener_fd;
    int task_id;
} AcceptJob;

#if defined(_WIN32)
static DWORD WINAPI accept_worker(LPVOID arg) {
    AcceptJob *job = (AcceptJob *)arg;
    int client = rt_tcp_accept(job->listener_fd);
    async_promise_complete(job->task_id, client);
    free(job);
    return 0;
}
#else
static void *accept_worker(void *arg) {
    AcceptJob *job = (AcceptJob *)arg;
    int client = rt_tcp_accept(job->listener_fd);
    async_promise_complete(job->task_id, client);
    free(job);
    return NULL;
}
#endif

int rt_tcp_accept_async(int listener_fd) {
    int task = async_promise_new();
    if (task <= 0) {
        return -1;
    }
    AcceptJob *job = (AcceptJob *)malloc(sizeof(AcceptJob));
    if (!job) {
        return -1;
    }
    job->listener_fd = listener_fd;
    job->task_id = task;
#if defined(_WIN32)
    HANDLE th = CreateThread(NULL, 0, accept_worker, job, 0, NULL);
    if (!th) {
        free(job);
        return -1;
    }
    CloseHandle(th);
#else
    pthread_t th;
    if (pthread_create(&th, NULL, accept_worker, job) != 0) {
        free(job);
        return -1;
    }
    pthread_detach(th);
#endif
    return task;
}

int rt_tcp_connect(const char *host, int port) {
#if defined(_WIN32)
    nyra_winsock_ensure();
#endif
    struct addrinfo hints;
    struct addrinfo *res = NULL;
    char portbuf[16];
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    snprintf(portbuf, sizeof(portbuf), "%d", port);
    if (getaddrinfo(host, portbuf, &hints, &res) != 0 || !res) {
        return -1;
    }
    int fd = -1;
    for (struct addrinfo *p = res; p; p = p->ai_next) {
        fd = (int)socket(p->ai_family, p->ai_socktype, p->ai_protocol);
        if (fd < 0) {
            continue;
        }
        if (connect(fd, p->ai_addr, (int)p->ai_addrlen) == 0) {
            break;
        }
        nyra_sock_close(fd);
        fd = -1;
    }
    freeaddrinfo(res);
    return fd;
}

char *rt_tcp_read(int fd, int max_bytes) {
    if (fd < 0 || max_bytes <= 0) {
        return NULL;
    }
    if (max_bytes > 1024 * 1024) {
        max_bytes = 1024 * 1024;
    }
    char *buf = (char *)malloc((size_t)max_bytes + 1);
    if (!buf) {
        return NULL;
    }
#if defined(_WIN32)
    nyra_winsock_ensure();
    int n = recv(fd, buf, max_bytes, 0);
#else
    ssize_t n = recv(fd, buf, (size_t)max_bytes, 0);
#endif
    if (n <= 0) {
        free(buf);
        return NULL;
    }
    buf[n] = '\0';
    return buf;
}

int rt_tcp_write_bytes(int fd, const char *data, int len) {
    if (fd < 0 || !data || len < 0) {
        return -1;
    }
#if defined(_WIN32)
    nyra_winsock_ensure();
    int n = send(fd, data, len, 0);
    return (n == len) ? 0 : -1;
#else
    ssize_t n = send(fd, data, (size_t)len, 0);
    return (n == (ssize_t)len) ? 0 : -1;
#endif
}

int rt_tcp_read_bytes(int fd, char *buf, int len) {
    if (fd < 0 || !buf || len <= 0) {
        return -1;
    }
    int got = 0;
    while (got < len) {
#if defined(_WIN32)
        nyra_winsock_ensure();
        int n = recv(fd, buf + got, len - got, 0);
#else
        ssize_t n = recv(fd, buf + got, (size_t)(len - got), 0);
#endif
        if (n <= 0) {
            return -1;
        }
        got += (int)n;
    }
    return 0;
}

int rt_tcp_write(int fd, const char *data) {
    if (fd < 0 || !data) {
        return -1;
    }
    return rt_tcp_write_bytes(fd, data, (int)strlen(data));
}

void rt_tcp_close(int fd) {
    nyra_sock_close(fd);
}

int sys_set_nonblock(int fd) {
#if defined(_WIN32)
    nyra_winsock_ensure();
#endif
    return nyra_sock_set_nonblock(fd);
}

int sys_set_timeout_ms(int fd, int timeout_ms) {
    if (fd < 0) {
        return -1;
    }
    if (timeout_ms < 0) {
        timeout_ms = 0;
    }
#if defined(_WIN32)
    DWORD tv = (DWORD)timeout_ms;
    if (setsockopt((SOCKET)fd, SOL_SOCKET, SO_RCVTIMEO, (const char *)&tv, sizeof(tv)) != 0) {
        return -1;
    }
    if (setsockopt((SOCKET)fd, SOL_SOCKET, SO_SNDTIMEO, (const char *)&tv, sizeof(tv)) != 0) {
        return -1;
    }
    return 0;
#else
    struct timeval tv;
    tv.tv_sec = timeout_ms / 1000;
    tv.tv_usec = (timeout_ms % 1000) * 1000;
    if (setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv)) != 0) {
        return -1;
    }
    if (setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof(tv)) != 0) {
        return -1;
    }
    return 0;
#endif
}


int sys_listen(const char *host, int port) {
    return rt_tcp_listen(host, port);
}

int sys_accept(int listener_fd) {
    return rt_tcp_accept(listener_fd);
}

int sys_connect(const char *host, int port) {
    return rt_tcp_connect(host, port);
}

char *sys_recv(int fd, int max_bytes) {
    return rt_tcp_read(fd, max_bytes);
}

int sys_send(int fd, const char *data) {
    return rt_tcp_write(fd, data);
}

void sys_close(int fd) {
    rt_tcp_close(fd);
}

int rt_udp_bind(const char *host, int port) {
#if defined(_WIN32)
    nyra_winsock_ensure();
#endif
    struct addrinfo hints;
    struct addrinfo *res = NULL;
    char portbuf[16];
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_DGRAM;
    if (host_is_passive(host)) {
        hints.ai_flags = AI_PASSIVE;
        host = NULL;
    }
    snprintf(portbuf, sizeof(portbuf), "%d", port);
    if (getaddrinfo(host, portbuf, &hints, &res) != 0 || !res) {
        return -1;
    }
    int fd = -1;
    for (struct addrinfo *p = res; p; p = p->ai_next) {
        fd = (int)socket(p->ai_family, p->ai_socktype, p->ai_protocol);
        if (fd < 0) {
            continue;
        }
        set_reuse(fd);
        if (bind(fd, p->ai_addr, (int)p->ai_addrlen) == 0) {
            break;
        }
        nyra_sock_close(fd);
        fd = -1;
    }
    freeaddrinfo(res);
    return fd;
}

char *rt_udp_recv(int fd, int max_bytes) {
    if (fd < 0 || max_bytes <= 0) {
        return NULL;
    }
    if (max_bytes > 65536) {
        max_bytes = 65536;
    }
    char *buf = (char *)malloc((size_t)max_bytes + 1);
    if (!buf) {
        return NULL;
    }
#if defined(_WIN32)
    nyra_winsock_ensure();
    int n = recv(fd, buf, max_bytes, 0);
#else
    ssize_t n = recv(fd, buf, (size_t)max_bytes, 0);
#endif
    if (n <= 0) {
        free(buf);
        return NULL;
    }
    buf[n] = '\0';
    return buf;
}

int rt_udp_send(int fd, const char *host, int port, const char *data) {
    if (fd < 0 || !host || !data) {
        return -1;
    }
    struct addrinfo hints;
    struct addrinfo *res = NULL;
    char portbuf[16];
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_DGRAM;
    snprintf(portbuf, sizeof(portbuf), "%d", port);
    if (getaddrinfo(host, portbuf, &hints, &res) != 0 || !res) {
        return -1;
    }
    int ok = -1;
#if defined(_WIN32)
    nyra_winsock_ensure();
    int n = sendto(fd, data, (int)strlen(data), 0, res->ai_addr, (int)res->ai_addrlen);
    ok = (n >= 0) ? 0 : -1;
#else
    ssize_t n = sendto(fd, data, strlen(data), 0, res->ai_addr, res->ai_addrlen);
    ok = (n >= 0) ? 0 : -1;
#endif
    freeaddrinfo(res);
    return ok;
}

void rt_udp_close(int fd) {
    nyra_sock_close(fd);
}

static int tcp_finish_connect(int fd, int timeout_ms) {
#if defined(_WIN32)
    (void)timeout_ms;
    return fd;
#else
    if (timeout_ms <= 0) {
        return fd;
    }
    fd_set wfds;
    FD_ZERO(&wfds);
    FD_SET(fd, &wfds);
    struct timeval tv;
    tv.tv_sec = timeout_ms / 1000;
    tv.tv_usec = (timeout_ms % 1000) * 1000;
    int sel = select(fd + 1, NULL, &wfds, NULL, &tv);
    if (sel <= 0) {
        nyra_sock_close(fd);
        return -1;
    }
    int err = 0;
    socklen_t elen = sizeof(err);
    if (getsockopt(fd, SOL_SOCKET, SO_ERROR, &err, &elen) != 0 || err != 0) {
        nyra_sock_close(fd);
        return -1;
    }
    int flags = fcntl(fd, F_GETFL, 0);
    if (flags >= 0) {
        fcntl(fd, F_SETFL, flags & ~O_NONBLOCK);
    }
    return fd;
#endif
}

int rt_tcp_connect_timeout(const char *host, int port, int timeout_ms) {
#if defined(_WIN32)
    (void)timeout_ms;
    return rt_tcp_connect(host, port);
#else
    struct addrinfo hints;
    struct addrinfo *res = NULL;
    char portbuf[16];
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    snprintf(portbuf, sizeof(portbuf), "%d", port);
    if (getaddrinfo(host, portbuf, &hints, &res) != 0 || !res) {
        return -1;
    }
    int fd = -1;
    for (struct addrinfo *p = res; p; p = p->ai_next) {
        fd = (int)socket(p->ai_family, p->ai_socktype, p->ai_protocol);
        if (fd < 0) {
            continue;
        }
        nyra_sock_set_nonblock(fd);
        int rc = connect(fd, p->ai_addr, (int)p->ai_addrlen);
        if (rc == 0) {
            int flags = fcntl(fd, F_GETFL, 0);
            if (flags >= 0) {
                fcntl(fd, F_SETFL, flags & ~O_NONBLOCK);
            }
            break;
        }
        if (errno == EINPROGRESS) {
            fd = tcp_finish_connect(fd, timeout_ms);
            if (fd >= 0) {
                break;
            }
            continue;
        }
        nyra_sock_close(fd);
        fd = -1;
    }
    freeaddrinfo(res);
    return fd;
#endif
}

char *rt_dns_lookup(const char *host) {
    if (!host || host[0] == '\0') {
        return NULL;
    }
#if defined(_WIN32)
    nyra_winsock_ensure();
#endif
    struct addrinfo hints;
    struct addrinfo *res = NULL;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    if (getaddrinfo(host, NULL, &hints, &res) != 0 || !res) {
        return NULL;
    }
    size_t cap = 256;
    size_t len = 0;
    char *out = (char *)malloc(cap);
    if (!out) {
        freeaddrinfo(res);
        return NULL;
    }
    out[0] = '\0';
    for (struct addrinfo *p = res; p; p = p->ai_next) {
        char ip[INET6_ADDRSTRLEN];
        const void *addr = NULL;
        if (p->ai_family == AF_INET) {
            addr = &((struct sockaddr_in *)p->ai_addr)->sin_addr;
        } else if (p->ai_family == AF_INET6) {
            addr = &((struct sockaddr_in6 *)p->ai_addr)->sin6_addr;
        } else {
            continue;
        }
        if (!inet_ntop(p->ai_family, addr, ip, sizeof(ip))) {
            continue;
        }
        size_t iplen = strlen(ip);
        if (len > 0) {
            if (len + 1 >= cap) {
                cap *= 2;
                char *n = (char *)realloc(out, cap);
                if (!n) {
                    break;
                }
                out = n;
            }
            out[len++] = '\n';
            out[len] = '\0';
        }
        if (len + iplen + 1 >= cap) {
            cap = len + iplen + 64;
            char *n = (char *)realloc(out, cap);
            if (!n) {
                break;
            }
            out = n;
        }
        memcpy(out + len, ip, iplen);
        len += iplen;
        out[len] = '\0';
    }
    freeaddrinfo(res);
    if (len == 0) {
        free(out);
        return NULL;
    }
    return out;
}

int rt_tcp_ping_ms(const char *host, int port, int timeout_ms) {
    extern int64_t instant_now(void);
    int64_t start = instant_now();
    int fd = rt_tcp_connect_timeout(host, port, timeout_ms);
    if (fd < 0) {
        return -1;
    }
    nyra_sock_close(fd);
    int64_t end = instant_now();
    int64_t delta = end - start;
    if (delta < 0) {
        return 0;
    }
    if (delta > 2147483647) {
        return 2147483647;
    }
    return (int)delta;
}

#if !defined(_WIN32)
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <sys/socket.h>
#include <unistd.h>
#include <netinet/in.h>

static uint16_t icmp_checksum(void *data, int len) {
    uint16_t *buf = (uint16_t *)data;
    uint32_t sum = 0;
    while (len > 1) {
        sum += *buf++;
        len -= 2;
    }
    if (len == 1) {
        sum += *(uint8_t *)buf;
    }
    sum = (sum >> 16) + (sum & 0xffff);
    sum += (sum >> 16);
    return (uint16_t)(~sum);
}

static int icmp_delta_ms(int64_t start, int64_t end) {
    int64_t delta = end - start;
    if (delta < 0) {
        return 0;
    }
    if (delta > 2147483647) {
        return 2147483647;
    }
    return (int)delta;
}

static int icmp_resolve_v4(const char *host, struct sockaddr_in *out) {
    struct addrinfo hints;
    struct addrinfo *res = NULL;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_DGRAM;
    if (getaddrinfo(host, NULL, &hints, &res) != 0 || !res) {
        return -1;
    }
    memcpy(out, res->ai_addr, sizeof(*out));
    freeaddrinfo(res);
    return 0;
}

static int icmp_parse_time_ms(const char *output) {
    const char *p = strstr(output, "time=");
    if (!p) {
        return -1;
    }
    p += 5;
    double ms = 0.0;
    if (sscanf(p, "%lf", &ms) != 1) {
        return -1;
    }
    if (ms < 0.0) {
        return 0;
    }
    if (ms > 2147483647.0) {
        return 2147483647;
    }
    return (int)(ms + 0.5);
}

#if defined(__linux__)
#include <netinet/ip_icmp.h>

static int rt_icmp_ping_socket_ms(const char *host, int timeout_ms, int sock_type) {
    extern int64_t instant_now(void);
    struct sockaddr_in addr;
    if (icmp_resolve_v4(host, &addr) != 0) {
        return -1;
    }
    int sock = (int)socket(AF_INET, sock_type, IPPROTO_ICMP);
    if (sock < 0) {
        return -2;
    }
    char packet[64];
    memset(packet, 0, sizeof(packet));
    struct icmphdr *icmp = (struct icmphdr *)packet;
    icmp->type = ICMP_ECHO;
    icmp->code = 0;
    icmp->un.echo.id = (uint16_t)getpid();
    icmp->un.echo.sequence = 1;
    icmp->checksum = 0;
    icmp->checksum = icmp_checksum(packet, 64);
    int64_t start = instant_now();
    if (sendto(sock, packet, 64, 0, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        close(sock);
        return -1;
    }
    struct pollfd pfd;
    pfd.fd = sock;
    pfd.events = POLLIN;
    char reply[128];
    if (poll(&pfd, 1, timeout_ms) <= 0) {
        close(sock);
        return -1;
    }
    socklen_t slen = sizeof(struct sockaddr_in);
    struct sockaddr_in from;
    if (recvfrom(sock, reply, sizeof(reply), 0, (struct sockaddr *)&from, &slen) < 0) {
        close(sock);
        return -1;
    }
    close(sock);
    return icmp_delta_ms(start, instant_now());
}

#elif defined(__APPLE__)
#include <netinet/in_systm.h>
#include <netinet/ip.h>
#include <netinet/ip_icmp.h>

static int rt_icmp_ping_socket_ms(const char *host, int timeout_ms, int sock_type) {
    extern int64_t instant_now(void);
    (void)sock_type;
    struct sockaddr_in addr;
    if (icmp_resolve_v4(host, &addr) != 0) {
        return -1;
    }
    int sock = (int)socket(AF_INET, SOCK_RAW, IPPROTO_ICMP);
    if (sock < 0) {
        return -2;
    }
    char packet[64];
    memset(packet, 0, sizeof(packet));
    struct icmp *icmp = (struct icmp *)packet;
    icmp->icmp_type = ICMP_ECHO;
    icmp->icmp_code = 0;
    icmp->icmp_id = (uint16_t)getpid();
    icmp->icmp_seq = 1;
    icmp->icmp_cksum = 0;
    icmp->icmp_cksum = icmp_checksum(packet, 64);
    int64_t start = instant_now();
    if (sendto(sock, packet, 64, 0, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        close(sock);
        return -1;
    }
    struct pollfd pfd;
    pfd.fd = sock;
    pfd.events = POLLIN;
    char reply[128];
    if (poll(&pfd, 1, timeout_ms) <= 0) {
        close(sock);
        return -1;
    }
    socklen_t slen = sizeof(struct sockaddr_in);
    struct sockaddr_in from;
    if (recvfrom(sock, reply, sizeof(reply), 0, (struct sockaddr *)&from, &slen) < 0) {
        close(sock);
        return -1;
    }
    close(sock);
    return icmp_delta_ms(start, instant_now());
}
#endif

int rt_icmp_ping_ms(const char *host, int timeout_ms) {
    if (!host || host[0] == '\0' || timeout_ms <= 0) {
        return -1;
    }
#if defined(__linux__) || defined(__APPLE__)
    if (geteuid() == 0) {
        return rt_icmp_ping_socket_ms(host, timeout_ms, SOCK_RAW);
    }
#if defined(__linux__)
    int dgram = rt_icmp_ping_socket_ms(host, timeout_ms, SOCK_DGRAM);
    if (dgram >= 0 || dgram == -1) {
        return dgram;
    }
#endif
    return -2;
#else
    (void)host;
    (void)timeout_ms;
    return -2;
#endif
}

int rt_icmp_ping_system_ms(const char *host, int timeout_ms) {
    if (!host || host[0] == '\0' || timeout_ms <= 0) {
        return -1;
    }
    char cmd[512];
#if defined(__APPLE__)
    snprintf(cmd, sizeof(cmd), "ping -c 1 -W %d '%s' 2>/dev/null", timeout_ms, host);
#else
    int sec = (timeout_ms + 999) / 1000;
    if (sec < 1) {
        sec = 1;
    }
    snprintf(cmd, sizeof(cmd), "ping -c 1 -W %d '%s' 2>/dev/null", sec, host);
#endif
    FILE *fp = popen(cmd, "r");
    if (!fp) {
        return -1;
    }
    char line[256];
    char output[1024];
    output[0] = '\0';
    while (fgets(line, sizeof(line), fp) != NULL) {
        if (strlen(output) + strlen(line) + 1 < sizeof(output)) {
            strcat(output, line);
        }
    }
    int rc = pclose(fp);
    if (rc != 0) {
        return -1;
    }
    return icmp_parse_time_ms(output);
}

int rt_icmp_capable(void) {
#if defined(_WIN32)
    return -1;
#elif defined(__linux__) || defined(__APPLE__)
    if (geteuid() == 0) {
        return 1;
    }
#if defined(__linux__)
    int sock = (int)socket(AF_INET, SOCK_DGRAM, IPPROTO_ICMP);
    if (sock >= 0) {
        close(sock);
        return 1;
    }
#endif
    return 0;
#else
    return -1;
#endif
}

#else
int rt_icmp_ping_ms(const char *host, int timeout_ms) {
    (void)host;
    (void)timeout_ms;
    return -2;
}

int rt_icmp_ping_system_ms(const char *host, int timeout_ms) {
    (void)host;
    (void)timeout_ms;
    return -1;
}

int rt_icmp_capable(void) {
    return -1;
}
#endif
