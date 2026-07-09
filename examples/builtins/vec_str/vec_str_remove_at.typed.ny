// [contrib-dev:vec_str_remove_at:vec_str]
import "stdlib/vec_str.ny"

fn main() -> void {
    let v = strs().push("a").push("b")
    print(v.remove_at(0))
}
// [/contrib-dev:vec_str_remove_at:vec_str]
