// parallel(cpu = P%) — use P percent of logical CPUs as worker count.
allow_extended

fn main() {
    parallel(cpu = 50%) for i in 0..4 {
        print(i)
    }
    print(999)
}
