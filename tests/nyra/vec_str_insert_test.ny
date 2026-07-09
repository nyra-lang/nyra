// [contrib-dev:vec_str_insert:vec_str]
import "stdlib/testing.ny"
import "stdlib/vec_str.ny"

test fn test_vec_str_insert() {
    let v = strs().push("b").insert(0, "a")
    assert_str_eq(v.get(0), "a")
}
// [/contrib-dev:vec_str_insert:vec_str]
