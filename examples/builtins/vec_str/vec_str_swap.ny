// [contrib-dev:vec_str_swap:vec_str]
import "stdlib/vec_str.ny"

fn main() {
    let v = strs().push("a").push("b").swap(0, 1)
    print(v.get(0))
}
// [/contrib-dev:vec_str_swap:vec_str]
