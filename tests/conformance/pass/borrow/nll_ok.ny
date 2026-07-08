import "stdlib/testing.ny"

test fn conf_borrow_001_nll_string_reassign_after_borrow() {
    let mut s = "hi"
    let r = &s
    if strlen(r) != 2 {
        assert_eq(1, 0)
    }
    s = "bye"
    assert_eq(strlen(s), 3)
}

test fn conf_borrow_002_i32_copy_after_bind() {
    let a = 7
    let b = a
    assert_eq(a, 7)
    assert_eq(b, 7)
}

test fn conf_borrow_003_copy_type_both_alive() {
    let a = 11
    let b = a
    assert_eq(a + b, 22)
}
