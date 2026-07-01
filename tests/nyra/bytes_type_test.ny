import "stdlib/fs/bytes.ny"

test fn test_bytes_from_string() {
    let data: bytes = bytes_from_string("abc")
    assert_eq(data.len(), 3)
    assert_eq(data[0], 97)
    let s = data.to_string()
    assert_eq(s.len(), 3)
}

test fn test_bytes_string_no_mix() {
    let s: string = "hi"
    let _ = s
    let b: bytes = bytes_from_string("hi")
    let _ = b
}

fn main() {
    print(0)
}
