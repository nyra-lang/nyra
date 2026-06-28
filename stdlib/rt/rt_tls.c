#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(__has_include)
#if __has_include(<openssl/ssl.h>)
#define NYRA_TLS_OPENSSL 1
#include <openssl/err.h>
#include <openssl/evp.h>
#include <openssl/pem.h>
#include <openssl/ssl.h>
#include <openssl/x509.h>
#endif
#endif

#ifndef NYRA_TLS_OPENSSL
#define NYRA_TLS_OPENSSL 0
#endif

#if !NYRA_TLS_OPENSSL
typedef struct NyraSslOpaque SSL;
typedef struct NyraSslCtxOpaque SSL_CTX;
#endif

#define NYRA_TLS_MAX 32
#define NYRA_TLS_HANDLE_BASE 0x100000

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
    strncpy(g_tls_last_error, msg, sizeof(g_tls_last_error) - 1);
    g_tls_last_error[sizeof(g_tls_last_error) - 1] = '\0';
}

static void tls_set_openssl_error(const char *prefix) {
#if NYRA_TLS_OPENSSL
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
#else
    (void)prefix;
    tls_set_error("OpenSSL not available");
#endif
}

static int tls_ctx_configure_verify(SSL_CTX *ctx, const char *ca_path, int verify_peer) {
#if !NYRA_TLS_OPENSSL
    (void)ctx;
    (void)ca_path;
    (void)verify_peer;
    return -1;
#else
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
#endif
}

static void tls_init_once(void) {
    if (g_tls_inited) {
        return;
    }
#if NYRA_TLS_OPENSSL
    SSL_library_init();
    SSL_load_error_strings();
    OpenSSL_add_all_algorithms();
#endif
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

extern int rt_tcp_connect(const char *host, int port);
extern void rt_tcp_close(int fd);
extern int rt_tcp_listen(const char *host, int port);
extern int rt_tcp_accept(int listener_fd);

#define NYRA_TLS_LISTEN_MAX 8

typedef struct {
    int used;
    int plain_fd;
    SSL_CTX *ctx;
} NyraTlsListener;

static NyraTlsListener g_tls_listeners[NYRA_TLS_LISTEN_MAX];

int tls_available(void) {
    return NYRA_TLS_OPENSSL ? 1 : 0;
}

const char *rt_tls_last_error(void) {
    return g_tls_last_error;
}

int rt_tls_validate_pem_files(const char *cert_pem_path, const char *key_pem_path) {
#if !NYRA_TLS_OPENSSL
    (void)cert_pem_path;
    (void)key_pem_path;
    tls_set_error("OpenSSL not available");
    return -1;
#else
    tls_init_once();
    if (!cert_pem_path || cert_pem_path[0] == '\0' || !key_pem_path || key_pem_path[0] == '\0') {
        tls_set_error("certificate and key paths are required");
        return -1;
    }
    SSL_CTX *ctx = SSL_CTX_new(TLS_server_method());
    if (!ctx) {
        tls_set_openssl_error("SSL_CTX_new failed");
        return -1;
    }
    if (SSL_CTX_use_certificate_file(ctx, cert_pem_path, SSL_FILETYPE_PEM) != 1) {
        SSL_CTX_free(ctx);
        tls_set_openssl_error("invalid certificate PEM");
        return -2;
    }
    if (SSL_CTX_use_PrivateKey_file(ctx, key_pem_path, SSL_FILETYPE_PEM) != 1) {
        SSL_CTX_free(ctx);
        tls_set_openssl_error("invalid private key PEM");
        return -3;
    }
    if (SSL_CTX_check_private_key(ctx) != 1) {
        SSL_CTX_free(ctx);
        tls_set_error("certificate and private key do not match");
        return -4;
    }
    SSL_CTX_free(ctx);
    tls_set_error(NULL);
    return 0;
#endif
}

int rt_tls_connect_ex(const char *host, int port, const char *ca_path, int verify_peer) {
#if !NYRA_TLS_OPENSSL
    (void)host;
    (void)port;
    (void)ca_path;
    (void)verify_peer;
    tls_set_error("OpenSSL not available");
    return -1;
#else
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
#endif
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

int rt_tls_upgrade_client_ex(int plain_fd, const char *hostname, const char *ca_path, int verify_peer) {
#if !NYRA_TLS_OPENSSL
    (void)plain_fd;
    (void)hostname;
    (void)ca_path;
    (void)verify_peer;
    tls_set_error("OpenSSL not available");
    return -1;
#else
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
#endif
}

int rt_tls_upgrade_client(int plain_fd, const char *hostname) {
    return rt_tls_upgrade_client_ex(plain_fd, hostname, NULL, 0);
}

int rt_tls_upgrade_client_verify(int plain_fd, const char *hostname) {
    return rt_tls_upgrade_client_ex(plain_fd, hostname, NULL, 1);
}

char *rt_tls_read(int handle, int max_bytes) {
#if !NYRA_TLS_OPENSSL
    (void)handle;
    (void)max_bytes;
    return NULL;
#else
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
#endif
}

int rt_tls_write_bytes(int handle, const char *data, int len) {
#if !NYRA_TLS_OPENSSL
    (void)handle;
    (void)data;
    (void)len;
    return -1;
#else
    NyraTlsSlot *slot = tls_slot_from_handle(handle);
    if (!slot || !data || len < 0) {
        return -1;
    }
    int n = SSL_write(slot->ssl, data, len);
    return (n == len) ? 0 : -1;
#endif
}

int rt_tls_write(int handle, const char *data) {
#if !NYRA_TLS_OPENSSL
    (void)handle;
    (void)data;
    return -1;
#else
    if (!data) {
        return -1;
    }
    return rt_tls_write_bytes(handle, data, (int)strlen(data));
#endif
}

int rt_tls_read_bytes(int handle, char *buf, int len) {
#if !NYRA_TLS_OPENSSL
    (void)handle;
    (void)buf;
    (void)len;
    return -1;
#else
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
#endif
}

void rt_tls_close(int handle) {
#if !NYRA_TLS_OPENSSL
    (void)handle;
    return;
#else
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
#endif
}

static int tls_listener_alloc(int plain_fd, SSL_CTX *ctx) {
    for (int i = 0; i < NYRA_TLS_LISTEN_MAX; i++) {
        if (!g_tls_listeners[i].used) {
            g_tls_listeners[i].used = 1;
            g_tls_listeners[i].plain_fd = plain_fd;
            g_tls_listeners[i].ctx = ctx;
            return 0x200000 + i;
        }
    }
    return -1;
}

static NyraTlsListener *tls_listener_from_handle(int handle) {
    if (handle < 0x200000) {
        return NULL;
    }
    int idx = handle - 0x200000;
    if (idx < 0 || idx >= NYRA_TLS_LISTEN_MAX || !g_tls_listeners[idx].used) {
        return NULL;
    }
    return &g_tls_listeners[idx];
}

int rt_tls_listen(const char *cert_pem_path, const char *key_pem_path, const char *host, int port) {
#if !NYRA_TLS_OPENSSL
    (void)cert_pem_path;
    (void)key_pem_path;
    (void)host;
    (void)port;
    return -1;
#else
    tls_init_once();
    if (!cert_pem_path || !key_pem_path) {
        tls_set_error("certificate and key paths are required");
        return -1;
    }
    if (rt_tls_validate_pem_files(cert_pem_path, key_pem_path) != 0) {
        return -1;
    }
    int fd = rt_tcp_listen(host, port);
    if (fd < 0) {
        tls_set_error("TCP listen failed");
        return -1;
    }
    SSL_CTX *ctx = SSL_CTX_new(TLS_server_method());
    if (!ctx) {
        rt_tcp_close(fd);
        tls_set_openssl_error("SSL_CTX_new failed");
        return -1;
    }
    if (SSL_CTX_use_certificate_file(ctx, cert_pem_path, SSL_FILETYPE_PEM) != 1) {
        SSL_CTX_free(ctx);
        rt_tcp_close(fd);
        tls_set_openssl_error("invalid certificate PEM");
        return -1;
    }
    if (SSL_CTX_use_PrivateKey_file(ctx, key_pem_path, SSL_FILETYPE_PEM) != 1) {
        SSL_CTX_free(ctx);
        rt_tcp_close(fd);
        tls_set_openssl_error("invalid private key PEM");
        return -1;
    }
    if (SSL_CTX_check_private_key(ctx) != 1) {
        SSL_CTX_free(ctx);
        rt_tcp_close(fd);
        tls_set_error("certificate and private key do not match");
        return -1;
    }
    int handle = tls_listener_alloc(fd, ctx);
    if (handle < 0) {
        SSL_CTX_free(ctx);
        rt_tcp_close(fd);
    }
    return handle;
#endif
}

int rt_tls_accept(int listener_handle) {
#if !NYRA_TLS_OPENSSL
    (void)listener_handle;
    return -1;
#else
    NyraTlsListener *ln = tls_listener_from_handle(listener_handle);
    if (!ln) {
        return -1;
    }
    int client_fd = rt_tcp_accept(ln->plain_fd);
    if (client_fd < 0) {
        return -1;
    }
    SSL *ssl = SSL_new(ln->ctx);
    if (!ssl) {
        rt_tcp_close(client_fd);
        return -1;
    }
    SSL_set_fd(ssl, client_fd);
    if (SSL_accept(ssl) != 1) {
        SSL_free(ssl);
        rt_tcp_close(client_fd);
        return -1;
    }
    int handle = tls_slot_alloc(client_fd, ssl, NULL);
    if (handle < 0) {
        SSL_free(ssl);
        rt_tcp_close(client_fd);
    }
    return handle;
#endif
}

void rt_tls_listener_close(int listener_handle) {
#if !NYRA_TLS_OPENSSL
    (void)listener_handle;
    return;
#else
    NyraTlsListener *ln = tls_listener_from_handle(listener_handle);
    if (!ln) {
        return;
    }
    if (ln->ctx) {
        SSL_CTX_free(ln->ctx);
        ln->ctx = NULL;
    }
    if (ln->plain_fd >= 0) {
        rt_tcp_close(ln->plain_fd);
        ln->plain_fd = -1;
    }
    ln->used = 0;
#endif
}

int rt_tls_gen_self_signed(const char *cert_path, const char *key_path, const char *common_name) {
#if !NYRA_TLS_OPENSSL
    (void)cert_path;
    (void)key_path;
    (void)common_name;
    return -1;
#else
    extern int write_file(const char *path, const char *content);
    if (!cert_path || !key_path || !common_name || common_name[0] == '\0') {
        return -1;
    }
    tls_init_once();
    EVP_PKEY *pkey = NULL;
    EVP_PKEY_CTX *pctx = EVP_PKEY_CTX_new_id(EVP_PKEY_RSA, NULL);
    if (!pctx || EVP_PKEY_keygen_init(pctx) <= 0) {
        EVP_PKEY_CTX_free(pctx);
        return -1;
    }
    if (EVP_PKEY_CTX_set_rsa_keygen_bits(pctx, 2048) <= 0) {
        EVP_PKEY_CTX_free(pctx);
        return -1;
    }
    if (EVP_PKEY_keygen(pctx, &pkey) <= 0) {
        EVP_PKEY_CTX_free(pctx);
        return -1;
    }
    EVP_PKEY_CTX_free(pctx);
    X509 *x509 = X509_new();
    if (!x509) {
        EVP_PKEY_free(pkey);
        return -1;
    }
    ASN1_INTEGER_set(X509_get_serialNumber(x509), 1);
    X509_gmtime_adj(X509_getm_notBefore(x509), 0);
    X509_gmtime_adj(X509_getm_notAfter(x509), 31536000L);
    X509_set_pubkey(x509, pkey);
    X509_NAME *name = X509_get_subject_name(x509);
    X509_NAME_add_entry_by_txt(name, "CN", MBSTRING_ASC, (const unsigned char *)common_name, -1, -1, 0);
    X509_set_issuer_name(x509, name);
    if (X509_sign(x509, pkey, EVP_sha256()) <= 0) {
        X509_free(x509);
        EVP_PKEY_free(pkey);
        return -1;
    }
    BIO *cert_bio = BIO_new_file(cert_path, "w");
    BIO *key_bio = BIO_new_file(key_path, "w");
    if (!cert_bio || !key_bio) {
        BIO_free_all(cert_bio);
        BIO_free_all(key_bio);
        X509_free(x509);
        EVP_PKEY_free(pkey);
        return -2;
    }
    if (PEM_write_bio_X509(cert_bio, x509) != 1 || PEM_write_bio_PrivateKey(key_bio, pkey, NULL, NULL, 0, NULL, NULL) != 1) {
        BIO_free_all(cert_bio);
        BIO_free_all(key_bio);
        X509_free(x509);
        EVP_PKEY_free(pkey);
        return -2;
    }
    BIO_free_all(cert_bio);
    BIO_free_all(key_bio);
    X509_free(x509);
    EVP_PKEY_free(pkey);
    return 0;
#endif
}
