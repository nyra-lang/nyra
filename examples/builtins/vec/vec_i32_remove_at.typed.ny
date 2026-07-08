import "stdlib/vec.ny"

fn main() -> void {
    let v = vec().push(10).push(20).remove(0)
    print(v.len())
}
