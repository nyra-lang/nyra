// Growable vector of POD (Copy) structs via vec_bytes_* runtime.
extern fn vec_bytes_new(elem_size: i32) -> ptr
extern fn vec_bytes_push(v: ptr, elem: ptr) -> void
extern fn vec_bytes_get(v: ptr, index: i32, out: ptr) -> void
extern fn vec_bytes_len(v: ptr) -> i32
extern fn vec_bytes_free(v: ptr) -> void
extern fn vec_bytes_push_ptr(v: ptr, elem: ptr) -> void
extern fn vec_bytes_get_ptr(v: ptr, index: i32) -> ptr

pub struct Vec<T> {
    handle: ptr
}
