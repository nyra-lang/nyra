// nyra test tests/nyra/json_version_encode_test.ny
import "stdlib/testing.ny"
import "stdlib/json/mod.ny"
import "stdlib/vec_str.ny"

test fn test_json_encode_semver_version_as_string() {
    let keys = Vec_str_new()
    let vals = Vec_str_new()
    Vec_str_push(keys, "version")
    Vec_str_push(vals, "0.1.0")
    let out = json_encode_object(keys, vals)
    assert_str_eq(out, "{\"version\":\"0.1.0\"}")
}

test fn test_vec_str_join_two_lines() {
    let lines = Vec_str_new()
    Vec_str_push(lines, "alpha")
    Vec_str_push(lines, "beta")
    let joined = Vec_str_join(lines, ",")
    assert_str_eq(joined, "alpha,beta")
}
