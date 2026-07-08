// nyra test tests/nyra/strcmp_borrow_test.ny
import "stdlib/testing.ny"
import "stdlib/strings.ny"

test fn test_strcmp_in_if_body_reuses_operand() {
    let a = "hello"
    let b = "world"
    if strcmp(a, b) == 0 {
        assert_str_eq(a, "hello")
    } else {
        assert_str_eq(a, "hello")
    }
}

test fn test_strcmp_twice_in_condition() {
    let a = "x"
    let b = "y"
    let c = "x"
    let ok = if strcmp(a, b) == 0 { 0 } else { 1 }
    let same = if strcmp(a, c) == 0 { 1 } else { 0 }
    assert_eq(same, 1)
    assert_eq(ok, 1)
}
