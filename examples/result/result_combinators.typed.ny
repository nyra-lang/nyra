// [contrib-dev:result_combinators:result_combinators]
import "stdlib/result/combinators.ny"

fn main() -> void {
    let ok = Result_i32_i32_ok(7)
    print(Result_i32_i32_unwrap_or(ok, 0))
    print(Result_i32_i32_is_err(Result_i32_i32_err(1)))
}
// [/contrib-dev:result_combinators:result_combinators]
