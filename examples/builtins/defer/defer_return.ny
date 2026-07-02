// defer on return — runs cleanup before function exits.
allow_extended

fn cleanup() {
    print(1)
}

fn main() {
    defer cleanup()
    return
}
