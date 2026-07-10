// [contrib-dev:vec_str_set:vec_str]
import "stdlib/testing.ny"
import "stdlib/vec_str.ny"

test fn test_vec_str_set() {
    let v = strs().push("a")
    vec_str_set(v.handle, 0, "z")
    assert_str_eq(v.get(0), "z")
}
// [/contrib-dev:vec_str_set:vec_str]
