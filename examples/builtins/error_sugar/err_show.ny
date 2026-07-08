fn main() {
    let e = err_io("boom").context("open")
    print(e.kind)
    print(e.message)
}
