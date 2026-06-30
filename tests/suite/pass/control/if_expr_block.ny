fn pick(n) {
    print(if n > 0 {
        let doubled = n * 2
        doubled
    } else {
        0
    })
}

fn main() {
    pick(5)
    pick(-1)
}
