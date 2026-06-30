// nyra test tests/nyra/strcat_reassign_test.ny
import "stdlib/testing.ny"
import "stdlib/vec_str.ny"

test fn test_vec_str_join_without_clone() {
    let lines = Vec_str_new()
    Vec_str_push(lines, "alpha")
    Vec_str_push(lines, "beta")
    let joined = Vec_str_join(lines, ",")
    assert_str_eq(joined, "alpha,beta")
}

test fn test_nested_strcat_reassign() {
    let mut out = "a"
    out = strcat(strcat(out, ","), "b")
    assert_str_eq(out, "a,b")
}
