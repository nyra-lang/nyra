// Nyra random — ChaCha20 CSPRNG seeded from OS / hardware entropy.
// Seeding: arc4random_buf / getentropy / BCryptGenRandom / RDRAND / /dev/urandom.
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#if defined(_WIN32)
#include <windows.h>
#include <bcrypt.h>
#include <process.h>
#define NYRA_GETPID() ((uint64_t)_getpid())
#else
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#define NYRA_GETPID() ((uint64_t)getpid())
#endif

#if defined(__APPLE__)
#include <sys/random.h>
#elif defined(__linux__)
#include <sys/random.h>
#endif

#if defined(__x86_64__) || defined(_M_X64) || defined(__i386__) || defined(_M_IX86)
#if defined(_MSC_VER)
#include <intrin.h>
#else
#include <immintrin.h>
#endif
#endif

#define NYRA_CHACHA_ROUNDS 20

typedef struct {
    uint32_t state[16];
    uint8_t block[64];
    size_t block_pos;
    int seeded;
} NyraRng;

static NyraRng nyra_rng;

static uint32_t rotl32(uint32_t x, unsigned n) {
    return (x << n) | (x >> (32u - n));
}

static void chacha_qr(uint32_t *a, uint32_t *b, uint32_t *c, uint32_t *d) {
    *a += *b;
    *d ^= *a;
    *d = rotl32(*d, 16);
    *c += *d;
    *b ^= *c;
    *b = rotl32(*b, 12);
    *a += *b;
    *d ^= *a;
    *d = rotl32(*d, 8);
    *c += *d;
    *b ^= *c;
    *b = rotl32(*b, 7);
}

static uint32_t u8to32_le(const uint8_t *p) {
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static void chacha_init_state(uint32_t state[16], const uint8_t key[32], const uint8_t nonce[12]) {
    static const uint8_t sigma[16] = {
        'e', 'x', 'p', 'a', 'n', 'd', ' ', '3', '2', '-', 'b', 'y', 't', 'e', ' ', 'k',
    };
    state[0] = u8to32_le(sigma + 0);
    state[1] = u8to32_le(sigma + 4);
    state[2] = u8to32_le(sigma + 8);
    state[3] = u8to32_le(sigma + 12);
    for (int i = 0; i < 8; i++) {
        state[4 + i] = u8to32_le(key + i * 4);
    }
    state[12] = 0;
    state[13] = u8to32_le(nonce + 0);
    state[14] = u8to32_le(nonce + 4);
    state[15] = u8to32_le(nonce + 8);
}

static void chacha_block(uint32_t state[16], uint8_t out[64]) {
    uint32_t x[16];
    memcpy(x, state, sizeof(x));
    for (int i = 0; i < NYRA_CHACHA_ROUNDS; i += 2) {
        chacha_qr(&x[0], &x[4], &x[8], &x[12]);
        chacha_qr(&x[1], &x[5], &x[9], &x[13]);
        chacha_qr(&x[2], &x[6], &x[10], &x[14]);
        chacha_qr(&x[3], &x[7], &x[11], &x[15]);
        chacha_qr(&x[0], &x[5], &x[10], &x[15]);
        chacha_qr(&x[1], &x[6], &x[11], &x[12]);
        chacha_qr(&x[2], &x[7], &x[8], &x[13]);
        chacha_qr(&x[3], &x[4], &x[9], &x[14]);
    }
    for (int i = 0; i < 16; i++) {
        x[i] += state[i];
        out[i * 4] = (uint8_t)(x[i] & 0xffu);
        out[i * 4 + 1] = (uint8_t)((x[i] >> 8) & 0xffu);
        out[i * 4 + 2] = (uint8_t)((x[i] >> 16) & 0xffu);
        out[i * 4 + 3] = (uint8_t)((x[i] >> 24) & 0xffu);
    }
    state[12]++;
}

static int nyra_rdrand64(uint64_t *out) {
#if defined(__x86_64__) || defined(_M_X64)
#if defined(_MSC_VER)
    return _rdrand64_step(out) ? 1 : 0;
#else
    unsigned char ok = 0;
    __asm__ volatile("rdrand %0; setc %1" : "=r"(*out), "=qm"(ok));
    return ok;
#endif
#elif defined(__i386__) || defined(_M_IX86)
    uint32_t lo = 0;
    uint32_t hi = 0;
#if defined(_MSC_VER)
    if (!_rdrand32_step(&lo) || !_rdrand32_step(&hi)) {
        return 0;
    }
#else
    unsigned char ok1 = 0;
    unsigned char ok2 = 0;
    __asm__ volatile("rdrand %0; setc %1" : "=r"(lo), "=qm"(ok1));
    __asm__ volatile("rdrand %0; setc %1" : "=r"(hi), "=qm"(ok2));
    if (!ok1 || !ok2) {
        return 0;
    }
#endif
    *out = ((uint64_t)hi << 32) | lo;
    return 1;
#else
    (void)out;
    return 0;
#endif
}

static int nyra_read_os_entropy(uint8_t *buf, size_t len) {
#if defined(__APPLE__)
    arc4random_buf(buf, len);
    return 0;
#elif defined(__linux__)
    if (getentropy(buf, len) == 0) {
        return 0;
    }
#endif
#if defined(_WIN32)
    if (BCryptGenRandom(NULL, (PUCHAR)buf, (ULONG)len, BCRYPT_USE_SYSTEM_PREFERRED_RNG) == 0) {
        return 0;
    }
#endif
#if !defined(_WIN32)
    int fd = open("/dev/urandom", O_RDONLY);
    if (fd >= 0) {
        size_t got = 0;
        while (got < len) {
            ssize_t n = read(fd, buf + got, len - got);
            if (n <= 0) {
                break;
            }
            got += (size_t)n;
        }
        close(fd);
        if (got == len) {
            return 0;
        }
    }
#endif
    return -1;
}

static void nyra_mix_entropy(uint8_t *buf, size_t len) {
    size_t off = 0;
    while (off + 8 <= len) {
        uint64_t hw = 0;
        if (nyra_rdrand64(&hw)) {
            memcpy(buf + off, &hw, 8);
            off += 8;
        } else {
            break;
        }
    }
}

static void nyra_rng_seed_once(void) {
    if (nyra_rng.seeded) {
        return;
    }
    uint8_t seed[32];
    memset(seed, 0, sizeof(seed));
    if (nyra_read_os_entropy(seed, sizeof(seed)) != 0) {
        uint64_t fallback = NYRA_GETPID() ^ (uint64_t)time(NULL);
        memcpy(seed, &fallback, sizeof(fallback));
    }
    nyra_mix_entropy(seed, sizeof(seed));

    uint8_t nonce[12];
    for (int i = 0; i < 12; i++) {
        nonce[i] = (uint8_t)(seed[i] ^ seed[i + 20]);
    }
    chacha_init_state(nyra_rng.state, seed, nonce);
    nyra_rng.block_pos = sizeof(nyra_rng.block);
    nyra_rng.seeded = 1;
}

static void nyra_rng_fill_block(void) {
    chacha_block(nyra_rng.state, nyra_rng.block);
    nyra_rng.block_pos = 0;
}

static void nyra_rand_bytes(uint8_t *out, size_t len) {
    nyra_rng_seed_once();
    size_t done = 0;
    while (done < len) {
        if (nyra_rng.block_pos >= sizeof(nyra_rng.block)) {
            nyra_rng_fill_block();
        }
        size_t take = sizeof(nyra_rng.block) - nyra_rng.block_pos;
        if (take > len - done) {
            take = len - done;
        }
        memcpy(out + done, nyra_rng.block + nyra_rng.block_pos, take);
        nyra_rng.block_pos += take;
        done += take;
    }
}

static uint32_t nyra_rand_u32(void) {
    uint32_t v = 0;
    nyra_rand_bytes((uint8_t *)&v, sizeof(v));
    return v;
}

static uint64_t nyra_rand_u64(void) {
    uint64_t lo = (uint64_t)nyra_rand_u32();
    uint64_t hi = (uint64_t)nyra_rand_u32();
    return (hi << 32) | lo;
}

static uint32_t nyra_uniform_u32_below(uint32_t span) {
    if (span == 0u) {
        return 0u;
    }
    uint32_t limit = 0xffffffffu - (0xffffffffu % span);
    uint32_t r;
    do {
        r = nyra_rand_u32();
    } while (r >= limit);
    return r % span;
}

static uint64_t nyra_uniform_u64_below(uint64_t span) {
    if (span == 0u) {
        return 0u;
    }
    uint64_t limit = 0xffffffffffffffffULL - (0xffffffffffffffffULL % span);
    uint64_t r;
    do {
        r = nyra_rand_u64();
    } while (r >= limit);
    return r % span;
}

int rand_i32(void) {
    return (int32_t)nyra_rand_u32();
}

int rand_range(int min_val, int max_val) {
    if (max_val <= min_val) {
        return min_val;
    }
    uint32_t span = (uint32_t)(max_val - min_val + 1);
    return min_val + (int)nyra_uniform_u32_below(span);
}

int64_t rand_i64(void) {
    return (int64_t)nyra_rand_u64();
}

int64_t rand_range_i64(int64_t min_val, int64_t max_val) {
    if (max_val <= min_val) {
        return min_val;
    }
    uint64_t span = (uint64_t)((uint64_t)max_val - (uint64_t)min_val + 1u);
    if (span <= (uint64_t)UINT32_MAX) {
        return min_val + (int64_t)nyra_uniform_u32_below((uint32_t)span);
    }
    return min_val + (int64_t)nyra_uniform_u64_below(span);
}

uint32_t rand_u32(void) {
    return nyra_rand_u32();
}

uint32_t rand_range_u32(uint32_t min_val, uint32_t max_val) {
    if (max_val <= min_val) {
        return min_val;
    }
    uint32_t span = max_val - min_val + 1u;
    return min_val + nyra_uniform_u32_below(span);
}

uint64_t rand_u64(void) {
    return nyra_rand_u64();
}

uint64_t rand_range_u64(uint64_t min_val, uint64_t max_val) {
    if (max_val <= min_val) {
        return min_val;
    }
    uint64_t span = max_val - min_val + 1u;
    if (span <= (uint64_t)UINT32_MAX) {
        return min_val + (uint64_t)nyra_uniform_u32_below((uint32_t)span);
    }
    return min_val + nyra_uniform_u64_below(span);
}

double rand_f64(void) {
    uint64_t r = ((uint64_t)nyra_rand_u32() << 32) | (uint64_t)nyra_rand_u32();
    return (double)(r >> 11) * (1.0 / 9007199254740992.0);
}

double rand_f64_range(double min_val, double max_val) {
    if (max_val <= min_val) {
        return min_val;
    }
    return min_val + rand_f64() * (max_val - min_val);
}

static const char nyra_hex_digits[] = "0123456789abcdef";

char *random_hex(int byte_count) {
    if (byte_count <= 0 || byte_count > 4096) {
        return NULL;
    }
    size_t out_len = (size_t)byte_count * 2;
    char *out = (char *)malloc(out_len + 1);
    if (!out) {
        return NULL;
    }
    uint8_t *raw = (uint8_t *)malloc((size_t)byte_count);
    if (!raw) {
        free(out);
        return NULL;
    }
    nyra_rand_bytes(raw, (size_t)byte_count);
    for (int i = 0; i < byte_count; i++) {
        out[i * 2] = nyra_hex_digits[raw[i] >> 4];
        out[i * 2 + 1] = nyra_hex_digits[raw[i] & 0x0f];
    }
    out[out_len] = '\0';
    free(raw);
    return out;
}
