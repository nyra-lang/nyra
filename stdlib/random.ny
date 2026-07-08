import "vec.ny"

// ChaCha20 CSPRNG — `random()` / `random(min, max)` and `random_f64()` / `random_f64(min, max)`
// are compiler builtins (no import required). Import this module for `shuffle_pick`.

extern fn rand_range(min_val: i32, max_val: i32) -> i32

fn shuffle_pick(v: ptr) -> i32 {
    let n = vec_len(v)
    if n <= 0 {
        return 0
    }
    let idx = rand_range(0, n - 1)
    return vec_get(v, idx)
}
