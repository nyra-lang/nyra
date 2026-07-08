import "stdlib/vec_str.ny"

fn main() -> void {
    let mut v = StrVec_new()
    v = v.push("alpha")
    v = v.push("beta")
    print(v.len())
    print(v.get(0))
}
