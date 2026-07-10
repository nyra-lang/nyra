// [contrib-dev:strvec_insert_extend:vec_str]
import "stdlib/vec_str.ny"

fn main() {
    let v = strs().push("b").insert(0, "a")
    print(v.get(0))
    print(v.remove_at(1))
}
// [/contrib-dev:strvec_insert_extend:vec_str]
