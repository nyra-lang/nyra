// [contrib-dev:vec_str_extend:vec_str]
import "stdlib/testing.ny"
import "stdlib/vec_str.ny"

test fn test_vec_str_extend() {
    let a = strs().push("x")
    let b = strs().push("y").push("z")
    vec_str_extend(a.handle, b.handle)
    assert_eq(a.len(), 3)
}
// [/contrib-dev:vec_str_extend:vec_str]
