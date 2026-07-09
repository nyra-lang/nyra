// [contrib-dev:strvec_methods:vec_str]
import "stdlib/testing.ny"
import "stdlib/vec_str.ny"

test fn test_strvec_methods() {
    let v = strs().push("x")
    assert_eq(v.is_empty(), 0)
    assert_str_eq(v.pop(), "x")
}
// [/contrib-dev:strvec_methods:vec_str]
