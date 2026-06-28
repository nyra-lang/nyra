// Compile-time block + while / break / continue in comptime evaluation.

const TOTAL = comptime {
    let mut acc = 0
    let mut i = 0
    while i < 4 {
        acc = acc + i
        i = i + 1
    }
    acc
}

fn main() {
    print("TOTAL", TOTAL)
}
