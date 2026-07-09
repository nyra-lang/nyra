// [contrib-dev:vec_str_extend:vec_str]
import "stdlib/vec_str.ny"

fn main() {
    let a = strs().push("x")
    let b = strs().push("y")
    vec_str_extend(a.handle, b.handle)
    print(a.len())
}
// [/contrib-dev:vec_str_extend:vec_str]
