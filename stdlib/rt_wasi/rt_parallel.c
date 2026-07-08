typedef int32_t (*NyraParBody)(int index, void *ctx);
typedef int32_t (*NyraParPred)(int index, void *ctx);

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

static int32_t parallel_search_sequential(int32_t start, int32_t end, NyraParPred pred, void *ctx,
                                          int32_t search_op) {
    if (end <= start || !pred) {
        if (search_op == 2) {
            return 1;
        }
        if (search_op == 1) {
            return -1;
        }
        return 0;
    }
    if (search_op == 0) {
        for (int32_t i = start; i < end; i++) {
            if (pred(i, ctx)) {
                return 1;
            }
        }
        return 0;
    }
    if (search_op == 1) {
        for (int32_t i = start; i < end; i++) {
            if (pred(i, ctx)) {
                return i;
            }
        }
        return -1;
    }
    for (int32_t i = start; i < end; i++) {
        if (!pred(i, ctx)) {
            return 0;
        }
    }
    return 1;
}

int32_t parallel_any_range(int32_t start, int32_t end, NyraParPred pred, void *ctx,
                           int32_t max_workers, int32_t exact_workers, int32_t mode,
                           int32_t cpu_percent, int32_t backend) {
    (void)max_workers;
    (void)exact_workers;
    (void)mode;
    (void)cpu_percent;
    (void)backend;
    return parallel_search_sequential(start, end, pred, ctx, 0);
}

int32_t parallel_find_range(int32_t start, int32_t end, NyraParPred pred, void *ctx,
                            int32_t max_workers, int32_t exact_workers, int32_t mode,
                            int32_t cpu_percent, int32_t backend) {
    (void)max_workers;
    (void)exact_workers;
    (void)mode;
    (void)cpu_percent;
    (void)backend;
    return parallel_search_sequential(start, end, pred, ctx, 1);
}

int32_t parallel_all_range(int32_t start, int32_t end, NyraParPred pred, void *ctx,
                           int32_t max_workers, int32_t exact_workers, int32_t mode,
                           int32_t cpu_percent, int32_t backend) {
    (void)max_workers;
    (void)exact_workers;
    (void)mode;
    (void)cpu_percent;
    (void)backend;
    return parallel_search_sequential(start, end, pred, ctx, 2);
}
