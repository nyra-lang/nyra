// Showcase: batch3–5 gap-fill builtins (zero-types style).
import "stdlib/math.ny"
import "stdlib/strconv/mod.ny"
import "stdlib/encoding/mod.ny"
import "stdlib/map.ny"

fn main() {
    // Strings (builtins — no import)
    print("abc".reverse())
    print("123".is_digit())
    print("a:b".before_sep(":"))
    print("  a   b  ".collapse_ws())
    print("hello".equal_fold("HELLO"))

    // Math
    print(saturating_add(2147483647, 1))
    print(gcd_i32(12, 8))
    print(format_bin(5))

    // Encoding / strconv
    print(hex_encode("Hi"))
    print(quote("say \"hi\""))

    // Vec / map method syntax
    let v = vec().push(1).push(2).reserve(8)
    print(v.capacity())
    let m = HashMap_i32_i32_new().insert(1, 42)
    print(m.get_or(1, 0))
    print(m.get_or(2, 99))
}
