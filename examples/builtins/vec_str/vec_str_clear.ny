// [contrib-dev:vec_str_clear:vec_str]
import "stdlib/vec_str.ny"

fn main() {
    let v = strs().push("a").clear()
    print(v.len())
}
// [/contrib-dev:vec_str_clear:vec_str]
