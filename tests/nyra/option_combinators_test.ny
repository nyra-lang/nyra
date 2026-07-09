// [contrib-dev:option_combinators:result]
import "stdlib/testing.ny"
import "stdlib/result.ny"

test fn test_option_combinators() {
    let some = Option_i32_some(5)
    assert_eq(Option_i32_unwrap_or(some, 0), 5)
    assert_eq(Option_i32_unwrap_or(Option_i32_none(), 9), 9)
}
// [/contrib-dev:option_combinators:result]
