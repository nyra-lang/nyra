fn pick(n) {
    return match n {
        0 => {
            let x = 5
            x + 5
        }
        _ => {
            let y = 1
            y
        }
    }
}

fn main() {
    print(pick(0))
    print(pick(9))
}
