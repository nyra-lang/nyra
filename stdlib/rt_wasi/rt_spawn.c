void *spawn_capture(void (*body)(void *), void *data, long long nbytes) {
    (void)body;
    (void)data;
    (void)nbytes;
    return 0;
}

int spawn_join(void *handle) {
    (void)handle;
    return -1;
}

void spawn_handle_drop(void *handle) {
    (void)handle;
}
