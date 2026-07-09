// [contrib-dev:strvec_set_method:vec_str]
import "stdlib/testing.ny"
import "stdlib/vec_str.ny"

test fn test_strvec_set_method() {
    let v = strs().push("a")
    let _ = v.set(0, "z")
    assert_str_eq(v.get(0), "z")
}
// [/contrib-dev:strvec_set_method:vec_str]
