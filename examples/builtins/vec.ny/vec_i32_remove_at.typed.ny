import "stdlib/vec.ny"

fn main() -> void {
    let v = vec().push(10).push(20)
    print(v.remove_at(0))
}
