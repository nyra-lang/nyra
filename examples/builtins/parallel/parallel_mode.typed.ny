// parallel(mode = balanced) — leave one CPU for responsiveness.
allow_extended

fn main() -> void {
    parallel(mode = balanced) for i in 0..4 {
        print(i)
    }
    print(999)
}
