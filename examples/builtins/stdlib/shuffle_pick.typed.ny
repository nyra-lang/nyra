import "stdlib/random.ny"
import "stdlib/vec.ny"

fn main() -> void {
    let v: ptr = Vec_i32_new()
    Vec_i32_push(v, 10)
    Vec_i32_push(v, 20)
    Vec_i32_push(v, 30)
    print(shuffle_pick(v))
}
