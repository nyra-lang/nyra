// [contrib-dev:option_combinators:option_combinators]
import "stdlib/option/combinators.ny"

fn main() -> void {
    let o = Option_i32_some(5)
    print(Option_i32_unwrap_or(o, 0))
}
// [/contrib-dev:option_combinators:option_combinators]
