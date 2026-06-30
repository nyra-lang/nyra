
extern fn random_hex(byte_count: i32) -> string
extern fn strcat(a: &string, b: &string) -> string
extern fn substring(s: &string, start: i32, len: i32) -> string

// UUID_v4 — random RFC-4122-style string (MVP formatting; use NyraPkg for strict compliance).
fn UUID_v4() -> string {
    let hex = random_hex(16)
    let a = substring(clone hex, 0, 8)
    let b = substring(clone hex, 8, 4)
    let c = substring(clone hex, 12, 4)
    let d = substring(clone hex, 16, 4)
    let e = substring(clone hex, 20, 12)
    let p1 = strcat(strcat(a, "-"), b)
    let p2 = strcat(strcat(p1, "-"), c)
    let p3 = strcat(strcat(p2, "-"), d)
    return strcat(strcat(p3, "-"), e)
}
