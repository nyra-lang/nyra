// Runtime smoke for core stdlib modules (zero-types). See make/lib/stdlib-runtime-smoke.sh.
import "stdlib/builtins_string.ny"
import "stdlib/math.ny"
import "stdlib/map.ny"
import "stdlib/vec.ny"
import "stdlib/json/mod.ny"
import "stdlib/crypto/sha256.ny"
import "stdlib/time/date.ny"
import "stdlib/encoding/base64.ny"

fn main() {
    if strcmp(trim("  hi  "), "hi") != 0 {
        return
    }
    print(max_i32(3, 7))

    let mut map = HashMap_str_i32_new()
    map = map.insert("one", 1)
    map = map.insert("two", 2)
    print(map.get("two"))

    let nums = Vec_i32_new()
    Vec_i32_push(nums, 10)
    Vec_i32_push(nums, 20)
    print(Vec_i32_len(nums))
    Vec_i32_free(nums)

    let json = decode_string("{\"name\":\"nyra\",\"n\":42}", "name")
    print(json)
    print(decode_i32("{\"name\":\"nyra\",\"n\":42}", "n"))

    let digest = sha256("abc")
    print(strlen(digest))

    print(base64_encode("hi"))

    let today = date_format(Date_new(2026, 6, 27))
    print(strlen(today))

    print("stdlib-runtime ok")
}
