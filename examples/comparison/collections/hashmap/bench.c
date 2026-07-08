#include <stdio.h>
#include <stdint.h>

int main(void) {

    const int modv = 1000000007;
    const int n = 200000;
    int acc = 0;
    enum { CAP = 32768 };
    int keys[CAP];
    int vals[CAP];
    unsigned char used[CAP];
    for (int i = 0; i < CAP; i++) { keys[i]=0; vals[i]=0; used[i]=0; }
    for (int i = 0; i < n; i++) {
        int k = i % 10000;
        unsigned h = (unsigned)k * 2654435761u % CAP;
        int inserted = 0;
        while (used[h]) {
            if (keys[h] == k) { vals[h] = i; inserted = 1; break; }
            h = (h + 1) % CAP;
        }
        if (!inserted) { used[h]=1; keys[h]=k; vals[h]=i; }
        h = (unsigned)k * 2654435761u % CAP;
        for (int step = 0; step < CAP; step++) {
            unsigned idx = (h + (unsigned)step) % CAP;
            if (!used[idx]) break;
            if (keys[idx] == k) { acc = (acc + vals[idx]) % modv; break; }
        }
    }
    printf("%d\n", acc);
    return 0;
}
