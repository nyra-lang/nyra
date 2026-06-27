// run-stdout: 10
fn main() {
    let mut count = 0
    for i in 0..2 {
        for j in 0..5 {
            count = count + 1
        }
    }
    print(count)
}
