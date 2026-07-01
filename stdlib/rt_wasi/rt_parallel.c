typedef void (*NyraParBody)(int index, void *ctx);

int cpu_count(void) {
    return 1;
}

void parallel_for_range(int start, int end, NyraParBody body, void *ctx, int max_workers,
                        int exact_workers, int mode, int cpu_percent, int backend) {
    (void)max_workers;
    (void)exact_workers;
    (void)mode;
    (void)cpu_percent;
    (void)backend;
    if (!body || end <= start) {
        return;
    }
    for (int i = start; i < end; i++) {
        body(i, ctx);
    }
}
