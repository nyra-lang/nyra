extern fn blackbox_i32(x: i32) -> i32

test fn test_parallel_for_range() {
    parallel for i in 0..16 {
        blackbox_i32(i)
    }
}

test fn test_parallel_for_array() {
    let nums = [1, 2, 3, 4]
    parallel for n in nums {
        blackbox_i32(n)
    }
}

test fn test_parallel_max_workers() {
    parallel(max = 2) for i in 0..8 {
        blackbox_i32(i)
    }
}

test fn test_parallel_exact_threads() {
    parallel(threads = 2) for i in 0..8 {
        blackbox_i32(i)
    }
}

test fn test_parallel_cpu_percent() {
    parallel(cpu = 50%) for i in 0..8 {
        blackbox_i32(i)
    }
}

test fn test_parallel_mode_balanced() {
    parallel(mode = balanced) for i in 0..8 {
        blackbox_i32(i)
    }
}

test fn test_parallel_threads_expr() {
    parallel(threads = cpu_count() - 1) for i in 0..8 {
        blackbox_i32(i)
    }
}

test fn test_parallel_task_default() {
    parallel(max = 2) for i in 0..8 {
        blackbox_i32(i)
    }
}

test fn test_parallel_thread_backend() {
    parallel:thread(max = 2) for i in 0..8 {
        blackbox_i32(i)
    }
}

test fn test_parallel_backend_option() {
    parallel(backend = thread, threads = 2) for i in 0..8 {
        blackbox_i32(i)
    }
}
