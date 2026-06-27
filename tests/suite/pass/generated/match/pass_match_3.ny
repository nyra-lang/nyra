enum Op3 {
    Add
    Sub
    Mul
}
fn main() {
    let op = Op3.Add
    let n = match op {
        Op3.Add => 3
        Op3.Sub => 4
        Op3.Mul => 5
    }
    print(n)
}
