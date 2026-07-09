// [contrib-dev:vec_str_pop:vec_str]
import "stdlib/vec_str.ny"

fn main() -> void {
    let v = strs().push("a").push("b")
    print(v.pop())
}
// [/contrib-dev:vec_str_pop:vec_str]
