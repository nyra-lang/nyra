// Runtime baseline: build lookup table + sum rounds (all languages).
extern fn blackbox_i32(x: i32) -> i32

const TABLE_LEN = 64
const BUILD_ITERS = 8000
const SUM_ROUNDS = 8
const MOD = 1000000007

fn mix(n) {
    let a = n * 100003
    let b = n * n
    return (a + b * 31 + 997) % MOD
}

fn main() {
    mut table = [0; TABLE_LEN]
    mut i = 0
    while i < TABLE_LEN {
        mut k = 0
        mut v = 0
        while k < BUILD_ITERS {
            v = (v + mix(i + k)) % MOD
            k = k + 1
        }
        table[i] = v
        i = i + 1
    }
    mut acc = 0
    mut r = 0
    while r < SUM_ROUNDS {
        mut j = 0
        while j < TABLE_LEN {
            acc = (acc + table[j]) % MOD
            j = j + 1
        }
        r = r + 1
    }
    print(blackbox_i32(acc))
}
