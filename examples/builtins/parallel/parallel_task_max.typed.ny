// parallel:task(max = N) — cap task-pool workers (alias of bare parallel).
allow_extended

fn main() -> void {
    parallel:task(max = 2) for i in 0..4 {
        print(i)
    }
    print(999)
}
