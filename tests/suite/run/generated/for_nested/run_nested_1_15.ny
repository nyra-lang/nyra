// run-stdout: 15
fn main() {
    let mut count = 0
    for i in 0..1 {
        for j in 0..15 {
            count = count + 1
        }
    }
    print(count)
}
