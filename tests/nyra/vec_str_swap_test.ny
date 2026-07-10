// [contrib-dev:vec_str_swap:vec_str]
import "stdlib/testing.ny"
import "stdlib/vec_str.ny"

test fn test_vec_str_swap() {
    let v = strs().push("a").push("b").swap(0, 1)
    assert_str_eq(v.get(0), "b")
}
// [/contrib-dev:vec_str_swap:vec_str]
