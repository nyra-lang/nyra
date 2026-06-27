import "stdlib/vec.ny"

fn main() {
    let mut v = Vec_i32_new()
    let r = &v
    Vec_i32_push(v, 1)
    let _ = r
}
