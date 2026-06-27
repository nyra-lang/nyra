import "stdlib/testing.ny"

test fn conf_break_while_typed() {
    let mut n: i32 = 0
    while n < 10 {
        if n == 3 {
            break
        }
        n = n + 1
    }
    assert_eq(n, 3)
}
