#include <stdio.h>
#include <stdint.h>

int main(void) {

    const int64_t n = 500000;
    const int64_t modv = 1000000007;
    int64_t acc = 0;
    int64_t bump = 0;
    for (int64_t i = 0; i < n; i++) {
        bump = (bump + 16) % 67108864;
        acc = (acc + bump + i) % modv;
    }
    printf("%lld\n", (long long)acc);
    return 0;
}
