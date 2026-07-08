#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Client TLS ABI lives in libnyra_rt_tls.a (rustls). This C unit keeps optional
 * OpenSSL-backed server listen/accept/self-signed helpers only. */
#ifndef NYRA_TLS_RUSTLS_CLIENT
#define NYRA_TLS_RUSTLS_CLIENT 1
#endif

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
    size_t n = strlen(msg);
    if (n >= sizeof(g_tls_last_error)) {
        n = sizeof(g_tls_last_error) - 1;
    }
    memcpy(g_tls_last_error, msg, n);
    g_tls_last_error[n] = '\0';
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
    tls_set_error("TLS server requires OpenSSL (client HTTPS uses built-in rustls)");
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

/* Client symbols (tls_available, connect/read/write/close/upgrade/last_error)
 * are provided by libnyra_rt_tls.a when NYRA_TLS_RUSTLS_CLIENT=1. */

#if !NYRA_TLS_RUSTLS_CLIENT
#error "OpenSSL-only TLS client path is removed; build with rustls (NYRA_TLS_RUSTLS_CLIENT=1)"
#endif

/* Server-side last_error buffer used by validate/listen stubs when rustls
 * owns the client rt_tls_last_error symbol. Exported under a distinct name. */
const char *rt_tls_server_last_error(void) {
    return g_tls_last_error;
}

int rt_tls_validate_pem_files(const char *cert_pem_path, const char *key_pem_path) {
#if !NYRA_TLS_OPENSSL
    (void)cert_pem_path;
    (void)key_pem_path;
    tls_set_error("TLS server PEM validation requires OpenSSL");
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
    tls_set_error("TLS server requires OpenSSL (HTTPS clients use built-in rustls)");
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
        tls_set_error("TLS listener table full");
    }
    return handle;
#endif
}

int rt_tls_accept(int listener_handle) {
#if !NYRA_TLS_OPENSSL
    (void)listener_handle;
    tls_set_error("TLS server requires OpenSSL (HTTPS clients use built-in rustls)");
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
    tls_set_error("self-signed cert generation requires OpenSSL");
    return -1;
#else
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
    if (PEM_write_bio_X509(cert_bio, x509) != 1
        || PEM_write_bio_PrivateKey(key_bio, pkey, NULL, NULL, 0, NULL, NULL) != 1) {
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

/* Close OpenSSL-backed server connections (accept slots). Client handles are
 * closed via rt_tls_close from libnyra_rt_tls.a. */
void rt_tls_server_conn_close(int handle) {
#if !NYRA_TLS_OPENSSL
    (void)handle;
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
