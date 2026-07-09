// [contrib-dev:strvec_set_method:vec_str]
import "stdlib/vec_str.ny"

fn main() {
    let v = strs().push("a")
    let _ = v.set(0, "z")
    print(v.get(0))
}
// [/contrib-dev:strvec_set_method:vec_str]
