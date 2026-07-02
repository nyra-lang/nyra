#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>

int main(void) {

    const int64_t n = 500000;
    const int64_t modv = 1000000007;
    int64_t acc = 0;
    for (int64_t i = 0; i < n; i++) {
        void *p = malloc(16);
        acc = (acc + i) % modv;
        free(p);
    }
    printf("%lld\n", (long long)acc);
    return 0;
}
