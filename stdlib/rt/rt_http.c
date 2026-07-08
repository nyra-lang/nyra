#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern int rt_tcp_connect(const char *host, int port);
extern char *rt_tcp_read(int fd, int max_bytes);
extern int rt_tcp_read_bytes(int fd, char *buf, int len);
extern int rt_tcp_write(int fd, const char *data);
extern void rt_tcp_close(int fd);
extern int tls_available(void);
extern int rt_tls_connect_verify(const char *host, int port);
extern int rt_tls_write(int handle, const char *data);
extern int rt_tls_read_bytes(int handle, char *buf, int len);
extern void rt_tls_close(int handle);

typedef struct {
    int tcp_fd;
    int tls_handle;
} HttpConn;

static int parse_url_full(
    const char *url,
    char *host,
    size_t hcap,
    int *port,
    char *path,
    size_t pcap,
    int *secure
) {
    const char *p = url;
    *port = 80;
    *secure = 0;
    host[0] = '\0';
    path[0] = '/';
    path[1] = '\0';
    if (strncmp(p, "https://", 8) == 0) {
        p += 8;
        *port = 443;
        *secure = 1;
    } else if (strncmp(p, "http://", 7) == 0) {
        p += 7;
    } else {
        return -1;
    }
    const char *slash = strchr(p, '/');
    const char *colon = strchr(p, ':');
    if (colon && (!slash || colon < slash)) {
        size_t hlen = (size_t)(colon - p);
        if (hlen >= hcap) {
            hlen = hcap - 1;
        }
        memcpy(host, p, hlen);
        host[hlen] = '\0';
        *port = atoi(colon + 1);
        if (slash) {
            snprintf(path, pcap, "%s", slash);
        }
    } else if (slash) {
        size_t hlen = (size_t)(slash - p);
        if (hlen >= hcap) {
            hlen = hcap - 1;
        }
        memcpy(host, p, hlen);
        host[hlen] = '\0';
        snprintf(path, pcap, "%s", slash);
    } else {
        snprintf(host, hcap, "%s", p);
    }
    return 0;
}

static void http_conn_init(HttpConn *conn) {
    conn->tcp_fd = -1;
    conn->tls_handle = -1;
}

static int http_conn_open(HttpConn *conn, const char *host, int port, int secure) {
    http_conn_init(conn);
    if (secure) {
        if (tls_available() == 0) {
            return -1;
        }
        conn->tls_handle = rt_tls_connect_verify(host, port);
        return conn->tls_handle < 0 ? -1 : 0;
    }
    conn->tcp_fd = rt_tcp_connect(host, port);
    return conn->tcp_fd < 0 ? -1 : 0;
}

static void http_conn_close(HttpConn *conn) {
    if (conn->tls_handle >= 0) {
        rt_tls_close(conn->tls_handle);
    } else if (conn->tcp_fd >= 0) {
        rt_tcp_close(conn->tcp_fd);
    }
    http_conn_init(conn);
}

static int http_conn_write(HttpConn *conn, const char *data) {
    if (conn->tls_handle >= 0) {
        return rt_tls_write(conn->tls_handle, data);
    }
    return rt_tcp_write(conn->tcp_fd, data);
}

static int http_conn_read(HttpConn *conn, char *buf, int len) {
    if (conn->tls_handle >= 0) {
        return rt_tls_read_bytes(conn->tls_handle, buf, len);
    }
    return rt_tcp_read_bytes(conn->tcp_fd, buf, len);
}

static int http_request_get(HttpConn *conn, const char *host, const char *path) {
    char req[1024];
    snprintf(
        req,
        sizeof(req),
        "GET %s HTTP/1.1\r\nHost: %s\r\nUser-Agent: Nyra/1.0\r\nAccept: */*\r\nConnection: close\r\n\r\n",
        path,
        host
    );
    return http_conn_write(conn, req);
}

static int http_status_from_buffer(const char *buf, size_t len) {
    if (!buf || len < 12 || strncmp(buf, "HTTP/", 5) != 0) {
        return 0;
    }
    const char *sp = memchr(buf, ' ', len);
    if (!sp) {
        return 0;
    }
    return atoi(sp + 1);
}

static int http_stream_body_to_file(HttpConn *conn, FILE *out) {
    char buf[8192];
    char pending[8192];
    size_t pending_len = 0;
    int headers_done = 0;
    int status = 0;

    for (;;) {
        int n = http_conn_read(conn, buf, (int)sizeof(buf));
        if (n < 0) {
            return -1;
        }
        if (n == 0) {
            break;
        }

        if (!headers_done) {
            size_t combined_cap = pending_len + (size_t)n;
            char *combined = (char *)malloc(combined_cap);
            if (!combined) {
                return -1;
            }
            if (pending_len > 0) {
                memcpy(combined, pending, pending_len);
            }
            memcpy(combined + pending_len, buf, (size_t)n);
            char *sep = NULL;
            for (size_t i = 0; i + 3 < combined_cap; i++) {
                if (combined[i] == '\r' && combined[i + 1] == '\n' && combined[i + 2] == '\r' &&
                    combined[i + 3] == '\n') {
                    sep = combined + i;
                    break;
                }
            }
            if (!sep) {
                if (combined_cap >= sizeof(pending)) {
                    free(combined);
                    return -1;
                }
                memcpy(pending, combined, combined_cap);
                pending_len = combined_cap;
                free(combined);
                continue;
            }
            status = http_status_from_buffer(combined, (size_t)(sep - combined));
            if (status != 200) {
                free(combined);
                return -1;
            }
            size_t body_off = (size_t)(sep - combined) + 4;
            size_t body_len = combined_cap - body_off;
            if (body_len > 0 && fwrite(combined + body_off, 1, body_len, out) != body_len) {
                free(combined);
                return -1;
            }
            free(combined);
            pending_len = 0;
            headers_done = 1;
            continue;
        }

        if (fwrite(buf, 1, (size_t)n, out) != (size_t)n) {
            return -1;
        }
    }

    return headers_done ? 0 : -1;
}

int http_download_file(const char *url, const char *path) {
    if (!url || !path) {
        return -1;
    }
    char host[256];
    char req_path[512];
    int port = 80;
    int secure = 0;
    if (parse_url_full(url, host, sizeof(host), &port, req_path, sizeof(req_path), &secure) != 0) {
        return -1;
    }
    HttpConn conn;
    if (http_conn_open(&conn, host, port, secure) != 0) {
        return -1;
    }
    if (http_request_get(&conn, host, req_path) != 0) {
        http_conn_close(&conn);
        return -1;
    }
    FILE *out = fopen(path, "wb");
    if (!out) {
        http_conn_close(&conn);
        return -1;
    }
    int rc = http_stream_body_to_file(&conn, out);
    fclose(out);
    http_conn_close(&conn);
    if (rc != 0) {
        remove(path);
    }
    return rc;
}

char *http_get(const char *url) {
    char host[256];
    char path[512];
    int port = 80;
    int secure = 0;
    if (parse_url_full(url, host, sizeof(host), &port, path, sizeof(path), &secure) != 0) {
        return NULL;
    }
    HttpConn conn;
    if (http_conn_open(&conn, host, port, secure) != 0) {
        return NULL;
    }
    if (http_request_get(&conn, host, path) != 0) {
        http_conn_close(&conn);
        return NULL;
    }
    char *raw = NULL;
    size_t raw_len = 0;
    size_t raw_cap = 0;
    char buf[8192];
    for (;;) {
        int n = http_conn_read(&conn, buf, (int)sizeof(buf));
        if (n < 0) {
            free(raw);
            http_conn_close(&conn);
            return NULL;
        }
        if (n == 0) {
            break;
        }
        if (raw_len + (size_t)n + 1 > raw_cap) {
            raw_cap = raw_cap == 0 ? 65536 : raw_cap * 2;
            char *next = (char *)realloc(raw, raw_cap);
            if (!next) {
                free(raw);
                http_conn_close(&conn);
                return NULL;
            }
            raw = next;
        }
        memcpy(raw + raw_len, buf, (size_t)n);
        raw_len += (size_t)n;
        raw[raw_len] = '\0';
    }
    http_conn_close(&conn);
    if (!raw) {
        return NULL;
    }
    char *body = strstr(raw, "\r\n\r\n");
    if (!body) {
        return raw;
    }
    body += 4;
    char *out = strdup(body);
    free(raw);
    return out;
}

int http_status(const char *response_header) {
    if (!response_header || strncmp(response_header, "HTTP/", 5) != 0) {
        return 0;
    }
    const char *sp = strchr(response_header, ' ');
    if (!sp) {
        return 0;
    }
    return atoi(sp + 1);
}
