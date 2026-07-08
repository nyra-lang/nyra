// `parallel for` — optional worker limits and scheduling modes.
extern fn blackbox_i32(x: i32) -> i32

fn main() {
    // 1. Auto (recommended): runtime picks workers from CPU count.
    parallel for i in 0..64 {
        blackbox_i32(i * i)
    }

    // 2. Cap workers (may use fewer when iteration count is small).
    parallel(max = 4) for i in 0..64 {
        blackbox_i32(i)
    }

    // 3. Exact worker count.
    parallel(threads = 4) for i in 0..64 {
        blackbox_i32(i)
    }

    // Fraction of logical CPUs (80% of 10 cores → 8 workers).
    parallel(cpu = 80%) for i in 0..32 {
        blackbox_i32(i)
    }

    // Leave one logical CPU for the OS / other apps.
    parallel(threads = cpu_count() - 1) for i in 0..32 {
        blackbox_i32(i)
    }

    // Scheduling modes: auto | balanced | max_performance | background
    parallel(mode = balanced) for i in 0..16 {
        blackbox_i32(i)
    }

    let nums = [10, 20, 30, 40]
    parallel(max = 2) for n in nums {
        blackbox_i32(n)
    }

    print(64)
}
