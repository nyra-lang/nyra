// parallel:thread(max = N) — fork-join OS threads per chunk.
allow_extended

fn main() {
    parallel:thread(max = 2) for i in 0..4 {
        print(i)
    }
    print(999)
}
