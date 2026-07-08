import "stdlib/testing.ny"
import "stdlib/json/mod.ny"
import "stdlib/strings/builder.ny"
import "stdlib/vec.ny"

fn test_json_short_typed() -> void {
    let o: HashMap_str_str = jparse("{\"n\":3}")
    assert_eq(jnum(o, "n"), 3)
}

fn test_vec_typed() -> void {
    let v: VecI32 = vec().push(9)
    assert_eq(v.get(0), 9)
}

fn main() -> void {
    test_json_short_typed()
    test_vec_typed()
    print("stdlib sugar typed ok")
}
