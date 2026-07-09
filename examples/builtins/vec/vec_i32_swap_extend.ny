// [contrib-dev:vec_i32_swap_extend:vec]
import "stdlib/vec.ny"

fn main() {
    let v = vec().push(1).push(2).swap(0, 1)
    print(v.get(0))
}
// [/contrib-dev:vec_i32_swap_extend:vec]
