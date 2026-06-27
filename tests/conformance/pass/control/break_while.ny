import "stdlib/testing.ny"

test fn conf_break_while() {
    let mut n = 0
    while n < 10 {
        if n == 3 {
            break
        }
        n = n + 1
    }
    assert_eq(n, 3)
}
