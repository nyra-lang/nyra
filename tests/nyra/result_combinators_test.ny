// [contrib-dev:result_combinators:result_combinators]
import "stdlib/testing.ny"
import "stdlib/result.ny"
import "stdlib/result/combinators.ny"

test fn test_result_combinators() {
    let ok = Result_i32_i32_ok(7)
    assert_eq(Result_i32_i32_unwrap_or(ok, 0), 7)
    assert_eq(Result_i32_i32_is_err(Result_i32_i32_err(1)), 1)
}
// [/contrib-dev:result_combinators:result_combinators]
