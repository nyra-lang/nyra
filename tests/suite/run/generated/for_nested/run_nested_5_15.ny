// run-stdout: 75
fn main() {
    let mut count = 0
    for i in 0..5 {
        for j in 0..15 {
            count = count + 1
        }
    }
    print(count)
}
