// Batch6: FS metadata, slice utilities, bit math (zero-types).
import "stdlib/math.ny"
import "stdlib/fs/mod.ny"

fn main() {
    print("abc".escape_json())
    print(trailing_zeros(8))
    print(path_is_file("Cargo.toml"))
    let v = vec().push(1).push(2).push(3).push(4)
    let w = v.window(1, 2)
    print(w.len())
    print(w.get(0))
}
