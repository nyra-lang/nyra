import "stdlib/vec.ny"

fn main() -> void {
    let v = vec().push(3).push(1).push(2).sort()
    print(v.get(0))
}
