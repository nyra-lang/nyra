import "stdlib/vec.ny"

fn main() {
    let v = Vec_i32_new()
    Vec_i32_push(v, 10)
    Vec_i32_push(v, 20)
    print(Vec_i32_len(v))
    print(Vec_i32_get(v, 0))
    print(Vec_i32_pop(v))
}
