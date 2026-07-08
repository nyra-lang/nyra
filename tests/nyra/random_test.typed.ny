import "stdlib/random.ny"
import "stdlib/builtins_math.ny"

fn test_random_range() {
    let r = random(1, 6)
    if r < 1 || r > 6 {
        print("fail random range", r)
    }
}

fn test_random_i64() {
    let min: i64 = 50
    let max: i64 = 100
    let r: i64 = random(min, max)
    if r < 50 || r > 100 {
        print("fail random i64 range", r)
    }
}

fn test_random_f64() -> bool {
    let r = random_f64()
    if r < 0.0 || r >= 1.0 {
        print("fail random_f64 unit", r)
        return false
    }
    let band = random_f64(10.0, 20.0)
    if band < 10.0 || band >= 20.0 {
        print("fail random_f64 range", band)
        return false
    }
    let m = Math_random()
    if m < 0.0 || m >= 1.0 {
        print("fail Math_random", m)
        return false
    }
    return true
}

fn main() {
    test_random_range()
    test_random_i64()
    if test_random_f64() {
        print("random_test ok")
    }
}
