extern fn map_i32_i32_new() -> ptr
extern fn map_i32_i32_insert(m: ptr, key: i32, value: i32) -> void
extern fn map_i32_i32_get(m: ptr, key: i32) -> i32
extern fn map_i32_i32_contains(m: ptr, key: i32) -> i32
extern fn map_i32_i32_free(m: ptr) -> void

extern fn blackbox_i32(x: i32) -> i32

fn main() {
    let mut acc = 0
    let m = map_i32_i32_new()
    let mut i = 0
    while i < 200000 {
        let kk = i % 10000
        map_i32_i32_insert(m, kk, i)
        acc = (acc + map_i32_i32_get(m, kk)) % 1000000007
        i = i + 1
    }
    map_i32_i32_free(m)

    print(blackbox_i32(acc))
}
