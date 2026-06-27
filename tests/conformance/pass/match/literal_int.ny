import "stdlib/testing.ny"

test fn conf_match_int_via_if() {
    let n = 2
    let out = if n == 2 { 20 } else { 0 }
    assert_eq(out, 20)
}

test fn conf_match_int_other_arm() {
    let n = 1
    let out = if n == 2 { 20 } else { 10 }
    assert_eq(out, 10)
}
