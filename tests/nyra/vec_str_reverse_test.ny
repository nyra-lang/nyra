// [contrib-dev:vec_str_reverse:vec_str]
import "stdlib/testing.ny"
import "stdlib/vec_str.ny"

test fn test_vec_str_reverse() {
    let v = strs().push("a").push("b").reverse()
    assert_str_eq(v.get(0), "b")
}
// [/contrib-dev:vec_str_reverse:vec_str]
