// [contrib-dev:vec_str_set]
import "stdlib/vec_str.ny"

fn main() {
    let v = strs().push("a")
    vec_str_set(v.handle, 0, "z")
    print(v.get(0))
}
// [/contrib-dev:vec_str_set]

