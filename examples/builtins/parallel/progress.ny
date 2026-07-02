// progress for — sequential loop with built-in progress bar.
allow_extended

fn main() {
    progress(label = "demo") for i in 0..3 {
        print(i)
    }
}
