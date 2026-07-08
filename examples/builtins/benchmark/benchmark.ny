// benchmark { } — wall time, RSS delta, and CPU% for a block.
allow_extended

extern fn blackbox_i32(x: i32) -> i32

fn run() {
    let mut acc = 0
    for i in 0..10000 {
        acc = blackbox_i32(acc + i)
    }
    blackbox_i32(acc)
}

fn main() {
    benchmark {
        run()
    }
}
