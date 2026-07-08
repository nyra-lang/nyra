import "stdlib/vec.ny"

fn main() -> void {
    let v = vec().push(1).push(3).insert(1, 2)
    print(v.get(1))
}
