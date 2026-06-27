enum Op150 {
    Add
    Sub
    Mul
}
fn main() {
    let op = Op150.Add
    let n = match op {
        Op150.Add => 150
        Op150.Sub => 151
        Op150.Mul => 152
    }
    print(n)
}
