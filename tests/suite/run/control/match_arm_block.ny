// run-stdout: 10
fn pick(n) {
    return match n {
        0 => {
            let x = 5
            x + 5
        }
        _ => 0
    }
}

fn main() {
    print(pick(0))
}
