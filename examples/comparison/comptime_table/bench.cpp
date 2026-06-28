#include <cstdio>
#include <cstdint>

namespace {
constexpr int TABLE_LEN = 64;
constexpr int BUILD_ITERS = 8000;
constexpr int SUM_ROUNDS = 8;
constexpr int64_t MOD = 1000000007LL;

int64_t mix(int64_t n) {
    int64_t a = n * 100003LL;
    int64_t b = n * n;
    return (a + b * 31 + 997) % MOD;
}
}  // namespace

int main() {
    int64_t table[TABLE_LEN];
    for (int i = 0; i < TABLE_LEN; ++i) {
        int64_t v = 0;
        for (int k = 0; k < BUILD_ITERS; ++k) {
            v = (v + mix(i + k)) % MOD;
        }
        table[i] = v;
    }
    int64_t acc = 0;
    for (int r = 0; r < SUM_ROUNDS; ++r) {
        for (int j = 0; j < TABLE_LEN; ++j) {
            acc = (acc + table[j]) % MOD;
        }
    }
    std::printf("%lld\n", static_cast<long long>(acc));
    return 0;
}
