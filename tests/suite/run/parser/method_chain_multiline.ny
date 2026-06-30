// run-stdout: 2
fn main() {
    let mut log = StrVec_new()
    log = log
        .push("a")
    log = log
        .push("b")
    print(log.len())
}
