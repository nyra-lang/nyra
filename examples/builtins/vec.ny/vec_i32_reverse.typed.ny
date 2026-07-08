import "stdlib/vec.ny"

fn main() -> void {
    let v = vec().push(1).push(2).push(3).reverse()
    print(v.get(0))
}
