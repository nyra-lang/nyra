#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#if defined(_WIN32)
/* MSYS2 zlib zconf.h enables HAVE_UNISTD_H; LLVM clang on CI may not resolve it. */
#undef HAVE_UNISTD_H
#endif
#include <zlib.h>

static const char nyra_hex_digits[] = "0123456789abcdef";

static char *buf_to_hex(const uint8_t *buf, size_t len) {
    char *out = (char *)malloc(len * 2 + 1);
    if (!out) {
        return NULL;
    }
    for (size_t i = 0; i < len; i++) {
        out[i * 2] = nyra_hex_digits[buf[i] >> 4];
        out[i * 2 + 1] = nyra_hex_digits[buf[i] & 0x0f];
    }
    out[len * 2] = '\0';
    return out;
}

static int hex_nibble(char c) {
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return c - 'a' + 10;
    }
    if (c >= 'A' && c <= 'F') {
        return c - 'A' + 10;
    }
    return -1;
}

static uint8_t *hex_to_buf(const char *hex, size_t *out_len) {
    size_t n = hex ? strlen(hex) : 0;
    if (n % 2 != 0) {
        return NULL;
    }
    uint8_t *buf = (uint8_t *)malloc(n / 2);
    if (!buf) {
        return NULL;
    }
    for (size_t i = 0; i < n / 2; i++) {
        int hi = hex_nibble(hex[i * 2]);
        int lo = hex_nibble(hex[i * 2 + 1]);
        if (hi < 0 || lo < 0) {
            free(buf);
            return NULL;
        }
        buf[i] = (uint8_t)((hi << 4) | lo);
    }
    *out_len = n / 2;
    return buf;
}

char *gzip_compress_hex(const char *data) {
    if (!data) {
        return NULL;
    }
    uLong src_len = (uLong)strlen(data);
    uLong bound = compressBound(src_len);
    uint8_t *tmp = (uint8_t *)malloc(bound + 32);
    if (!tmp) {
        return NULL;
    }
    z_stream strm;
    memset(&strm, 0, sizeof(strm));
    strm.next_in = (Bytef *)data;
    strm.avail_in = (uInt)src_len;
    strm.next_out = (Bytef *)tmp;
    strm.avail_out = (uInt)(bound + 32);
    if (deflateInit2(&strm, Z_BEST_COMPRESSION, Z_DEFLATED, 15 + 16, 8, Z_DEFAULT_STRATEGY) != Z_OK) {
        free(tmp);
        return NULL;
    }
    if (deflate(&strm, Z_FINISH) != Z_STREAM_END) {
        deflateEnd(&strm);
        free(tmp);
        return NULL;
    }
    uLong dst_len = (uLong)strm.total_out;
    deflateEnd(&strm);
    char *hex = buf_to_hex(tmp, dst_len);
    free(tmp);
    return hex;
}

char *gzip_decompress_hex(const char *hex) {
    if (!hex) {
        return NULL;
    }
    size_t comp_len = 0;
    uint8_t *comp = hex_to_buf(hex, &comp_len);
    if (!comp) {
        return NULL;
    }
    uLong dst_len = (uLong)(comp_len * 8 + 256);
    uint8_t *out = (uint8_t *)malloc(dst_len);
    if (!out) {
        free(comp);
        return NULL;
    }
    z_stream strm;
    memset(&strm, 0, sizeof(strm));
    strm.next_in = comp;
    strm.avail_in = (uInt)comp_len;
    strm.next_out = out;
    strm.avail_out = (uInt)dst_len;
    if (inflateInit2(&strm, 15 + 16) != Z_OK) {
        free(out);
        free(comp);
        return NULL;
    }
    int rc = inflate(&strm, Z_FINISH);
    if (rc != Z_STREAM_END) {
        inflateEnd(&strm);
        free(out);
        free(comp);
        return NULL;
    }
    dst_len = (uLong)strm.total_out;
    inflateEnd(&strm);
    free(comp);
    char *s = (char *)malloc(dst_len + 1);
    if (!s) {
        free(out);
        return NULL;
    }
    memcpy(s, out, dst_len);
    s[dst_len] = '\0';
    free(out);
    return s;
}

int gzip_file(const char *src, const char *dst) {
    FILE *in = fopen(src, "rb");
    if (!in) {
        return -1;
    }
    gzFile out = gzopen(dst, "wb9");
    if (!out) {
        fclose(in);
        return -1;
    }
    char buf[65536];
    for (;;) {
        size_t n = fread(buf, 1, sizeof(buf), in);
        if (n == 0) {
            break;
        }
        if (gzwrite(out, buf, (unsigned)n) != (int)n) {
            gzclose(out);
            fclose(in);
            return -1;
        }
    }
    fclose(in);
    if (gzclose(out) != Z_OK) {
        return -1;
    }
    return 0;
}

int gunzip_file(const char *src, const char *dst) {
    gzFile in = gzopen(src, "rb");
    if (!in) {
        return -1;
    }
    FILE *out = fopen(dst, "wb");
    if (!out) {
        gzclose(in);
        return -1;
    }
    char buf[65536];
    for (;;) {
        int n = gzread(in, buf, (unsigned)sizeof(buf));
        if (n <= 0) {
            break;
        }
        if (fwrite(buf, 1, (size_t)n, out) != (size_t)n) {
            gzclose(in);
            fclose(out);
            return -1;
        }
    }
    int err = 0;
    const char *msg = gzerror(in, &err);
    (void)msg;
    gzclose(in);
    fclose(out);
    return (err == Z_STREAM_END || err == Z_OK) ? 0 : -1;
}

char *flate_compress_hex(const char *data) {
    if (!data) {
        return NULL;
    }
    uLong src_len = (uLong)strlen(data);
    uLong bound = compressBound(src_len);
    uint8_t *tmp = (uint8_t *)malloc(bound);
    if (!tmp) {
        return NULL;
    }
    z_stream strm;
    memset(&strm, 0, sizeof(strm));
    strm.next_in = (Bytef *)data;
    strm.avail_in = (uInt)src_len;
    strm.next_out = (Bytef *)tmp;
    strm.avail_out = (uInt)bound;
    if (deflateInit(&strm, Z_BEST_COMPRESSION) != Z_OK) {
        free(tmp);
        return NULL;
    }
    if (deflate(&strm, Z_FINISH) != Z_STREAM_END) {
        deflateEnd(&strm);
        free(tmp);
        return NULL;
    }
    uLong dst_len = (uLong)strm.total_out;
    deflateEnd(&strm);
    char *hex = buf_to_hex(tmp, dst_len);
    free(tmp);
    return hex;
}

char *flate_decompress_hex(const char *hex) {
    if (!hex) {
        return NULL;
    }
    size_t comp_len = 0;
    uint8_t *comp = hex_to_buf(hex, &comp_len);
    if (!comp) {
        return NULL;
    }
    uLong dst_len = (uLong)(comp_len * 8 + 256);
    uint8_t *out = (uint8_t *)malloc(dst_len);
    if (!out) {
        free(comp);
        return NULL;
    }
    z_stream strm;
    memset(&strm, 0, sizeof(strm));
    strm.next_in = comp;
    strm.avail_in = (uInt)comp_len;
    strm.next_out = out;
    strm.avail_out = (uInt)dst_len;
    if (inflateInit(&strm) != Z_OK) {
        free(out);
        free(comp);
        return NULL;
    }
    int rc = inflate(&strm, Z_FINISH);
    if (rc != Z_STREAM_END) {
        inflateEnd(&strm);
        free(out);
        free(comp);
        return NULL;
    }
    dst_len = (uLong)strm.total_out;
    inflateEnd(&strm);
    free(comp);
    char *s = (char *)malloc(dst_len + 1);
    if (!s) {
        free(out);
        return NULL;
    }
    memcpy(s, out, dst_len);
    s[dst_len] = '\0';
    free(out);
    return s;
}
