// [contrib-dev:vec_str_reverse:vec_str]
import "stdlib/vec_str.ny"

fn main() -> void {
    let v = strs().push("a").push("b").reverse()
    print(v.get(0))
}
// [/contrib-dev:vec_str_reverse:vec_str]
