#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

int main(void) {

    const int modv = 1000000007;
    int acc = 0;
    size_t len = 1;
    char *s = (char *)malloc(2);
    s[0]='a'; s[1]='\0';
    for (int i = 0; i < 100000; i++) {
        size_t nlen = len + 1;
        char *ns = (char *)malloc(nlen + 1);
        memcpy(ns, s, len);
        ns[len]='x'; ns[nlen]='\0';
        free(s);
        s = ns;
        len = nlen;
        acc = (acc + (int)len) % modv;
    }
    free(s);
    printf("%d\n", acc);
    return 0;
}
