// [contrib-dev:vec_str_pop:vec_str]
import "stdlib/testing.ny"
import "stdlib/vec_str.ny"

test fn test_vec_str_pop() {
    let v = strs().push("a").push("b")
    assert_str_eq(v.pop(), "b")
    assert_eq(v.len(), 1)
}
// [/contrib-dev:vec_str_pop:vec_str]
