// nyra test tests/nyra/borrow_deref_test.ny

test fn test_mut_ref_deref_print() {
    let mut count = 0
    let r = &mut count
    let val = *r
    assert_eq(val, 0)
    count = count + 1
    assert_eq(count, 1)
}

test fn test_immut_ref_deref_read() {
    let v = 10
    let r = &v
    let val = *r
    assert_eq(val, 10)
}

test fn test_nll_borrow_after_last_use() {
    let mut v = 1
    let r = &v
    print(*r)
    v = 2
    assert_eq(v, 2)
}
