// parallel(threads = N) — exactly N workers (backend still task pool by default).
allow_extended

fn main() {
    parallel(threads = 2) for i in 0..4 {
        print(i)
    }
    print(999)
}
