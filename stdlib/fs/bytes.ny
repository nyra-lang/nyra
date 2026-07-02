// Binary I/O — `bytes` is a first-class type (opaque handle), distinct from `string`.

extern fn bytes_read_file(path: string) -> bytes
extern fn bytes_len(handle: bytes) -> i64
extern fn byte_at(handle: bytes, index: i64) -> i32
extern fn bytes_write_file(path: string, handle: bytes) -> i32
extern fn bytes_from_string(s: string) -> bytes
extern fn bytes_to_string(handle: bytes) -> string
extern fn bytes_free(handle: bytes) -> void
extern fn stdin_read_bytes(max_bytes: i32) -> bytes
extern fn stdout_write_bytes(handle: bytes) -> void

fn Bytes_read(path: string) -> bytes {
    return bytes_read_file(path)
}

fn Bytes_len(data: bytes) -> i64 {
    return bytes_len(data)
}

fn Bytes_to_string(data: bytes) -> string {
    return bytes_to_string(data)
}

fn Bytes_free(data: bytes) -> void {
    bytes_free(data)
}

// Legacy struct alias — prefer `bytes` type directly.
struct Bytes {
    handle: ptr
}

fn Bytes_from_handle(handle: ptr) -> bytes {
    return bytes_from_string("")
}
