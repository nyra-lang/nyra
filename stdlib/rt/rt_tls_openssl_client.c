/*
 * Optional OpenSSL TLS *client* — linked only when `tls openssl` is selected.
 * Default HTTPS uses libnyra_rt_tls.a (rustls). Excluded from the fat prebuilt
 * runtime archive to avoid duplicate symbols with rustls.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(__has_include)
#if __has_include(<openssl/ssl.h>)
#define NYRA_TLS_OPENSSL_CLIENT 1
#include <openssl/err.h>
#include <openssl/ssl.h>
#include <openssl/x509.h>
#endif
#endif

#ifndef NYRA_TLS_OPENSSL_CLIENT
#define NYRA_TLS_OPENSSL_CLIENT 0
#endif

#define NYRA_TLS_MAX 32
#define NYRA_TLS_HANDLE_BASE 0x100000

extern int rt_tcp_connect(const char *host, int port);
extern void rt_tcp_close(int fd);

#if !NYRA_TLS_OPENSSL_CLIENT

static char g_tls_last_error[256] =
    "OpenSSL headers not found — install OpenSSL or use `tls rustls` in nyra.mod";

int tls_available(void) { return 0; }
const char *rt_tls_last_error(void) { return g_tls_last_error; }
int rt_tls_connect_ex(const char *h, int p, const char *c, int v) {
    (void)h;
    (void)p;
    (void)c;
    (void)v;
    return -1;
}
int rt_tls_connect(const char *h, int p) { return rt_tls_connect_ex(h, p, NULL, 0); }
int rt_tls_connect_verify(const char *h, int p) { return rt_tls_connect_ex(h, p, NULL, 1); }
int rt_tls_connect_ca(const char *h, int p, const char *c) {
    return rt_tls_connect_ex(h, p, c, 1);
}
int rt_tls_upgrade_client_ex(int fd, const char *hn, const char *c, int v) {
    (void)fd;
    (void)hn;
    (void)c;
    (void)v;
    return -1;
}
int rt_tls_upgrade_client(int fd, const char *hn) {
    return rt_tls_upgrade_client_ex(fd, hn, NULL, 0);
}
int rt_tls_upgrade_client_verify(int fd, const char *hn) {
    return rt_tls_upgrade_client_ex(fd, hn, NULL, 1);
}
char *rt_tls_read(int handle, int max_bytes) {
    (void)handle;
    (void)max_bytes;
    return NULL;
}
int rt_tls_write_bytes(int handle, const char *data, int len) {
    (void)handle;
    (void)data;
    (void)len;
    return -1;
}
int rt_tls_write(int handle, const char *data) {
    (void)handle;
    (void)data;
    return -1;
}
int rt_tls_read_bytes(int handle, char *buf, int len) {
    (void)handle;
    (void)buf;
    (void)len;
    return -1;
}
void rt_tls_close(int handle) { (void)handle; }

#else

typedef struct {
    int used;
    int plain_fd;
    SSL *ssl;
    SSL_CTX *ctx;
} NyraTlsSlot;

static NyraTlsSlot g_tls[NYRA_TLS_MAX];
static int g_tls_inited;
static char g_tls_last_error[256];

static void tls_set_error(const char *msg) {
    if (!msg) {
        g_tls_last_error[0] = '\0';
        return;
    }
    size_t n = strlen(msg);
    if (n >= sizeof(g_tls_last_error)) {
        n = sizeof(g_tls_last_error) - 1;
    }
    memcpy(g_tls_last_error, msg, n);
    g_tls_last_error[n] = '\0';
}

static void tls_set_openssl_error(const char *prefix) {
    unsigned long err = ERR_get_error();
    if (err == 0) {
        tls_set_error(prefix ? prefix : "TLS error");
        return;
    }
    char buf[192];
    ERR_error_string_n(err, buf, sizeof(buf));
    if (prefix && prefix[0]) {
        snprintf(g_tls_last_error, sizeof(g_tls_last_error), "%s: %s", prefix, buf);
    } else {
        strncpy(g_tls_last_error, buf, sizeof(g_tls_last_error) - 1);
        g_tls_last_error[sizeof(g_tls_last_error) - 1] = '\0';
    }
}

static void tls_init_once(void) {
    if (g_tls_inited) {
        return;
    }
    SSL_library_init();
    SSL_load_error_strings();
    OpenSSL_add_all_algorithms();
    g_tls_inited = 1;
}

static int tls_slot_alloc(int plain_fd, SSL *ssl, SSL_CTX *ctx) {
    for (int i = 0; i < NYRA_TLS_MAX; i++) {
        if (!g_tls[i].used) {
            g_tls[i].used = 1;
            g_tls[i].plain_fd = plain_fd;
            g_tls[i].ssl = ssl;
            g_tls[i].ctx = ctx;
            return NYRA_TLS_HANDLE_BASE + i;
        }
    }
    return -1;
}

static NyraTlsSlot *tls_slot_from_handle(int handle) {
    if (handle < NYRA_TLS_HANDLE_BASE) {
        return NULL;
    }
    int idx = handle - NYRA_TLS_HANDLE_BASE;
    if (idx < 0 || idx >= NYRA_TLS_MAX || !g_tls[idx].used) {
        return NULL;
    }
    return &g_tls[idx];
}

static int tls_ctx_configure_verify(SSL_CTX *ctx, const char *ca_path, int verify_peer) {
    if (!verify_peer) {
        SSL_CTX_set_verify(ctx, SSL_VERIFY_NONE, NULL);
        return 0;
    }
    SSL_CTX_set_verify(ctx, SSL_VERIFY_PEER, NULL);
    if (ca_path && ca_path[0] != '\0') {
        if (SSL_CTX_load_verify_locations(ctx, ca_path, NULL) != 1) {
            tls_set_openssl_error("failed to load CA file");
            return -1;
        }
        return 0;
    }
    if (SSL_CTX_set_default_verify_paths(ctx) != 1) {
        tls_set_openssl_error("failed to load system CA store");
        return -1;
    }
    return 0;
}

int tls_available(void) { return 1; }
const char *rt_tls_last_error(void) { return g_tls_last_error; }

int rt_tls_connect_ex(const char *host, int port, const char *ca_path, int verify_peer) {
    tls_init_once();
    tls_set_error(NULL);
    if (!host || port <= 0) {
        tls_set_error("invalid host or port");
        return -1;
    }
    int fd = rt_tcp_connect(host, port);
    if (fd < 0) {
        tls_set_error("TCP connect failed");
        return -1;
    }
    SSL_CTX *ctx = SSL_CTX_new(TLS_client_method());
    if (!ctx) {
        rt_tcp_close(fd);
        tls_set_openssl_error("SSL_CTX_new failed");
        return -1;
    }
    if (tls_ctx_configure_verify(ctx, ca_path, verify_peer) != 0) {
        SSL_CTX_free(ctx);
        rt_tcp_close(fd);
        return -1;
    }
    SSL *ssl = SSL_new(ctx);
    if (!ssl) {
        SSL_CTX_free(ctx);
        rt_tcp_close(fd);
        tls_set_openssl_error("SSL_new failed");
        return -1;
    }
    SSL_set_tlsext_host_name(ssl, host);
    SSL_set_fd(ssl, fd);
    if (SSL_connect(ssl) != 1) {
        SSL_free(ssl);
        SSL_CTX_free(ctx);
        rt_tcp_close(fd);
        tls_set_openssl_error("TLS handshake failed");
        return -1;
    }
    if (verify_peer && SSL_get_verify_result(ssl) != X509_V_OK) {
        SSL_free(ssl);
        SSL_CTX_free(ctx);
        rt_tcp_close(fd);
        tls_set_error("certificate verification failed");
        return -1;
    }
    int handle = tls_slot_alloc(fd, ssl, ctx);
    if (handle < 0) {
        SSL_free(ssl);
        SSL_CTX_free(ctx);
        rt_tcp_close(fd);
        tls_set_error("TLS handle table full");
    }
    return handle;
}

int rt_tls_connect(const char *host, int port) {
    return rt_tls_connect_ex(host, port, NULL, 0);
}
int rt_tls_connect_verify(const char *host, int port) {
    return rt_tls_connect_ex(host, port, NULL, 1);
}
int rt_tls_connect_ca(const char *host, int port, const char *ca_path) {
    return rt_tls_connect_ex(host, port, ca_path, 1);
}

int rt_tls_upgrade_client_ex(
    int plain_fd,
    const char *hostname,
    const char *ca_path,
    int verify_peer
) {
    tls_init_once();
    tls_set_error(NULL);
    if (plain_fd < 0 || !hostname) {
        tls_set_error("invalid fd or hostname");
        return -1;
    }
    SSL_CTX *ctx = SSL_CTX_new(TLS_client_method());
    if (!ctx) {
        tls_set_openssl_error("SSL_CTX_new failed");
        return -1;
    }
    if (tls_ctx_configure_verify(ctx, ca_path, verify_peer) != 0) {
        SSL_CTX_free(ctx);
        return -1;
    }
    SSL *ssl = SSL_new(ctx);
    if (!ssl) {
        SSL_CTX_free(ctx);
        tls_set_openssl_error("SSL_new failed");
        return -1;
    }
    SSL_set_tlsext_host_name(ssl, hostname);
    SSL_set_fd(ssl, plain_fd);
    if (SSL_connect(ssl) != 1) {
        SSL_free(ssl);
        SSL_CTX_free(ctx);
        tls_set_openssl_error("TLS upgrade handshake failed");
        return -1;
    }
    if (verify_peer && SSL_get_verify_result(ssl) != X509_V_OK) {
        SSL_free(ssl);
        SSL_CTX_free(ctx);
        tls_set_error("certificate verification failed");
        return -1;
    }
    int handle = tls_slot_alloc(plain_fd, ssl, ctx);
    if (handle < 0) {
        SSL_free(ssl);
        SSL_CTX_free(ctx);
        tls_set_error("TLS handle table full");
        return -1;
    }
    return handle;
}

int rt_tls_upgrade_client(int plain_fd, const char *hostname) {
    return rt_tls_upgrade_client_ex(plain_fd, hostname, NULL, 0);
}
int rt_tls_upgrade_client_verify(int plain_fd, const char *hostname) {
    return rt_tls_upgrade_client_ex(plain_fd, hostname, NULL, 1);
}

char *rt_tls_read(int handle, int max_bytes) {
    NyraTlsSlot *slot = tls_slot_from_handle(handle);
    if (!slot || max_bytes <= 0) {
        return NULL;
    }
    if (max_bytes > 1024 * 1024) {
        max_bytes = 1024 * 1024;
    }
    char *buf = (char *)malloc((size_t)max_bytes + 1);
    if (!buf) {
        return NULL;
    }
    int n = SSL_read(slot->ssl, buf, max_bytes);
    if (n <= 0) {
        free(buf);
        return NULL;
    }
    buf[n] = '\0';
    return buf;
}

int rt_tls_write_bytes(int handle, const char *data, int len) {
    NyraTlsSlot *slot = tls_slot_from_handle(handle);
    if (!slot || !data || len < 0) {
        return -1;
    }
    int n = SSL_write(slot->ssl, data, len);
    return (n == len) ? 0 : -1;
}

int rt_tls_write(int handle, const char *data) {
    if (!data) {
        return -1;
    }
    return rt_tls_write_bytes(handle, data, (int)strlen(data));
}

int rt_tls_read_bytes(int handle, char *buf, int len) {
    NyraTlsSlot *slot = tls_slot_from_handle(handle);
    if (!slot || !buf || len <= 0) {
        return -1;
    }
    int got = 0;
    while (got < len) {
        int n = SSL_read(slot->ssl, buf + got, len - got);
        if (n <= 0) {
            return -1;
        }
        got += n;
    }
    return 0;
}

void rt_tls_close(int handle) {
    NyraTlsSlot *slot = tls_slot_from_handle(handle);
    if (!slot) {
        return;
    }
    if (slot->ssl) {
        SSL_shutdown(slot->ssl);
        SSL_free(slot->ssl);
        slot->ssl = NULL;
    }
    if (slot->ctx) {
        SSL_CTX_free(slot->ctx);
        slot->ctx = NULL;
    }
    if (slot->plain_fd >= 0) {
        rt_tcp_close(slot->plain_fd);
        slot->plain_fd = -1;
    }
    slot->used = 0;
}

#endif
