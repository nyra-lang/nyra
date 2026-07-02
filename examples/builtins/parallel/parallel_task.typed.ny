// parallel for — default task-pool backend (same workers as spawn).
allow_extended

fn main() -> void {
    parallel for i in 0..4 {
        print(i)
    }
    print(999)
}
