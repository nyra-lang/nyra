void *channel_new(void) {
    return NULL;
}

void channel_send(void *ch, int value) {
    (void)ch;
    (void)value;
}

int channel_recv(void *ch) {
    (void)ch;
    return 0;
}

int channel_try_recv(void *ch) {
    (void)ch;
    return 0;
}

int channel_try_value(void) {
    return 0;
}

int channel_recv_async(void *ch) {
    (void)ch;
    return -1;
}

void channel_free(void *ch) {
    (void)ch;
}
