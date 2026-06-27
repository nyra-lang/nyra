import "stdlib/testing.ny"
import "stdlib/vec.ny"

test fn conf_vec_push_len() {
    let v = Vec_i32_new()
    Vec_i32_push(v, 1)
    Vec_i32_push(v, 2)
    assert_eq(Vec_i32_len(v), 2)
    assert_eq(Vec_i32_get(v, 1), 2)
    Vec_i32_free(v)
}
