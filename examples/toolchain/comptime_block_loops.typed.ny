// Typed variant — comptime block with while loop.

const TOTAL: i32 = comptime {
    let mut acc: i32 = 0
    let mut i: i32 = 0
    while i < 4 {
        acc = acc + i
        i = i + 1
    }
    acc
}

fn main() {
    print("TOTAL", TOTAL)
}
