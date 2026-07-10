// [contrib-dev:strvec_methods:vec_str]
import "stdlib/vec_str.ny"

fn main() {
    let v = strs().push("x")
    print(v.is_empty())
    print(v.pop())
}
// [/contrib-dev:strvec_methods:vec_str]
