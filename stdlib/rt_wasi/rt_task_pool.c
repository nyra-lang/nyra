#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef void (*NyraSpawnBody)(void *);

void *spawn_task_capture(NyraSpawnBody body, void *data, int64_t nbytes) {
    (void)nbytes;
    if (body) {
        body(data);
    }
    return (void *)1;
}

int spawn_task_join(void *handle) {
    (void)handle;
    return 0;
}

void spawn_task_handle_drop(void *handle) {
    (void)handle;
}
