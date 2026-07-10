// [contrib-dev:vec_str_insert:vec_str]
import "stdlib/vec_str.ny"

fn main() -> void {
    let v = strs().push("b").insert(0, "a")
    print(v.get(0))
}
// [/contrib-dev:vec_str_insert:vec_str]
