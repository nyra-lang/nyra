// [contrib-dev:vec_i32_slice_methods:vec]
import "stdlib/vec.ny"

fn main() -> void {
    let v = vec().push(1).push(2).push(3).push(4)
    let w = v.window(1, 2)
    print(w.len())
}
// [/contrib-dev:vec_i32_slice_methods:vec]
