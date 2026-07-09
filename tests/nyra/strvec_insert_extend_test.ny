// [contrib-dev:strvec_insert_extend:vec_str]
import "stdlib/testing.ny"
import "stdlib/vec_str.ny"

test fn test_strvec_insert_extend() {
    let v = strs().push("b").insert(0, "a")
    assert_str_eq(v.get(0), "a")
    assert_str_eq(v.remove_at(1), "b")
}
// [/contrib-dev:strvec_insert_extend:vec_str]
