#include <stdio.h>
#include <stdint.h>

int main(void) {

    const int64_t n = 500000;
    const int64_t modv = 1000000007;
    int64_t acc = 0;
    for (int64_t i = 0; i < n; i++) {
        int64_t a = i % 1000;
        int64_t b = (i * 7) % 1000;
        acc = (acc + a + b) % modv;
    }
    printf("%lld\n", (long long)acc);
    return 0;
}
